use std::{
    collections::HashMap,
    io::{Read, Seek, SeekFrom, Write},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("Invalid action")]
    InvalidAction,
    #[error("Not enough parts")]
    NotEnoughParts,
    #[error("Too many parts")]
    TooManyParts,
}

#[derive(Debug)]
pub enum Command {
    Set { k: String, v: String },
    Get { k: String },
    Delete { k: String },
}

impl TryFrom<&str> for Command {
    type Error = CommandError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut parts = value.split_whitespace();
        let Some(action) = parts.next().map(|s| s.to_lowercase()) else {
            return Err(Self::Error::NotEnoughParts);
        };
        let Some(k) = parts.next().map(|s| s.to_string()) else {
            return Err(Self::Error::NotEnoughParts);
        };

        if action == "set" {
            let Some(v) = parts.next().map(|s| s.to_string()) else {
                return Err(Self::Error::NotEnoughParts);
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
            Err(Self::Error::InvalidAction)
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
                    input_str.clear();
                }
            }
        }
    }
    pub fn execute<T>(&self, buf: &mut T, index: &mut HashMap<String, u64>) -> anyhow::Result<()>
    where
        T: Read + Write + Seek,
    {
        match self {
            Self::Set { k, v } => {
                buf.seek(SeekFrom::End(0))?;
                let offset = buf.stream_position()?;
                let bytes = wincode::serialize(&(k, v))?;
                buf.write_all(&bytes)?;
                index.insert(k.clone(), offset);
            }
            Self::Get { k } => {
                let Some(&offset) = index.get(k) else {
                    println!("{} not found", k);
                    return Ok(());
                };
                buf.seek(SeekFrom::Start(offset))?;
                let mut data = Vec::new();
                buf.read_to_end(&mut data)?;
                let (_k, v) = wincode::deserialize::<(String, String)>(&data)?;
                println!("{}", v);
            }
            Self::Delete { k } => {
                index.remove(k);
            }
        }
        Ok(())
    }
}
