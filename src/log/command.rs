use crate::{
    log::{Log, entry::Entry},
    tui,
};
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
    Set { key: String, value: String },
    /// Retrieve the value for a key.
    Get { key: String },
    /// Remove a key.
    Delete { key: String },
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
                let (Some(key), Some(value)) = (
                    parts.next().map(|s| s.to_string()),
                    parts.next().map(|s| s.to_string()),
                ) else {
                    return Err(Self::Error::MissingRequiredArguments);
                };
                if parts.next().is_some() {
                    return Err(Self::Error::TooManyArguments);
                }
                Ok(Self::Set { key, value })
            }
            "get" | "g" => {
                let Some(key) = parts.next().map(|s| s.to_string()) else {
                    return Err(Self::Error::MissingRequiredArguments);
                };
                if parts.next().is_some() {
                    return Err(Self::Error::TooManyArguments);
                }
                Ok(Self::Get { key })
            }
            "delete" | "del" | "d" => {
                let Some(key) = parts.next().map(|s| s.to_string()) else {
                    return Err(Self::Error::MissingRequiredArguments);
                };
                if parts.next().is_some() {
                    return Err(Self::Error::TooManyArguments);
                }
                Ok(Self::Delete { key })
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

    /// Constructs a `Set` command.
    pub fn set(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Set { key: key.into(), value: value.into() }
    }

    /// Constructs a `Get` command.
    pub fn get(key: impl Into<String>) -> Self {
        Self::Get { key: key.into() }
    }

    /// Constructs a `Delete` command.
    pub fn delete(key: impl Into<String>) -> Self {
        Self::Delete { key: key.into() }
    }

    /// Returns the key for commands that carry one (`Set`, `Get`, `Delete`), `None` otherwise.
    pub fn key(&self) -> Option<&str> {
        match self {
            Self::Set { key, .. } | Self::Get { key } | Self::Delete { key } => Some(key.as_str()),
            Self::Quit | Self::Help => None,
        }
    }

    /// Returns the value for `Set` commands, `None` for all others.
    pub fn value(&self) -> Option<&str> {
        match self {
            Self::Set { value, .. } => Some(value.as_str()),
            Self::Get { .. } | Self::Delete { .. } | Self::Quit | Self::Help => None,
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
            Command::Set { key, value } => {
                let set = Entry::set(key, value);
                self.write(&set)?;
                println!("{} => {}", set.key(), set.value().unwrap());
                self.maybe_flush()?;
            }
            Command::Get { key } => match self.get(&key)? {
                Some(Entry::Set { value, .. }) => println!("{} => {}", key, value),
                Some(Entry::Delete { .. }) => println!("Entry at {} corrupted.", key),
                None => println!("{} not found", key),
            },
            Command::Delete { key } => {
                // Only write tombstone if key exists, avoids unnecessary log growth.
                if self.contains(&key)? {
                    let delete = Entry::delete(key);
                    self.write(&delete)?;
                    println!("{} deleted", delete.key());
                } else {
                    println!("{} not found", key);
                }
                self.maybe_flush()?;
            }
            // Quit logic handled in run loop to avoid hard exit
            Command::Quit => {}
            Command::Help => {
                tui::hr();
                tui::command_hint();
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn command_set_from_standard() {
        assert!(matches!(
            Command::try_from("set a b"),
            Ok(Command::Set { .. })
        ));
    }

    #[test]
    fn command_set_from_alias_s() {
        assert!(matches!(
            Command::try_from("s a b"),
            Ok(Command::Set { .. })
        ));
    }

    #[test]
    fn command_get_from_standard() {
        assert!(matches!(
            Command::try_from("get a"),
            Ok(Command::Get { .. })
        ));
    }

    #[test]
    fn command_get_from_alias_g() {
        assert!(matches!(Command::try_from("g a"), Ok(Command::Get { .. })));
    }

    #[test]
    fn command_delete_from_standard() {
        assert!(matches!(
            Command::try_from("delete a"),
            Ok(Command::Delete { .. })
        ));
    }

    #[test]
    fn command_delete_from_alias_del() {
        assert!(matches!(
            Command::try_from("del a"),
            Ok(Command::Delete { .. })
        ));
    }

    #[test]
    fn command_delete_from_alias_d() {
        assert!(matches!(
            Command::try_from("d a"),
            Ok(Command::Delete { .. })
        ));
    }

    #[test]
    fn command_quit_from_standard() {
        assert!(matches!(Command::try_from("quit"), Ok(Command::Quit)));
    }

    #[test]
    fn command_quit_from_alias_q() {
        assert!(matches!(Command::try_from("q"), Ok(Command::Quit)));
    }

    #[test]
    fn command_quit_from_alias_exit() {
        assert!(matches!(Command::try_from("exit"), Ok(Command::Quit)));
    }

    #[test]
    fn command_help_from_standard() {
        assert!(matches!(Command::try_from("help"), Ok(Command::Help)));
    }

    #[test]
    fn command_help_from_alias_h() {
        assert!(matches!(Command::try_from("h"), Ok(Command::Help)));
    }

    #[test]
    fn command_err_empty_input() {
        assert!(matches!(
            Command::try_from(""),
            Err(CommandError::MissingRequiredArguments)
        ));
    }

    #[test]
    fn command_err_set_missing_value() {
        assert!(matches!(
            Command::try_from("set a"),
            Err(CommandError::MissingRequiredArguments)
        ));
    }

    #[test]
    fn command_err_set_missing_key_and_value() {
        assert!(matches!(
            Command::try_from("set"),
            Err(CommandError::MissingRequiredArguments)
        ));
    }

    #[test]
    fn command_err_get_missing_key() {
        assert!(matches!(
            Command::try_from("get"),
            Err(CommandError::MissingRequiredArguments)
        ));
    }

    #[test]
    fn command_err_delete_missing_key() {
        assert!(matches!(
            Command::try_from("delete"),
            Err(CommandError::MissingRequiredArguments)
        ));
    }

    #[test]
    fn command_err_set_too_many_arguments() {
        assert!(matches!(
            Command::try_from("set a b c"),
            Err(CommandError::TooManyArguments)
        ));
    }

    #[test]
    fn command_err_get_too_many_arguments() {
        assert!(matches!(
            Command::try_from("get a b"),
            Err(CommandError::TooManyArguments)
        ));
    }

    #[test]
    fn command_err_delete_too_many_arguments() {
        assert!(matches!(
            Command::try_from("delete a b"),
            Err(CommandError::TooManyArguments)
        ));
    }

    #[test]
    fn command_err_unrecognized_command() {
        assert!(matches!(
            Command::try_from("foo"),
            Err(CommandError::UnrecognizedCommand)
        ));
    }

    fn temp_log() -> (tempfile::TempDir, Log) {
        let dir = tempdir().unwrap();
        let log = Log::new(
            dir.path(),
            dir.path().join("test.log"),
            dir.path().join("sstables"),
            true,
        )
        .unwrap();
        (dir, log)
    }

    #[test]
    fn execute_set_adds_to_memtable() {
        let (_dir, mut log) = temp_log();
        let cmd = Command::set("a", "1");
        let key = cmd.key().unwrap().to_string();
        log.execute(cmd).unwrap();
        assert_eq!(log.memtable.len(), 1);
        assert!(log.memtable.contains_key(&key));
    }

    #[test]
    fn execute_get_existing_key() {
        let (_dir, mut log) = temp_log();
        let set = Command::set("a", "1");
        let key = set.key().unwrap().to_string();
        log.execute(set).unwrap();
        log.execute(Command::get(key)).unwrap();
    }

    #[test]
    fn execute_get_missing_key() {
        let (_dir, mut log) = temp_log();
        log.execute(Command::get("a")).unwrap();
    }

    #[test]
    fn execute_delete_existing_key_removes_from_memtable() {
        let (_dir, mut log) = temp_log();
        let set = Command::set("a", "1");
        let key = set.key().unwrap().to_string();
        log.execute(set).unwrap();
        log.execute(Command::delete(key)).unwrap();
        assert!(log.memtable.is_empty());
    }

    #[test]
    fn execute_delete_missing_key() {
        let (_dir, mut log) = temp_log();
        log.execute(Command::delete("a")).unwrap();
        assert!(log.memtable.is_empty());
    }

    #[test]
    fn execute_delete_persists_tombstone() {
        let dir = tempfile::tempdir().unwrap();
        let memtable_path = dir.path().join("test.log");
        let sstables_path = dir.path().join("sstables");
        {
            let mut log = Log::new(dir.path(), &memtable_path, &sstables_path, true).unwrap();
            let set = Command::set("a", "1");
            let key = set.key().unwrap().to_string();
            let delete = Command::delete(key);
            log.execute(set).unwrap();
            log.execute(delete).unwrap();
        }
        let log = Log::new(dir.path(), &memtable_path, &sstables_path, false).unwrap();
        assert!(log.memtable.is_empty());
    }

    #[test]
    fn execute_quit_is_noop() {
        let (_dir, mut log) = temp_log();
        log.execute(Command::Quit).unwrap();
        assert!(log.memtable.is_empty());
    }

    #[test]
    fn execute_help_is_noop() {
        let (_dir, mut log) = temp_log();
        log.execute(Command::Help).unwrap();
        assert!(log.memtable.is_empty());
    }

    #[test]
    fn execute_set_overwrite_updates_value() {
        let dir = tempfile::tempdir().unwrap();
        let memtable_path = dir.path().join("test.log");
        let sstables_path = dir.path().join("sstables");
        {
            let mut log = Log::new(dir.path(), &memtable_path, &sstables_path, true).unwrap();
            let first = Command::set("a", "1");
            let key = first.key().unwrap().to_string();
            let second = Command::set(key.clone(), "2");
            log.execute(first).unwrap();
            log.execute(second).unwrap();
        }
        let mut log = Log::new(dir.path(), &memtable_path, &sstables_path, false).unwrap();
        assert_eq!(log.memtable.len(), 1);
        log.execute(Command::get("a")).unwrap();
    }

    #[test]
    fn execute_get_finds_key_in_sstable_after_flush() {
        let (_dir, mut log) = temp_log();
        let set = Command::set("a", "1");
        let key = set.key().unwrap().to_string();
        log.execute(set).unwrap();
        log.flush().unwrap();
        assert!(log.contains(&key).unwrap());
        log.execute(Command::get(key)).unwrap();
    }
}
