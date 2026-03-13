pub mod command;
pub mod entry;
pub mod header;
pub mod index;

use entry::Entry;
use header::Header;
use index::Index;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions, rename},
    io::{Seek, SeekFrom},
};

/// Append-only log store. Owns the data file and in-memory key-to-offset index.
#[derive(Debug)]
pub struct Log {
    file: File,
    path: String,
    index: HashMap<String, u64>,
}

/// Default log file path.
pub const STD_PATH: &str = "data.log";

impl Log {
    /// Opens or creates a log file and rebuilds the index from its contents.
    pub fn new(path: &str, truncate: bool) -> anyhow::Result<Self> {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(truncate)
            .read(true)
            .write(true)
            .open(path)?;
        let index = Index::from_file(&mut file)?;
        Ok(Self {
            file,
            path: path.to_owned(),
            index,
        })
    }

    /// Appends an entry to the log updating the index if it was an `Entry::Set { .. }`.
    pub fn write(&mut self, entry: &Entry) -> anyhow::Result<()> {
        let wrote_at = self.file.write_entry_with_header(entry)?;
        match entry {
            Entry::Set { .. } => {
                self.index.insert(entry.k().to_owned(), wrote_at);
            }
            Entry::Delete { .. } => (),
        };
        Ok(())
    }

    /// Reads the next entry from the current file cursor position.
    pub fn read_next(&mut self) -> anyhow::Result<Option<Entry>> {
        self.file.read_next_entry_with_header()
    }

    /// Compacts the log by deduplicating entries into a new file, then swaps it in.
    pub fn merge(&mut self) -> anyhow::Result<()> {
        let mut entries = HashMap::<String, Entry>::new();
        self.file.seek(SeekFrom::Start(0))?;
        loop {
            match self.read_next() {
                Ok(Some(entry @ Entry::Set { .. })) => entries.insert(entry.k().to_owned(), entry),
                Ok(Some(entry @ Entry::Delete { .. })) => entries.remove(entry.k()),
                Ok(None) => break, // reached end of file
                Err(_) => break,   // handle this later
            };
        }

        let temp_path = format!("{}.tmp", self.path);
        let mut temp = Log::new(&temp_path, true)?;
        for entry in entries.values() {
            temp.write(entry)?;
        }

        rename(&temp_path, &self.path)?;
        temp.path = self.path.clone();

        // Reuse temp's already-built index to avoid a redundant scan.
        *self = temp;

        Ok(())
    }

