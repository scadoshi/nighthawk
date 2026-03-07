use wincode::{SchemaRead, SchemaWrite};

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
