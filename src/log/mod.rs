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

#[derive(Debug)]
pub struct Log {
    file: File,
    index: HashMap<String, u64>,
}

pub const STD_PATH: &str = "data.log";
pub const TEMP_PATH: &str = "temp.log";

impl Log {
    pub fn new(path: &str, truncate: bool) -> anyhow::Result<Self> {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(truncate)
            .read(true)
            .write(true)
            .open(path)?;
        let index = Index::from_file(&mut file)?;
        Ok(Self { file, index })
    }

    pub fn write(&mut self, entry: &Entry) -> anyhow::Result<()> {
        let offset = self.file.write_entry_with_header(entry)?;
        self.index.insert(entry.k().to_owned(), offset);
        Ok(())
    }

    pub fn read_next(&mut self) -> anyhow::Result<Option<Entry>> {
        self.file.read_next_entry_with_header()
    }

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

        let mut temp = Log::new(TEMP_PATH, true)?;
        for entry in entries.values() {
            temp.write(entry)?;
        }

        rename(TEMP_PATH, STD_PATH)?;

        *self = temp;

        Ok(())
    }

    pub fn megabytes(&self) -> anyhow::Result<u64> {
        Ok(self.file.metadata()?.len() / (1 << 20))
    }
}
