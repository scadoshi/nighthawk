use super::{entry::Entry, header::Header};
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
