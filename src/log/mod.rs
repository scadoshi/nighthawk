pub mod command;
pub mod compact;
pub mod entry;
pub mod header;
pub mod memtable;

use entry::Entry;
use header::{HeaderReader, HeaderWriter};
use memtable::MemTable;
use std::{
    cmp::Reverse,
    fs::{File, OpenOptions, create_dir_all, read_dir},
    io::{Seek, SeekFrom},
    path::PathBuf,
};

/// Root directory for all persisted data.
pub const DATA_PATH: &str = "data";
/// Path to the write-ahead log (WAL) file.
pub const MEMTABLE_PATH: &str = "data/memtable";
/// Directory containing flushed SSTable files.
pub const SSTABLES_PATH: &str = "data/sstables";
/// Multiple which flush_count is checked against in order to determine compaction timing
const COMPACT_EVERY_N_FLUSHES: u64 = 10;

/// Append-only log store. Owns the data file and in-memory key-to-entry memtable.
#[derive(Debug)]
pub struct Log {
    memtable_path: PathBuf,
    memtable_file: File,
    memtable: MemTable,
    sstables_path: PathBuf,
    flush_count: u64,
}

impl Log {
    /// Opens or creates a log file and rebuilds the memtable from its contents. Takes truncate flag
    /// informing whether to overwrite or append existing file content.
    pub fn new(
        data_path: impl Into<PathBuf>,
        memtable_path: impl Into<PathBuf>,
        sstables_path: impl Into<PathBuf>,
        truncate: bool,
    ) -> anyhow::Result<Self> {
        // Initialize paths and dirs
        let data_path = data_path.into();
        let memtable_path = memtable_path.into();
        let sstables_path = sstables_path.into();
        create_dir_all(&data_path)?;
        create_dir_all(&sstables_path)?;
        // Open WAL and initialize memtable
        let mut memtable_file = OpenOptions::new()
            .create(true)
            .truncate(truncate)
            .read(true)
            .write(true)
            .open(&memtable_path)?;
        let memtable = MemTable::from_file(&mut memtable_file)?;
        // SSTable count equates to flush count
        let flush_count = read_dir(&sstables_path)?.count() as u64;
        // Return
        Ok(Self {
            memtable_file,
            memtable_path,
            memtable,
            sstables_path,
            flush_count,
        })
    }

    /// Appends an entry with header to the WAL and syncs to disk, then applies it to the memtable.
    pub fn write(&mut self, entry: &Entry) -> anyhow::Result<()> {
        self.memtable_file.write_entry_with_header(entry)?;
        self.memtable_file.sync_all()?;
        self.memtable.process(entry)?;
        Ok(())
    }

    /// Looks up a key: checks the memtable first, then scans SSTables newest-to-oldest.
    /// Returns `None` if the key is absent from both layers.
    pub fn get(&self, key: impl AsRef<str>) -> anyhow::Result<Option<Entry>> {
        // Try memtable first
        if let Some(entry) = self.memtable.get(key.as_ref()) {
            return Ok(Some(entry.clone()));
        }
        // Then if not found sift through SSTables
        // Only does linear search for now
        let Ok(entries) = read_dir(&self.sstables_path) else {
            return Ok(None);
        };
        let mut entries: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        entries.sort_by_key(|e| Reverse(e.file_name()));
        for entry in entries {
            let mut file = OpenOptions::new().read(true).open(entry.path())?;
            while let Some(entry @ Entry::Set { .. }) = file.read_next_entry_with_header()? {
                if entry.key() == key.as_ref() {
                    return Ok(Some(entry));
                }
            }
        }
        Ok(None)
    }

    /// Returns `true` if `key` exists in the memtable or any SSTable.
    pub fn contains(&self, key: impl AsRef<str>) -> anyhow::Result<bool> {
        self.get(key).map(|o| o.is_some())
    }

    /// Writes all memtable entries sorted by key to a new timestamped SSTable file,
    /// then truncates the WAL and clears the memtable.
    pub fn flush(&mut self) -> anyhow::Result<()> {
        self.memtable.flush_to(self.sstables_path.clone())?;
        self.memtable_file.set_len(0)?;
        self.memtable_file.seek(SeekFrom::Start(0))?;
        self.flush_count += 1;
        if self.flush_count.is_multiple_of(COMPACT_EVERY_N_FLUSHES) {
            self.compact()?;
        }
        Ok(())
    }

