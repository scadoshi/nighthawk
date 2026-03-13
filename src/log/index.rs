use super::{entry::Entry, header::HeaderReader};
use std::{collections::HashMap, fs::File, io::Seek};

/// Builds a key-to-offset index by scanning a log file.
pub trait Index {
    /// Scans the file from the beginning, recording the latest offset per key.
    fn from_file(buf: &mut File) -> anyhow::Result<Self>
    where
        Self: Sized;
}

impl Index for HashMap<String, u64> {
    fn from_file(file: &mut File) -> anyhow::Result<Self> {
        let mut index = HashMap::<String, u64>::new();
        file.seek(std::io::SeekFrom::Start(0))?;
        loop {
            let offset = file.stream_position()?;
            match file.read_next_entry_with_header() {
                Ok(Some(Entry::Set { k, .. })) => index.insert(k, offset),
                Ok(Some(Entry::Delete { k })) => index.remove(&k),
                Ok(None) => break,
                Err(_) => break,
            };
        }
        Ok(index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::log::header::HeaderWriter;

    #[test]
    fn from_file_empty() {
        let mut file = tempfile::tempfile().unwrap();
        let index = HashMap::<String, u64>::from_file(&mut file).unwrap();
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
        let index = HashMap::<String, u64>::from_file(&mut file).unwrap();
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
        let index = HashMap::<String, u64>::from_file(&mut file).unwrap();
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
        let index = HashMap::<String, u64>::from_file(&mut file).unwrap();
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
        let index = HashMap::<String, u64>::from_file(&mut file).unwrap();
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
        let index = HashMap::<String, u64>::from_file(&mut file).unwrap();
        assert!(index.is_empty());
    }
}
