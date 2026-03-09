use crate::{
    log::{Log, entry::Entry},
    tui,
};
use anyhow::anyhow;
use std::io::{Seek, SeekFrom};
use thiserror::Error;

/// Errors from parsing user input into a command.
#[derive(Debug, Error)]
pub enum CommandError {
    /// Unrecognized command name.
    #[error("unrecognized command")]
    UnrecognizedCommand,
    /// Missing required arguments.
    #[error("missing required arguments")]
    MissingRequiredArguments,
    /// Extra arguments provided.
    #[error("too many arguments")]
    TooManyArguments,
}

/// A parsed user command. Not all variants produce log entries (e.g. Quit, Help).
#[derive(Debug)]
pub enum Command {
    /// Store a key-value pair.
    Set { k: String, v: String },
    /// Retrieve the value for a key.
    Get { k: String },
    /// Remove a key.
    Delete { k: String },
    /// Exit the REPL.
    Quit,
    /// Print available commands.
    Help,
}

impl TryFrom<&str> for Command {
    type Error = CommandError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut parts = value.split_whitespace();
        let Some(command_str) = parts.next().map(|s| s.to_lowercase()) else {
            return Err(Self::Error::MissingRequiredArguments);
        };

        if command_str == "set" || command_str == "s" {
            let (Some(k), Some(v)) = (
                parts.next().map(|s| s.to_string()),
                parts.next().map(|s| s.to_string()),
            ) else {
                return Err(Self::Error::MissingRequiredArguments);
            };
            if parts.next().is_some() {
                return Err(Self::Error::TooManyArguments);
            }
            Ok(Self::Set { k, v })
        } else if command_str == "get" || command_str == "g" {
            let Some(k) = parts.next().map(|s| s.to_string()) else {
                return Err(Self::Error::MissingRequiredArguments);
            };
            if parts.next().is_some() {
                return Err(Self::Error::TooManyArguments);
            }
            Ok(Self::Get { k })
        } else if command_str == "delete" || command_str == "del" || command_str == "d" {
            let Some(k) = parts.next().map(|s| s.to_string()) else {
                return Err(Self::Error::MissingRequiredArguments);
            };
            if parts.next().is_some() {
                return Err(Self::Error::TooManyArguments);
            }
            Ok(Self::Delete { k })
        } else if command_str == "quit" || command_str == "q" || command_str == "exit" {
            Ok(Self::Quit)
        } else if command_str == "help" || command_str == "h" {
            Ok(Self::Help)
        } else {
            Err(Self::Error::UnrecognizedCommand)
        }
    }
}

impl Command {
    /// Loops on stdin until a valid command is parsed.
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

/// Runs a command against a log store. Separated from `Log` so command
/// handling logic lives alongside the `Command` type.
pub trait Execute {
    /// Dispatches a command: reads/writes the log and prints results.
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
                // Only write tombstone if key exists, avoids unnecessary log growth.
                match self.index.remove(&k) {
                    Some(_) => {
                        let entry = Entry::Delete { k };
                        self.write(&entry)?;
                        println!("{} deleted", entry.k())
                    }
                    None => println!("{} not found", k),
                };
            }
            // Quit logic handled in run loop to avoid hard quit
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
