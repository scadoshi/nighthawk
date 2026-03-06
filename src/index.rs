use std::{
    collections::HashMap,
    fs::OpenOptions,
    io::{Read, Write},
};

use crate::{command::Entry, run::DATA_PATH};

pub type Index = HashMap<String, u64>;

pub trait IndexOps<T>
where
    T: Read + Read,
{
    fn from_buf(buf: &mut T) -> anyhow::Result<Index>;
    fn buf_merge(&mut self, buf: &mut T) -> anyhow::Result<()>;
}

const TEMP_PATH: &str = "temp.log";

impl<T> IndexOps<T> for Index
where
    T: Read + Write,
{
    fn from_buf(buf: &mut T) -> anyhow::Result<Index> {
        let mut data = Vec::<u8>::new();
        buf.read_to_end(&mut data)?;

        let mut index = HashMap::<String, u64>::new();
        let mut offset: u64 = 0;

        while (offset as usize) < data.len() {
            let slice = &data[offset as usize..];
            match wincode::deserialize::<Entry>(slice) {
                Ok(entry @ Entry::Set { .. }) => {
                    let size = wincode::serialized_size(&entry)?;
                    index.insert(entry.k().to_owned(), offset);
                    offset += size;
                }
                Ok(entry @ Entry::Delete { .. }) => {
                    let size = wincode::serialized_size(&entry)?;
                    index.remove(entry.k());
                    offset += size;
                }
                Err(_) => break,
            }
        }

        Ok(index)
    }

    fn buf_merge(&mut self, buf: &mut T) -> anyhow::Result<()> {
        let mut temp = OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(TEMP_PATH)?;

        let mut data = Vec::<u8>::new();
        buf.read_to_end(&mut data)?;

        let mut entries = HashMap::<String, Entry>::new();
        let mut offset: u64 = 0;

        while (offset as usize) < data.len() {
            let slice = &data[offset as usize..];
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

        self.clear();
        let mut offset: u64 = 0;
        for entry in entries.values() {
            let bytes = wincode::serialize(&entry)?;
            temp.write_all(&bytes)?;
            self.insert(entry.k().to_owned(), offset);
            offset += bytes.len() as u64;
        }

        std::fs::remove_file(DATA_PATH)?;
        std::fs::rename(TEMP_PATH, DATA_PATH)?;

        *self = Self::from_buf(buf)?;

        Ok(())
    }
}
