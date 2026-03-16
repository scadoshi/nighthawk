use crate::log::header::HeaderWriter;

use super::{entry::Entry, header::HeaderReader};
use std::{
    collections::{BTreeMap, btree_map::Values},
    fs::{File, OpenOptions},
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

/// In-memory sorted map of the most recent entry per key. Tracks byte size for flush triggering.
#[derive(Debug)]
pub struct MemTable {
    inner: BTreeMap<String, Entry>,
    size: u64,
}

pub const FLUSH_THRESHOLD_MB: u64 = 4;

impl MemTable {
    pub fn new() -> Self {
        let inner = BTreeMap::<String, Entry>::new();
        let size = 0;
        Self { inner, size }
    }

    /// Rebuilds a `MemTable` by replaying all entries from the beginning of `file`.
    pub fn from_file(file: &mut File) -> anyhow::Result<Self> {
        let mut memtable = Self::new();
        file.seek(SeekFrom::Start(0))?;
        while let Ok(Some(entry)) = file.read_next_entry_with_header() {
            memtable.process(&entry)?;
        }
        Ok(memtable)
    }

    /// Total estimated byte size of all entries, used to trigger SSTable flushes.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Applies an entry: inserts for `Set`, removes for `Delete`. Updates byte size tracking.
    /// Returns the previous entry for the key, if any.
    pub fn process(&mut self, entry: impl Into<Entry>) -> anyhow::Result<Option<Entry>> {
        match entry.into() {
            set @ Entry::Set { .. } => {
                if let Some(previous) = self.inner.get(set.key()) {
                    self.size -=
                        previous.key().len() as u64 + wincode::serialize(&previous)?.len() as u64;
                }
                self.size += set.key().len() as u64 + wincode::serialize(&set)?.len() as u64;
                Ok(self.inner.insert(set.key().to_owned(), set))
            }
            delete @ Entry::Delete { .. } => {
                if let Some(set) = self.inner.get(delete.key()) {
                    self.size -= set.key().len() as u64 + wincode::serialize(&set)?.len() as u64;
                }
                Ok(self.inner.remove(delete.key()))
            }
        }
    }

    /// Returns `true` if the memtable has exceeded the 4 MB flush threshold.
    pub fn should_flush(&self) -> bool {
        self.size() > FLUSH_THRESHOLD_MB * (1 << 20)
    }

    /// Writes all entries in sorted key order to a new timestamped SSTable file in `path`, then clears the memtable.
    pub fn flush_to(&mut self, mut path: PathBuf) -> anyhow::Result<()> {
        // Insurance
        if self.is_empty() {
            return Ok(());
        }
        // Ensure file exists
        std::fs::create_dir_all(&path)?;
        // Generate timestamp file name
        let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros();
        path.push(format!("{:020}.sst", ts));
        let mut file = OpenOptions::new()
            .truncate(true)
            .create(true)
            .write(true)
            .open(path)?;
        // 10 bits per key is ideal for k=7
        let bit_count: u32 = self.len() as u32 * 10;
        let byte_count: u32 = bit_count.div_ceil(8);
        let mut bloomfilter = vec![0; byte_count as usize];
        for entry in self.values() {
            file.write_entry_with_header(entry)?;
            // Generate hashes
            let hash1 = xxh3::hash64_with_seed(entry.key().as_bytes(), 0);
            let hash2 = xxh3::hash64_with_seed(entry.key().as_bytes(), 1);
            // Update bloomfilter with k=7
            for i in 0..7 {
                let pos = ((hash1.wrapping_add(i).wrapping_mul(hash2)) % bit_count as u64) as usize;
                bloomfilter[pos / 8] |= 1 << (pos % 8);
            }
        }
        // Write bloomfilter and bit_count
        file.write_all(bloomfilter.as_slice())?;
        file.write_all(&bit_count.to_le_bytes())?;
        // Trigger fsync
        file.sync_all()?;
        self.clear();
        // Return
        Ok(())
    }

    /// Removes all entries and resets byte size to zero.
    pub fn clear(&mut self) {
        self.size = 0;
        self.inner.clear()
    }

    /// Number of unique keys currently in the memtable.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the memtable holds no entries and has zero tracked size.
    pub fn is_empty(&self) -> bool {
        self.size == 0 && self.inner.is_empty()
    }

    /// Returns `true` if `key` is present in the memtable.
    pub fn contains_key(&self, key: impl AsRef<str>) -> bool {
        self.inner.contains_key(key.as_ref())
    }

    /// Returns a reference to the entry for `key`, or `None` if absent.
    pub fn get(&self, key: impl AsRef<str>) -> Option<&Entry> {
        self.inner.get(key.as_ref())
    }

    /// Iterates over all entries in ascending key order.
    pub fn values<'a>(&'a self) -> Values<'a, String, Entry> {
        self.inner.values()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::header::HeaderWriter;

    #[test]
    fn from_file_empty() {
        let mut file = tempfile::tempfile().unwrap();
        let memtable = MemTable::from_file(&mut file).unwrap();
        assert!(memtable.is_empty());
    }

    #[test]
    fn from_file_single_set() {
        let mut file = tempfile::tempfile().unwrap();
        let set = Entry::set("a", "1");
        file.write_entry_with_header(&set).unwrap();
        let memtable = MemTable::from_file(&mut file).unwrap();
        assert_eq!(memtable.len(), 1);
        assert!(memtable.contains_key(set.key()));
    }

    #[test]
    fn from_file_set_then_delete_removes_key() {
        let mut file = tempfile::tempfile().unwrap();
        let set = Entry::set("a", "1");
        let delete = Entry::delete("a");
        file.write_entry_with_header(&set).unwrap();
        file.write_entry_with_header(&delete).unwrap();
        let memtable = MemTable::from_file(&mut file).unwrap();
        assert!(memtable.is_empty());
    }

    #[test]
    fn from_file_multiple_keys() {
        let mut file = tempfile::tempfile().unwrap();
        let a = Entry::set("a", "1");
        let b = Entry::set("b", "2");
        let c = Entry::set("c", "3");
        file.write_entry_with_header(&a).unwrap();
        file.write_entry_with_header(&b).unwrap();
        file.write_entry_with_header(&c).unwrap();
        let memtable = MemTable::from_file(&mut file).unwrap();
        assert_eq!(memtable.len(), 3);
        assert!(memtable.contains_key(a.key()));
        assert!(memtable.contains_key(b.key()));
        assert!(memtable.contains_key(c.key()));
    }

    #[test]
    fn from_file_delete_nonexistent_key_is_noop() {
        let mut file = tempfile::tempfile().unwrap();
        let delete = Entry::delete("a");
        file.write_entry_with_header(&delete).unwrap();
        let memtable = MemTable::from_file(&mut file).unwrap();
        assert!(memtable.is_empty());
    }

    #[test]
    fn process_set_increments_size() {
        let mut file = tempfile::tempfile().unwrap();
        let mut memtable = MemTable::from_file(&mut file).unwrap();
        let set = Entry::set("a", "1");
        memtable.process(&set).unwrap();
        assert!(memtable.size() > 0);
    }

    #[test]
    fn process_delete_decrements_size() {
        let mut file = tempfile::tempfile().unwrap();
        let mut memtable = MemTable::from_file(&mut file).unwrap();
        let set = Entry::set("a", "1");
        let delete = Entry::delete("a");
        memtable.process(&set).unwrap();
        let size = memtable.size();
        memtable.process(&delete).unwrap();
        assert!(size > memtable.size());
        assert_eq!(memtable.size(), 0);
    }

    #[test]
    fn process_set_overwrite_does_not_double_count_size() {
        let mut file = tempfile::tempfile().unwrap();
        let mut memtable = MemTable::from_file(&mut file).unwrap();
        let set = Entry::set("a", "1");
        memtable.process(&set).unwrap();
        let size_after_set = memtable.size();
        memtable.process(&set).unwrap();
        assert_eq!(size_after_set, memtable.size());
    }

    #[test]
    fn process_delete_nonexistent_key_is_noop() {
        let mut file = tempfile::tempfile().unwrap();
        let mut memtable = MemTable::from_file(&mut file).unwrap();
        let delete = Entry::delete("a");
        memtable.process(&delete).unwrap();
        assert_eq!(memtable.size(), 0);
    }

    #[test]
    fn clear_resets_size_and_entries() {
        let mut file = tempfile::tempfile().unwrap();
        let mut memtable = MemTable::from_file(&mut file).unwrap();
        let set = Entry::set("a", "1");
        memtable.process(&set).unwrap();
        memtable.clear();
        assert!(memtable.is_empty());
        assert_eq!(memtable.size(), 0);
    }

    #[test]
    fn get_returns_some_for_existing_key() {
        let mut file = tempfile::tempfile().unwrap();
        let mut memtable = MemTable::from_file(&mut file).unwrap();
        let set = Entry::set("a", "1");
        memtable.process(&set).unwrap();
        let result = memtable.get("a").unwrap();
        assert_eq!(set.key(), result.key());
        assert_eq!(set.value(), result.value());
    }

    #[test]
    fn get_returns_none_for_absent_key() {
        let mut file = tempfile::tempfile().unwrap();
        let memtable = MemTable::from_file(&mut file).unwrap();
        assert!(memtable.get("a").is_none());
    }

    #[test]
    fn contains_key_true_for_existing() {
        let mut file = tempfile::tempfile().unwrap();
        let mut memtable = MemTable::from_file(&mut file).unwrap();
        let set = Entry::set("a", "1");
        memtable.process(&set).unwrap();
        assert!(memtable.contains_key(set.key()));
    }

    #[test]
    fn contains_key_false_for_absent() {
        let mut file = tempfile::tempfile().unwrap();
        let memtable = MemTable::from_file(&mut file).unwrap();
        assert!(!memtable.contains_key("a"));
    }

    #[test]
    fn len_reflects_unique_key_count() {
        let mut file = tempfile::tempfile().unwrap();
        let mut memtable = MemTable::from_file(&mut file).unwrap();
        let set_a = Entry::set("a", "1");
        let set_b = Entry::set("b", "2");
        memtable.process(&set_a).unwrap();
        memtable.process(&set_b).unwrap();
        let len = memtable.len();
        memtable.process(&set_a).unwrap();
        assert_eq!(len, memtable.len());
    }

    #[test]
    fn values_iterates_in_sorted_key_order() {
        let mut file = tempfile::tempfile().unwrap();
        let mut memtable = MemTable::from_file(&mut file).unwrap();
        let set1 = Entry::set("a", "1");
        let set2 = Entry::set("b", "2");
        let set3 = Entry::set("c", "3");
        memtable.process(&set3).unwrap();
        memtable.process(&set2).unwrap();
        memtable.process(&set1).unwrap();
        let expected = vec![&set1, &set2, &set3];
        let result: Vec<_> = memtable.values().collect();
        assert_eq!(expected, result);
    }

    // --- Bloom filter test stubs ---

    #[test]
    #[ignore]
    fn flush_writes_bloomfilter_footer_to_sstable() {
        // Flush a memtable with known keys
        // Read last 4 bytes of .sst file as u32 little-endian = bit_count
        // Read bit_count.div_ceil(8) bytes before that = bloom filter bytes
        // Verify bit_count and bloom filter byte length are correct
    }

    #[test]
    #[ignore]
    fn flush_bloomfilter_size_matches_key_count() {
        // Flush N keys, read bit_count from footer
        // Assert bit_count == N * 10
    }

    #[test]
    #[ignore]
    fn bloomfilter_reports_present_for_inserted_key() {
        // Flush memtable with known keys
        // Read bloom filter from .sst footer
        // Hash a key that WAS inserted using same xxh3 double-hashing (k=7)
        // Check all 7 bit positions are set in the bloom filter
    }

    #[test]
    #[ignore]
    fn bloomfilter_reports_absent_for_missing_key() {
        // Flush memtable with known keys
        // Read bloom filter from .sst footer
        // Hash a key that was NOT inserted using same xxh3 double-hashing (k=7)
        // Check that at least one of the 7 bit positions is 0
    }
}
