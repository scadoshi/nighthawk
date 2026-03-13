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

        match command_str.as_str() {
            "set" | "s" => {
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
            }
            "get" | "g" => {
                let Some(k) = parts.next().map(|s| s.to_string()) else {
                    return Err(Self::Error::MissingRequiredArguments);
                };
                if parts.next().is_some() {
                    return Err(Self::Error::TooManyArguments);
                }
                Ok(Self::Get { k })
            }
            "delete" | "del" | "d" => {
                let Some(k) = parts.next().map(|s| s.to_string()) else {
                    return Err(Self::Error::MissingRequiredArguments);
                };
                if parts.next().is_some() {
                    return Err(Self::Error::TooManyArguments);
                }
                Ok(Self::Delete { k })
            }
            "quit" | "q" | "exit" => Ok(Self::Quit),
            "help" | "h" => Ok(Self::Help),
            _ => Err(Self::Error::UnrecognizedCommand),
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
                if self.index.contains_key(&k) {
                    let entry = Entry::Delete { k };
                    self.write(&entry)?;
                    self.index.remove(entry.k());
                    println!("{} deleted", entry.k());
                } else {
                    println!("{} not found", k);
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_set_from_standard() {
        let result = Command::try_from("set a b");
        assert!(matches!(result, Ok(Command::Set { .. })));
    }

    #[test]
    fn command_set_from_alias_s() {
        let result = Command::try_from("s a b");
        assert!(matches!(result, Ok(Command::Set { .. })));
    }

    #[test]
    fn command_get_from_standard() {
        let result = Command::try_from("get a");
        assert!(matches!(result, Ok(Command::Get { .. })));
    }

    #[test]
    fn command_get_from_alias_g() {
        let result = Command::try_from("g a");
        assert!(matches!(result, Ok(Command::Get { .. })));
    }

    #[test]
    fn command_delete_from_standard() {
        let result = Command::try_from("delete a");
        assert!(matches!(result, Ok(Command::Delete { .. })));
    }

    #[test]
    fn command_delete_from_alias_del() {
        let result = Command::try_from("del a");
        assert!(matches!(result, Ok(Command::Delete { .. })));
    }

    #[test]
    fn command_delete_from_alias_d() {
        let result = Command::try_from("d a");
        assert!(matches!(result, Ok(Command::Delete { .. })));
    }

    #[test]
    fn command_quit_from_standard() {
        let result = Command::try_from("quit");
        assert!(matches!(result, Ok(Command::Quit)));
    }

    #[test]
    fn command_quit_from_alias_q() {
        let result = Command::try_from("q");
        assert!(matches!(result, Ok(Command::Quit)));
    }

    #[test]
    fn command_quit_from_alias_exit() {
        let result = Command::try_from("exit");
        assert!(matches!(result, Ok(Command::Quit)));
    }

    #[test]
    fn command_help_from_standard() {
        let result = Command::try_from("help");
        assert!(matches!(result, Ok(Command::Help)));
    }

    #[test]
    fn command_help_from_alias_h() {
        let result = Command::try_from("h");
        assert!(matches!(result, Ok(Command::Help)));
    }

    #[test]
    fn command_err_empty_input() {
        let result = Command::try_from("");
        assert!(matches!(
            result,
            Err(CommandError::MissingRequiredArguments)
        ));
    }

    #[test]
    fn command_err_set_missing_value() {
        let result = Command::try_from("set a");
        assert!(matches!(
            result,
            Err(CommandError::MissingRequiredArguments)
        ));
    }

    #[test]
    fn command_err_set_missing_key_and_value() {
        let result = Command::try_from("set");
        assert!(matches!(
            result,
            Err(CommandError::MissingRequiredArguments)
        ));
    }

    #[test]
    fn command_err_get_missing_key() {
        let result = Command::try_from("get");
        assert!(matches!(
            result,
            Err(CommandError::MissingRequiredArguments)
        ));
    }

    #[test]
    fn command_err_delete_missing_key() {
        let result = Command::try_from("delete");
        assert!(matches!(
            result,
            Err(CommandError::MissingRequiredArguments)
        ));
    }

    #[test]
    fn command_err_set_too_many_arguments() {
        let result = Command::try_from("set a b c");
        assert!(matches!(result, Err(CommandError::TooManyArguments)));
    }

    #[test]
    fn command_err_get_too_many_arguments() {
        let result = Command::try_from("get a b");
        assert!(matches!(result, Err(CommandError::TooManyArguments)));
    }

    #[test]
    fn command_err_delete_too_many_arguments() {
        let result = Command::try_from("delete a b");
        assert!(matches!(result, Err(CommandError::TooManyArguments)));
    }

    #[test]
    fn command_err_unrecognized_command() {
        let result = Command::try_from("foo");
        assert!(matches!(result, Err(CommandError::UnrecognizedCommand)));
    }

    fn temp_log() -> (Log, tempfile::TempDir) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        let log = Log::new(path.to_str().unwrap(), true).unwrap();
        (log, dir)
    }

    #[test]
    fn execute_set_adds_to_index() {
        let (mut log, _dir) = temp_log();
        let cmd = Command::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        log.execute(cmd).unwrap();
        assert_eq!(log.index.len(), 1);
        assert!(log.index.contains_key("a"));
    }

    #[test]
    fn execute_get_existing_key() {
        let (mut log, _dir) = temp_log();
        let set = Command::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        let get = Command::Get { k: "a".to_string() };
        log.execute(set).unwrap();
        // Should not error — key exists.
        log.execute(get).unwrap();
    }

    #[test]
    fn execute_get_missing_key() {
        let (mut log, _dir) = temp_log();
        let get = Command::Get { k: "a".to_string() };
        // Should not error — prints "not found" and returns Ok.
        log.execute(get).unwrap();
    }

    #[test]
    fn execute_delete_existing_key_removes_from_index() {
        let (mut log, _dir) = temp_log();
        let set = Command::Set {
            k: "a".to_string(),
            v: "1".to_string(),
        };
        let delete = Command::Delete { k: "a".to_string() };
        log.execute(set).unwrap();
        log.execute(delete).unwrap();
        assert!(log.index.is_empty());
    }

    #[test]
    fn execute_delete_missing_key() {
        let (mut log, _dir) = temp_log();
        let delete = Command::Delete { k: "a".to_string() };
        // Should not error — prints "not found" and returns Ok.
        log.execute(delete).unwrap();
        assert!(log.index.is_empty());
    }

    #[test]
    fn execute_delete_persists_tombstone() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        let path_str = path.to_str().unwrap();
        {
            let mut log = Log::new(path_str, true).unwrap();
            let set = Command::Set {
                k: "a".to_string(),
                v: "1".to_string(),
            };
            let delete = Command::Delete { k: "a".to_string() };
            log.execute(set).unwrap();
            log.execute(delete).unwrap();
        }
        // Reopen — tombstone should remove key during index rebuild.
        let log = Log::new(path_str, false).unwrap();
        assert!(log.index.is_empty());
    }

    #[test]
    fn execute_quit_is_noop() {
        let (mut log, _dir) = temp_log();
        log.execute(Command::Quit).unwrap();
        assert!(log.index.is_empty());
    }

    #[test]
    fn execute_help_is_noop() {
        let (mut log, _dir) = temp_log();
        log.execute(Command::Help).unwrap();
        assert!(log.index.is_empty());
    }

    #[test]
    fn execute_set_overwrite_updates_value() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.log");
        let path_str = path.to_str().unwrap();
        {
            let mut log = Log::new(path_str, true).unwrap();
            let first = Command::Set {
                k: "a".to_string(),
                v: "1".to_string(),
            };
            let second = Command::Set {
                k: "a".to_string(),
                v: "2".to_string(),
            };
            log.execute(first).unwrap();
            log.execute(second).unwrap();
        }
        // Reopen and get — should read latest value.
        let mut log = Log::new(path_str, false).unwrap();
        assert_eq!(log.index.len(), 1);
        let get = Command::Get { k: "a".to_string() };
        // Should not error — the offset points to the second set.
        log.execute(get).unwrap();
    }
}