    /// Flushes to an SSTable if the memtable has exceeded the 4 MB size threshold.
    pub fn maybe_flush(&mut self) -> anyhow::Result<()> {
        if self.memtable.should_flush() {
            self.flush()
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_log() -> (tempfile::TempDir, Log) {
        let dir = tempfile::tempdir().unwrap();
        let log = Log::new(
            dir.path(),
            dir.path().join("memtable"),
            dir.path().join("sstables"),
            true,
        )
        .unwrap();
        (dir, log)
    }

    #[test]
    fn new_creates_empty_memtable() {
        let (_dir, log) = temp_log();
        assert!(log.memtable.is_empty());
    }

    #[test]
    fn new_rebuilds_memtable_from_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let memtable_path = dir.path().join("test.log");
        let sstables_path = dir.path().join("sstables");
        {
            let mut log = Log::new(dir.path(), &memtable_path, &sstables_path, true).unwrap();
            let set = Entry::set("a", "1");
            log.write(&set).unwrap();
        }
        let log = Log::new(dir.path(), &memtable_path, &sstables_path, false).unwrap();
        assert_eq!(log.memtable.len(), 1);
        assert!(log.memtable.contains_key("a"));
    }

    #[test]
    fn get_returns_entry_from_memtable() {
        let (_dir, mut log) = temp_log();
        let set = Entry::set("a", "1");
        log.write(&set).unwrap();
        let result = log.get("a").unwrap().unwrap();
        assert_eq!(result.key(), set.key());
        assert_eq!(result.value(), set.value());
    }

    #[test]
    fn get_returns_none_when_absent_from_both() {
        let (_dir, log) = temp_log();
        assert!(log.get("a").unwrap().is_none());
    }

    #[test]
    fn get_finds_entry_in_sstable_after_flush() {
        let (_dir, mut log) = temp_log();
        let set = Entry::set("a", "1");
        log.write(&set).unwrap();
        log.flush().unwrap();
        let result = log.get("a").unwrap().unwrap();
        assert_eq!(set.key(), result.key());
        assert_eq!(set.value(), result.value());
    }

    #[test]
    fn get_returns_newest_when_key_in_multiple_sstables() {
        let (_dir, mut log) = temp_log();
        let set1 = Entry::set("a", "1");
        log.write(&set1).unwrap();
        let set2 = Entry::set("a", "2");
        log.write(&set2).unwrap();
        log.flush().unwrap();
        assert_eq!(log.get("a").unwrap().unwrap().value(), Some("2"));
    }

    #[test]
    fn flush_creates_sstable_file_on_disk() {
        let (_dir, mut log) = temp_log();
        let set = Entry::set("a", "1");
        log.write(&set).unwrap();
        log.flush().unwrap();
        let sst_exists = read_dir(log.sstables_path)
            .unwrap()
            .filter_map(|e| e.ok())
            .any(|e| e.path().extension().is_some_and(|ext| ext == "sst"));
        assert!(sst_exists);
    }

    #[test]
    fn flush_clears_memtable() {
        let (_dir, mut log) = temp_log();
        let set = Entry::set("a", "1");
        log.write(&set).unwrap();
        log.flush().unwrap();
        assert!(log.memtable.is_empty());
    }

    #[test]
    fn flush_truncates_wal() {
        let (_dir, mut log) = temp_log();
        let set = Entry::set("a", "1");
        log.write(&set).unwrap();
        log.flush().unwrap();
        assert!(log.memtable_file.metadata().unwrap().len() == 0);
    }

    #[test]
    fn maybe_flush_does_not_flush_when_below_threshold() {
        let (_dir, mut log) = temp_log();
        let set = Entry::set("a", "1");
        log.write(&set).unwrap();
        log.maybe_flush().unwrap();
        assert!(!log.memtable.is_empty());
        assert!(log.memtable_file.metadata().unwrap().len() != 0);
        let sst_exists = read_dir(&log.sstables_path)
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .any(|e| e.path().extension().is_some_and(|ext| ext == "sst"));
        assert!(!sst_exists);
    }
}