    /// Returns the file size in megabytes.
    pub fn megabytes(&self) -> anyhow::Result<u64> {
        Ok(self.file.metadata()?.len() / (1 << 20))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_log() -> (Log, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        let log = Log::new(path.to_str().unwrap(), true).unwrap();
        (log, dir)
    }

    #[test]
    fn new_creates_empty_index() {
        let (log, _dir) = temp_log();
        assert!(log.index.is_empty());
    }

    #[test]
    fn write_set_adds_to_index() {
        let (mut log, _dir) = temp_log();
        let entry = Entry::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        log.write(&entry).unwrap();
        assert_eq!(log.index.len(), 1);
        assert!(log.index.contains_key("a"));
    }

    #[test]
    fn write_delete_does_not_add_to_index() {
        let (mut log, _dir) = temp_log();
        let entry = Entry::Delete { k: "a".to_string() };
        log.write(&entry).unwrap();
        assert!(log.index.is_empty());
    }

    #[test]
    fn write_set_then_read_returns_entry() {
        let (mut log, _dir) = temp_log();
        let entry = Entry::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        log.write(&entry).unwrap();
        log.file.seek(SeekFrom::Start(0)).unwrap();
        let result = log.read_next().unwrap().unwrap();
        assert_eq!(result.k(), "a");
        assert_eq!(result.v(), Some("1"));
    }

    #[test]
    fn write_overwrite_updates_index_offset() {
        let (mut log, _dir) = temp_log();
        let first = Entry::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        let second = Entry::Set {
            k: "a".to_string(),
            v: "2".to_string(),
        };
        log.write(&first).unwrap();
        let first_offset = log.index["a"];
        log.write(&second).unwrap();
        let second_offset = log.index["a"];
        assert_ne!(first_offset, second_offset);
        assert_eq!(log.index.len(), 1);
    }

    #[test]
    fn read_next_on_empty_returns_none() {
        let (mut log, _dir) = temp_log();
        let result = log.read_next().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn megabytes_empty_file_is_zero() {
        let (log, _dir) = temp_log();
        assert_eq!(log.megabytes().unwrap(), 0);
    }

    #[test]
    fn new_rebuilds_index_from_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        let path_str = path.to_str().unwrap();
        {
            let mut log = Log::new(path_str, true).unwrap();
            let entry = Entry::Set {
                k: "a".to_string(),
                v: "1".to_string(),
            };
            log.write(&entry).unwrap();
        }
        // Reopen — index should rebuild from file contents.
        let log = Log::new(path_str, false).unwrap();
        assert_eq!(log.index.len(), 1);
        assert!(log.index.contains_key("a"));
    }

    #[test]
    fn merge_deduplicates_entries() {
        let (mut log, _dir) = temp_log();
        let first = Entry::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        let second = Entry::Set {
            k: "a".to_string(),
            v: "2".to_string(),
        };
        log.write(&first).unwrap();
        log.write(&second).unwrap();
        log.merge().unwrap();
        assert_eq!(log.index.len(), 1);
        assert!(log.index.contains_key("a"));
        // Read the only entry — should be the latest value.
        log.file.seek(SeekFrom::Start(0)).unwrap();
        let entry = log.read_next().unwrap().unwrap();
        assert_eq!(entry.k(), "a");
        assert_eq!(entry.v(), Some("2"));
        // No more entries after it.
        assert!(log.read_next().unwrap().is_none());
    }

    #[test]
    fn merge_removes_deleted_keys() {
        let (mut log, _dir) = temp_log();
        let set = Entry::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        let delete = Entry::Delete { k: "a".to_string() };
        log.write(&set).unwrap();
        log.write(&delete).unwrap();
        log.merge().unwrap();
        assert!(log.index.is_empty());
        // File should have no entries.
        log.file.seek(SeekFrom::Start(0)).unwrap();
        assert!(log.read_next().unwrap().is_none());
    }

    #[test]
    fn merge_preserves_undeleted_keys() {
        let (mut log, _dir) = temp_log();
        let a = Entry::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        let b = Entry::Set {
            k: "b".to_string(),
            v: "2".to_string(),
        };
        let delete_a = Entry::Delete { k: "a".to_string() };
        log.write(&a).unwrap();
        log.write(&b).unwrap();
        log.write(&delete_a).unwrap();
        log.merge().unwrap();
        assert_eq!(log.index.len(), 1);
        assert!(log.index.contains_key("b"));
        assert!(!log.index.contains_key("a"));
    }

    #[test]
    fn merge_shrinks_file() {
        let (mut log, _dir) = temp_log();
        // Write many duplicates to inflate the file.
        for i in 0..20 {
            let entry = Entry::Set {
                k: "a".to_string(),
                v: i.to_string(),
            };
            log.write(&entry).unwrap();
        }
        let size_before = log.file.metadata().unwrap().len();
        log.merge().unwrap();
        let size_after = log.file.metadata().unwrap().len();
        assert!(size_after < size_before);
    }

    #[test]
    fn merge_on_empty_log() {
        let (mut log, _dir) = temp_log();
        log.merge().unwrap();
        assert!(log.index.is_empty());
        log.file.seek(SeekFrom::Start(0)).unwrap();
        assert!(log.read_next().unwrap().is_none());
    }

    #[test]
    fn merge_index_offsets_are_valid() {
        let (mut log, _dir) = temp_log();
        let a = Entry::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        let b = Entry::Set {
            k: "b".to_string(),
            v: "2".to_string(),
        };
        log.write(&a).unwrap();
        log.write(&b).unwrap();
        log.merge().unwrap();
        // Each index offset should point to a readable entry with the right key.
        let index_snapshot: Vec<_> = log.index.iter().map(|(k, o)| (k.clone(), *o)).collect();
        for (key, offset) in index_snapshot {
            log.file.seek(SeekFrom::Start(offset)).unwrap();
            let entry = log.read_next().unwrap().unwrap();
            assert_eq!(entry.k(), key.as_str());
        }
    }
}
