use super::entry::Entry;
use std::{collections::HashMap, fs::File, io::Read};

pub type Index = HashMap<String, u64>;

pub trait IndexOps {
    fn from_file(buf: &mut File) -> anyhow::Result<Index>;
}

impl IndexOps for Index {
    fn from_file(buf: &mut File) -> anyhow::Result<Index> {
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
}
