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
