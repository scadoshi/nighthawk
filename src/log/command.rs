use crate::{
    log::{Log, entry::Entry},
    tui,
};
use anyhow::anyhow;
use std::io::{Seek, SeekFrom};
use thiserror::Error;

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
    Quit,
    Help,
}

impl TryFrom<&str> for Command {
    type Error = CommandError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut parts = value.split_whitespace();
        let Some(command_str) = parts.next().map(|s| s.to_lowercase()) else {
            return Err(Self::Error::TooFewParts);
        };

        if command_str == "set" || command_str == "s" {
            let (Some(k), Some(v)) = (
                parts.next().map(|s| s.to_string()),
                parts.next().map(|s| s.to_string()),
            ) else {
                return Err(Self::Error::TooFewParts);
            };
            if parts.next().is_some() {
                return Err(Self::Error::TooManyParts);
            }
            Ok(Self::Set { k, v })
        } else if command_str == "get" || command_str == "g" {
            let Some(k) = parts.next().map(|s| s.to_string()) else {
                return Err(Self::Error::TooFewParts);
            };
            if parts.next().is_some() {
                return Err(Self::Error::TooManyParts);
            }
            Ok(Self::Get { k })
        } else if command_str == "delete" || command_str == "del" || command_str == "d" {
            let Some(k) = parts.next().map(|s| s.to_string()) else {
                return Err(Self::Error::TooFewParts);
            };
            if parts.next().is_some() {
                return Err(Self::Error::TooManyParts);
            }
            Ok(Self::Delete { k })
        } else if command_str == "quit" || command_str == "q" || command_str == "exit" {
            Ok(Self::Quit)
        } else if command_str == "help" || command_str == "h" {
            Ok(Self::Help)
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
}

pub trait Execute {
    fn execute(&mut self, command: Command) -> anyhow::Result<()>;
}

impl Execute for Log {
    fn execute(&mut self, command: Command) -> anyhow::Result<()> {
        match command {
            Command::Set { k, v } => {
                let entry = Entry::Set { k, v };
                self.write(&entry)?;
                println!("{} => {}", entry.k(), entry.v().unwrap());
            }
            Command::Get { k } => {
                let Some(&offset) = self.index.get(&k) else {
                    println!("{} not found", k);
                    return Ok(());
                };
                self.file.seek(SeekFrom::Start(offset))?;
                match self.read_next()? {
                    Some(Entry::Set { v, .. }) => println!("{} => {}", k, v),
                    Some(Entry::Delete { .. }) | None => {
                        return Err(anyhow!("Entry at offset {} corrupted.", offset,));
                    }
                };
            }
            Command::Delete { k } => {
                match self.index.remove(&k) {
                    Some(_) => {
                        let entry = Entry::Delete { k };
                        self.write(&entry)?;
                        println!("{} deleted", entry.k())
                    }
                    None => println!("{} not found", k),
                };
            }
            Command::Quit => {}
            Command::Help => {
                tui::hr();
                tui::command_hint();
            }
        }

        if self.megabytes()? > 10 {
            self.merge()?
        }

        Ok(())
    }
}
