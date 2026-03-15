use wincode::{SchemaRead, SchemaWrite};

/// A key-value operation serialized to the log file.
#[derive(Debug, SchemaRead, SchemaWrite, Clone)]
pub enum Entry {
    /// Stores a value for the given key.
    Set { key: String, value: String },
    /// Tombstone — marks a key as deleted.
    Delete { key: String },
}

impl Entry {
    /// Returns the key for any entry variant.
    pub fn key(&self) -> &str {
        match self {
            Self::Set { key, .. } => key.as_str(),
            Self::Delete { key } => key.as_str(),
        }
    }

    /// Returns the value if this is a `Set` entry, `None` for `Delete`.
    pub fn value(&self) -> Option<&str> {
        match self {
            Self::Set { value, .. } => Some(value.as_str()),
            Self::Delete { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_set_key_returns_key() {
        let key = "k".to_string();
        let entry = Entry::Set {
            key: key.clone(),
            value: "v".to_string(),
        };
        assert_eq!(entry.key(), key.as_str());
    }

    #[test]
    fn entry_delete_key_returns_key() {
        let key = "k".to_string();
        let entry = Entry::Delete { key: key.clone() };
        assert_eq!(entry.key(), key);
    }

    #[test]
    fn entry_set_value_returns_value() {
        let value = "v".to_string();
        let entry = Entry::Set {
            key: "k".to_string(),
            value: value.clone(),
        };
        assert_eq!(entry.value(), Some(value.as_str()));
    }

    #[test]
    fn entry_delete_value_returns_none() {
        let entry = Entry::Delete { key: "k".to_string() };
        assert_eq!(entry.value(), None);
    }
}
