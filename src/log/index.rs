use super::{entry::Entry, header::HeaderReader};
use std::{
    collections::HashMap,
    fs::File,
    io::Seek,
    ops::{Deref, DerefMut},
};

/// In-memory key-to-offset index backed by a `HashMap`.
/// Tracks total entries on disk to support ratio-based merge triggering.
#[derive(Debug)]
pub struct Index {
    /// Maps each live key to its byte offset in the log file.
    inner: HashMap<String, u64>,
    /// Total entries (sets + deletes) written to disk, including duplicates and tombstones.
    entry_count: u64,
}

impl Deref for Index {
    type Target = HashMap<String, u64>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Index {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl Index {
    /// Scans a log file from the beginning, building the key-to-offset map and counting total entries.
    pub fn from_file(file: &mut File) -> anyhow::Result<Self> {
        let mut inner = HashMap::<String, u64>::new();
        let mut entry_count = 0;
        file.seek(std::io::SeekFrom::Start(0))?;
        loop {
            let offset = file.stream_position()?;
            match file.read_next_entry_with_header() {
                Ok(Some(Entry::Set { k, .. })) => {
                    inner.insert(k, offset);
                    entry_count += 1;
                }
                Ok(Some(Entry::Delete { k })) => {
                    inner.remove(&k);
                    entry_count += 1;
                }
                Ok(None) => break,
                Err(_) => break,
            };
        }
        Ok(Self { inner, entry_count })
    }

    /// Increments the on-disk entry count. Call after each write to disk.
    pub fn track_write(&mut self) {
        self.entry_count += 1;
    }

    /// Returns true when the ratio of total entries to unique keys exceeds 2.
    pub fn should_merge(&self) -> bool {
        !self.inner.is_empty() && self.entry_count / self.inner.len() as u64 > 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::header::HeaderWriter;

    #[test]
    fn from_file_empty() {
        let mut file = tempfile::tempfile().unwrap();
        let index = Index::from_file(&mut file).unwrap();
        assert!(index.is_empty());
    }

    #[test]
    fn from_file_single_set() {
        let mut file = tempfile::tempfile().unwrap();
        let entry = Entry::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        file.write_entry_with_header(&entry).unwrap();
        let index = Index::from_file(&mut file).unwrap();
        assert_eq!(index.len(), 1);
        assert!(index.contains_key("a"));
    }

    #[test]
    fn from_file_set_then_delete_removes_key() {
        let mut file = tempfile::tempfile().unwrap();
        let set = Entry::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        let delete = Entry::Delete { k: "a".to_string() };
        file.write_entry_with_header(&set).unwrap();
        file.write_entry_with_header(&delete).unwrap();
        let index = Index::from_file(&mut file).unwrap();
        assert!(index.is_empty());
    }

    #[test]
    fn from_file_overwrite_keeps_latest_offset() {
        let mut file = tempfile::tempfile().unwrap();
        let first = Entry::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        let second = Entry::Set {
            k: "a".to_string(),
            v: "2".to_string(),
        };
        let first_offset = file.write_entry_with_header(&first).unwrap();
        let second_offset = file.write_entry_with_header(&second).unwrap();
        assert_ne!(first_offset, second_offset);
        let index = Index::from_file(&mut file).unwrap();
        assert_eq!(index.len(), 1);
        assert_eq!(index["a"], second_offset);
    }

    #[test]
    fn from_file_multiple_keys() {
        let mut file = tempfile::tempfile().unwrap();
        let a = Entry::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        let b = Entry::Set {
            k: "b".to_string(),
            v: "2".to_string(),
        };
        let c = Entry::Set {
            k: "c".to_string(),
            v: "3".to_string(),
        };
        file.write_entry_with_header(&a).unwrap();
        file.write_entry_with_header(&b).unwrap();
        file.write_entry_with_header(&c).unwrap();
        let index = Index::from_file(&mut file).unwrap();
        assert_eq!(index.len(), 3);
        assert!(index.contains_key("a"));
        assert!(index.contains_key("b"));
        assert!(index.contains_key("c"));
    }

    #[test]
    fn from_file_delete_nonexistent_key_is_noop() {
        let mut file = tempfile::tempfile().unwrap();
        let delete = Entry::Delete { k: "a".to_string() };
        file.write_entry_with_header(&delete).unwrap();
        let index = Index::from_file(&mut file).unwrap();
        assert!(index.is_empty());
    }

    #[test]
    fn from_file_entry_count_includes_all_entries() {
        let mut file = tempfile::tempfile().unwrap();
        let set = Entry::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        let overwrite = Entry::Set {
            k: "a".to_string(),
            v: "2".to_string(),
        };
        let delete = Entry::Delete { k: "a".to_string() };
        file.write_entry_with_header(&set).unwrap();
        file.write_entry_with_header(&overwrite).unwrap();
        file.write_entry_with_header(&delete).unwrap();
        let index = Index::from_file(&mut file).unwrap();
        assert_eq!(index.entry_count, 3);
    }

    #[test]
    fn track_write_increments_entry_count() {
        let mut file = tempfile::tempfile().unwrap();
        let mut index = Index::from_file(&mut file).unwrap();
        assert_eq!(index.entry_count, 0);
        index.track_write();
        assert_eq!(index.entry_count, 1);
        index.track_write();
        assert_eq!(index.entry_count, 2);
    }

    #[test]
    fn should_merge_false_on_empty_index() {
        let mut file = tempfile::tempfile().unwrap();
        let index = Index::from_file(&mut file).unwrap();
        assert!(!index.should_merge());
    }

    #[test]
    fn should_merge_false_when_ratio_low() {
        let mut file = tempfile::tempfile().unwrap();
        let a = Entry::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        let b = Entry::Set {
            k: "b".to_string(),
            v: "2".to_string(),
        };
        file.write_entry_with_header(&a).unwrap();
        file.write_entry_with_header(&b).unwrap();
        let index = Index::from_file(&mut file).unwrap();
        // 2 entries, 2 unique keys — ratio is 1.
        assert!(!index.should_merge());
    }

    #[test]
    fn should_merge_true_when_ratio_exceeds_threshold() {
        let mut file = tempfile::tempfile().unwrap();
        let entry = Entry::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        // Write same key 4 times — 4 entries, 1 unique key, ratio is 4.
        for _ in 0..4 {
            file.write_entry_with_header(&entry).unwrap();
        }
        let index = Index::from_file(&mut file).unwrap();
        assert!(index.should_merge());
    }

    #[test]
    fn should_merge_false_at_boundary() {
        let mut file = tempfile::tempfile().unwrap();
        let entry = Entry::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        // Write same key 2 times — 2 entries, 1 unique key, ratio is 2 (not > 2).
        file.write_entry_with_header(&entry).unwrap();
        file.write_entry_with_header(&entry).unwrap();
        let index = Index::from_file(&mut file).unwrap();
        assert!(!index.should_merge());
    }
}
