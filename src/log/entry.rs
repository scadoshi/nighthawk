use wincode::{SchemaRead, SchemaWrite};

/// A key-value operation serialized to the log file.
#[derive(Debug, SchemaRead, SchemaWrite)]
pub enum Entry {
    /// Stores a value for the given key.
    Set { k: String, v: String },
    /// Tombstone — marks a key as deleted.
    Delete { k: String },
}

impl Entry {
    /// Returns the key for any entry variant.
    pub fn k(&self) -> &str {
        match self {
            Self::Set { k, .. } => k.as_str(),
            Self::Delete { k } => k.as_str(),
        }
    }

    /// Returns the value if this is a `Set` entry, `None` for `Delete`.
    pub fn v(&self) -> Option<&str> {
        match self {
            Self::Set { v, .. } => Some(v.as_str()),
            Self::Delete { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_set_k_returns_key() {
        let k = "k".to_string();
        let entry = Entry::Set {
            k: k.clone(),
            v: "v".to_string(),
        };
        assert_eq!(entry.k(), k.as_str());
    }

    #[test]
    fn entry_delete_k_returns_key() {
        let k = "k".to_string();
        let entry = Entry::Delete { k: k.clone() };
        assert_eq!(entry.k(), k);
    }

    #[test]
    fn entry_set_v_returns_value() {
        let v = "v".to_string();
        let entry = Entry::Set {
            k: "k".to_string(),
            v: v.clone(),
        };
        assert_eq!(entry.v(), Some(v.as_str()));
    }

    #[test]
    fn entry_delete_v_returns_none() {
        let entry = Entry::Delete { k: "k".to_string() };
        assert_eq!(entry.v(), None);
    }
}
