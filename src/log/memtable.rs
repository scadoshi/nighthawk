use super::{entry::Entry, header::HeaderReader};
use std::{
    collections::{BTreeMap, btree_map::Values},
    fs::File,
    io::{Seek, SeekFrom},
};

#[derive(Debug)]
pub struct MemTable {
    inner: BTreeMap<String, Entry>,
    size: u64,
}

impl MemTable {
    fn new() -> Self {
        let inner = BTreeMap::<String, Entry>::new();
        let size = 0;
        Self { inner, size }
    }

    pub fn from_file(file: &mut File) -> anyhow::Result<Self> {
        let mut memtable = Self::new();
        file.seek(SeekFrom::Start(0))?;
        loop {
            match file.read_next_entry_with_header() {
                Ok(Some(entry)) => memtable.process(entry)?,
                Ok(None) | Err(_) => break,
            };
        }
        Ok(memtable)
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn process(&mut self, entry: Entry) -> anyhow::Result<Option<Entry>> {
        match entry {
            set @ Entry::Set { .. } => {
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

    pub fn clear(&mut self) {
        self.size = 0;
        self.inner.clear()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.size == 0 && self.inner.is_empty()
    }

    pub fn contains_key(&self, key: impl AsRef<str>) -> bool {
        self.inner.contains_key(key.as_ref())
    }

    pub fn get(&self, key: impl AsRef<str>) -> Option<&Entry> {
        self.inner.get(key.as_ref())
    }

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
        let entry = Entry::Set {
            key: "a".to_string(),
            value: "1".to_string(),
        };
        file.write_entry_with_header(&entry).unwrap();
        let memtable = MemTable::from_file(&mut file).unwrap();
        assert_eq!(memtable.len(), 1);
        assert!(memtable.contains_key("a"));
    }

    #[test]
    fn from_file_set_then_delete_removes_key() {
        let mut file = tempfile::tempfile().unwrap();
        let set = Entry::Set {
            key: "a".to_string(),
            value: "1".to_string(),
        };
        let delete = Entry::Delete { key: "a".to_string() };
        file.write_entry_with_header(&set).unwrap();
        file.write_entry_with_header(&delete).unwrap();
        let memtable = MemTable::from_file(&mut file).unwrap();
        assert!(memtable.is_empty());
    }

    #[test]
    fn from_file_multiple_keys() {
        let mut file = tempfile::tempfile().unwrap();
        let a = Entry::Set {
            key: "a".to_string(),
            value: "1".to_string(),
        };
        let b = Entry::Set {
            key: "b".to_string(),
            value: "2".to_string(),
        };
        let c = Entry::Set {
            key: "c".to_string(),
            value: "3".to_string(),
        };
        file.write_entry_with_header(&a).unwrap();
        file.write_entry_with_header(&b).unwrap();
        file.write_entry_with_header(&c).unwrap();
        let memtable = MemTable::from_file(&mut file).unwrap();
        assert_eq!(memtable.len(), 3);
        assert!(memtable.contains_key("a"));
        assert!(memtable.contains_key("b"));
        assert!(memtable.contains_key("c"));
    }

    #[test]
    fn from_file_delete_nonexistent_key_is_noop() {
        let mut file = tempfile::tempfile().unwrap();
        let delete = Entry::Delete { key: "a".to_string() };
        file.write_entry_with_header(&delete).unwrap();
        let memtable = MemTable::from_file(&mut file).unwrap();
        assert!(memtable.is_empty());
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn process_set_increments_size() {
        // size() should be > 0 after inserting a Set entry
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn process_delete_decrements_size() {
        // size() should return to 0 after inserting then deleting the same key
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn process_set_overwrite_does_not_double_count_size() {
        // setting the same key twice — size() should reflect one entry, not two
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn process_delete_nonexistent_key_is_noop() {
        // deleting a key that was never inserted — size stays 0, no panic
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn clear_resets_size_and_entries() {
        // after inserting entries then calling clear(), is_empty() is true and size() is 0
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn get_returns_some_for_existing_key() {
        // after process(Set), get() returns Some with matching key
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn get_returns_none_for_absent_key() {
        // get() on an empty memtable returns None
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn contains_key_true_for_existing() {
        // contains_key returns true for a key that was inserted
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn contains_key_false_for_absent() {
        // contains_key returns false for a key that was never inserted
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn len_reflects_unique_key_count() {
        // inserting 3 distinct keys → len() == 3; overwriting one key keeps len() == 3
        todo!()
    }

    #[test]
    #[ignore]
    #[allow(dead_code)]
    fn values_iterates_in_sorted_key_order() {
        // insert keys "c", "a", "b" — values() should yield them in order a, b, c
        todo!()
    }
}
