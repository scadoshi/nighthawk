pub mod command;
pub mod entry;
pub mod header;
pub mod memtable;

use entry::Entry;
use header::{HeaderReader, HeaderWriter};
use memtable::MemTable;
use std::{
    fs::{File, OpenOptions, create_dir_all, read_dir},
    io::{Seek, SeekFrom},
    time::{SystemTime, UNIX_EPOCH},
};

/// Append-only log store. Owns the data file and in-memory key-to-entry memtable.
#[derive(Debug)]
pub struct Log {
    file: File,
    path: String,
    memtable: MemTable,
}

/// Default log file path.
pub const DATA_DIR_PATH: &str = "data";
pub const MEMTABLE_PATH: &str = "data/memtable";
pub const SSTABLES_DIR_PATH: &str = "data/sstables";

impl Log {
    /// Opens or creates a log file and rebuilds the memtable from its contents. Takes truncate flag
    /// informing whether to overwrite or append existing file content.
    pub fn new(path: impl AsRef<str> + Into<String>, truncate: bool) -> anyhow::Result<Self> {
        create_dir_all(DATA_DIR_PATH)?;
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(truncate)
            .read(true)
            .write(true)
            .open(path.as_ref())?;
        let memtable = MemTable::from_file(&mut file)?;
        Ok(Self {
            file,
            path: path.into(),
            memtable,
        })
    }

    /// Appends an entry with header to end of file in [`Log`]. Returns the byte offset where it was written.
    pub fn write(&mut self, entry: &Entry) -> anyhow::Result<u64> {
        let wrote_at = self.file.write_entry_with_header(entry)?;
        self.file.sync_all()?;
        Ok(wrote_at)
    }

    pub fn get(&mut self, key: impl AsRef<str>) -> anyhow::Result<Option<Entry>> {
        // Try memtable first
        if let Some(entry) = self.memtable.get(key.as_ref()) {
            return Ok(Some(entry.clone()));
        }
        // Then if not found sift through SSTables
        // Only does linear search for now
        create_dir_all(SSTABLES_DIR_PATH)?;
        let mut entries: Vec<_> = read_dir(SSTABLES_DIR_PATH)?
            .filter_map(|e| e.ok())
            .collect();
        entries.sort_by(|a, b| b.file_name().cmp(&a.file_name()));
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

    /// Reads the next entry from the current file cursor position.
    pub fn read_next(&mut self) -> anyhow::Result<Option<Entry>> {
        self.file.read_next_entry_with_header()
    }

    pub fn flush(&mut self) -> anyhow::Result<()> {
        create_dir_all(SSTABLES_DIR_PATH)?;
        let ts = SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros();
        let path = format!("{}/{:020}.sst", SSTABLES_DIR_PATH, ts);
        let mut file = OpenOptions::new()
            .truncate(true)
            .create(true)
            .write(true)
            .open(path)?;
        for entry in self.memtable.values() {
            file.write_entry_with_header(entry)?;
        }
        file.sync_all()?;
        self.file.set_len(0)?;
        self.file.seek(SeekFrom::Start(0))?;
        self.memtable.clear();
        Ok(())
    }

    pub fn maybe_flush(&mut self) -> anyhow::Result<()> {
        if self.memtable.size() > 4 * (1 << 20) {
            self.flush()
        } else {
            Ok(())
        }
    }
}
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
#[cfg(test)]
mod tests {
    use super::*;

    fn temp_log() -> (Log, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.wal");
        let log = Log::new(path.to_str().unwrap(), true).unwrap();
        (log, dir)
    }

    #[test]
    fn new_creates_empty_memtable() {
        let (log, _dir) = temp_log();
        assert!(log.memtable.is_empty());
    }

    #[test]
    fn write_returns_offset() {
        let (mut log, _dir) = temp_log();
        let entry = Entry::Set {
            key: "a".to_string(),
            value: "1".to_string(),
        };
        let offset = log.write(&entry).unwrap();
        assert_eq!(offset, 0);
    }

    #[test]
    fn write_then_read_returns_entry() {
        let (mut log, _dir) = temp_log();
        let entry = Entry::Set {
            key: "a".to_string(),
            value: "1".to_string(),
        };
        log.write(&entry).unwrap();
        log.file.seek(SeekFrom::Start(0)).unwrap();
        let result = log.read_next().unwrap().unwrap();
        assert_eq!(result.key(), "a");
        assert_eq!(result.value(), Some("1"));
    }

    #[test]
    fn write_does_not_modify_memtable() {
        let (mut log, _dir) = temp_log();
        let entry = Entry::Set {
            key: "a".to_string(),
            value: "1".to_string(),
        };
        log.write(&entry).unwrap();
        assert!(log.memtable.is_empty());
    }

    #[test]
    fn read_next_on_empty_returns_none() {
        let (mut log, _dir) = temp_log();
        let result = log.read_next().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn new_rebuilds_memtable_from_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        let path_str = path.to_str().unwrap();
        {
            let mut log = Log::new(path_str, true).unwrap();
            let entry = Entry::Set {
                key: "a".to_string(),
                value: "1".to_string(),
            };
            log.write(&entry).unwrap();
        }
        // Reopen — memtable should rebuild from file contents.
        let log = Log::new(path_str, false).unwrap();
        assert_eq!(log.memtable.len(), 1);
        assert!(log.memtable.contains_key("a"));
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn get_returns_entry_from_memtable() {
        // write an entry and process it into memtable — get() should return it without touching disk
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn get_returns_none_when_absent_from_both() {
        // get() on an empty log with no SSTables returns None
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn get_finds_entry_in_sstable_after_flush() {
        // write and flush a key to SSTable, then get() should find it even though memtable is empty
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn get_returns_newest_when_key_in_multiple_sstables() {
        // flush key="a" value="1", then flush key="a" value="2" — get("a") should return "2"
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn flush_creates_sstable_file_on_disk() {
        // after calling flush() with a non-empty memtable, a .sst file should exist in sstables dir
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn flush_clears_memtable() {
        // after flush(), memtable.is_empty() should be true
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn flush_truncates_wal() {
        // after flush(), the WAL file length should be 0
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn maybe_flush_does_not_flush_when_below_threshold() {
        // a small memtable should not trigger a flush — no SSTable file created
        todo!()
    }
}
