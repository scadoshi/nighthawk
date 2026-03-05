use anyhow::anyhow;
use std::{
    collections::HashMap,
    io::{Read, Seek, SeekFrom, Write},
};
use thiserror::Error;
use wincode::{SchemaRead, SchemaWrite};

use crate::tui;

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Invalid command")]
    InvalidCommand,
    #[error("Too Few parts")]
    TooFewParts,
    #[error("Too many parts")]
    TooManyParts,
}

#[derive(Debug)]
pub enum Command {
    Set { k: String, v: String },
    Get { k: String },
    Delete { k: String },
}

#[derive(Debug, SchemaRead, SchemaWrite)]
pub enum Entry {
    Set { k: String, v: String },
    Delete { k: String },
}

impl Entry {
    pub fn k(&self) -> &str {
        match self {
            Self::Set { k, .. } => k.as_str(),
            Self::Delete { k } => k.as_str(),
        }
    }
}

impl TryFrom<&str> for Command {
    type Error = CommandError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut parts = value.split_whitespace();
        let Some(action) = parts.next().map(|s| s.to_lowercase()) else {
            return Err(Self::Error::TooFewParts);
        };
        let Some(k) = parts.next().map(|s| s.to_string()) else {
            return Err(Self::Error::TooFewParts);
        };

        if action == "set" {
            let Some(v) = parts.next().map(|s| s.to_string()) else {
                return Err(Self::Error::TooFewParts);
            };
            if parts.next().is_some() {
                return Err(Self::Error::TooManyParts);
            }
            Ok(Self::Set { k, v })
        } else if action == "get" {
            if parts.next().is_some() {
                return Err(Self::Error::TooManyParts);
            }
            Ok(Self::Get { k })
        } else if action == "delete" {
            if parts.next().is_some() {
                return Err(Self::Error::TooManyParts);
            }
            Ok(Self::Delete { k })
        } else {
            Err(Self::Error::InvalidCommand)
        }
    }
}

impl Command {
    pub fn unfallible_get() -> Self {
        loop {
            let mut input_str = String::new();
            std::io::stdin().read_line(&mut input_str).ok();
            match Command::try_from(input_str.as_str()) {
                Ok(input) => return input,
                Err(e) => {
                    eprintln!("Invalid input: {}", e);
                    tui::hr();
                    input_str.clear();
                }
            }
        }
    }

    pub fn execute<T>(self, buf: &mut T, index: &mut HashMap<String, u64>) -> anyhow::Result<()>
    where
        T: Read + Write + Seek,
    {
        match self {
            Self::Set { k, v } => {
                buf.seek(SeekFrom::End(0))?;
                let offset = buf.stream_position()?;
                let entry = Entry::Set { k, v };
                let bytes = wincode::serialize(&entry)?;
                buf.write_all(&bytes)?;
                index.insert(entry.k().to_owned(), offset);
            }
            Self::Get { k } => {
                let Some(&offset) = index.get(&k) else {
                    println!("{} not found", k);
                    return Ok(());
                };
                buf.seek(SeekFrom::Start(offset))?;
                let mut data = Vec::new();
                buf.read_to_end(&mut data)?;
                let Entry::Set { v, .. } = wincode::deserialize::<Entry>(&data)? else {
                    return Err(anyhow!(
                        "Entry at offset {} expected to return `Entry::Set {{ .. }}`. Database likely corrupted.",
                        offset
                    ));
                };
                println!("{}", v);
            }
            Self::Delete { k } => {
                buf.seek(SeekFrom::End(0))?;
                let entry = Entry::Delete { k };
                let bytes = wincode::serialize(&entry)?;
                buf.write_all(&bytes)?;
                index.remove(entry.k());
            }
        }
        Ok(())
    }
}
