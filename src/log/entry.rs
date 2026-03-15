use wincode::{SchemaRead, SchemaWrite};

/// A key-value operation serialized to the log file.
#[derive(Debug, SchemaRead, SchemaWrite, Clone, PartialEq)]
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

    pub fn set(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Set {
            key: key.into(),
            value: value.into(),
        }
    }

    pub fn delete(key: impl Into<String>) -> Self {
        Self::Delete { key: key.into() }
    }
}

impl From<&Entry> for Entry {
    fn from(value: &Entry) -> Self {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_set_key_returns_key() {
        let key = "k".to_string();
        let set = Entry::set(key.clone(), "v");
        assert_eq!(set.key(), key.as_str());
    }

    #[test]
    fn entry_delete_key_returns_key() {
        let key = "k".to_string();
        let delete = Entry::delete(key.clone());
        assert_eq!(delete.key(), key);
    }

    #[test]
    fn entry_set_value_returns_value() {
        let value = "v".to_string();
        let set = Entry::set("k", value.clone());
        assert_eq!(set.value(), Some(value.as_str()));
    }

    #[test]
    fn entry_delete_value_returns_none() {
        let delete = Entry::delete("k");
        assert_eq!(delete.value(), None);
    }
}
