pub mod command;
pub mod entry;
pub mod index;

use entry::Entry;
use index::{Index, IndexOps};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions, rename},
    io::{Read, Seek, SeekFrom, Write},
};

#[derive(Debug)]
pub struct Log {
    file: File,
    index: Index,
}

pub const DATA_PATH: &str = "data.log";
pub const TEMP_PATH: &str = "temp.log";

impl Log {
    pub fn new() -> anyhow::Result<Self> {
        let mut file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(DATA_PATH)?;
        let index = Index::from_file(&mut file)?;
        Ok(Self { file, index })
    }

    pub fn merge(&mut self) -> anyhow::Result<()> {
        let mut temp = OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(TEMP_PATH)?;

        let mut bytes = Vec::<u8>::new();
        self.file.seek(SeekFrom::Start(0))?;
        self.file.read_to_end(&mut bytes)?;

        let mut entries = HashMap::<String, Entry>::new();
        let mut offset: u64 = 0;

        while (offset as usize) < bytes.len() {
            let slice = &bytes[offset as usize..];
            match wincode::deserialize::<Entry>(slice) {
                Ok(entry @ Entry::Set { .. }) => {
                    let size = wincode::serialized_size(&entry)?;
                    entries.insert(entry.k().to_owned(), entry);
                    offset += size;
                }
                Ok(entry @ Entry::Delete { .. }) => {
                    let size = wincode::serialized_size(&entry)?;
                    entries.remove(entry.k());
                    offset += size;
                }
                Err(_) => break,
            }
        }

        self.index.clear();
        let mut offset: u64 = 0;
        for entry in entries.values() {
            let bytes = wincode::serialize(&entry)?;
            temp.write_all(&bytes)?;
            self.index.insert(entry.k().to_owned(), offset);
            offset += bytes.len() as u64;
        }

        temp.sync_all()?;

        rename(TEMP_PATH, DATA_PATH)?;

        self.file = OpenOptions::new().read(true).write(true).open(DATA_PATH)?;

        Ok(())
    }

    pub fn megabytes(&self) -> anyhow::Result<u64> {
        Ok(self.file.metadata()?.len() / (1 << 20))
    }
}
