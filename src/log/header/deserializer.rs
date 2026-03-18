use super::{HEADER_LEN, MAGIC};
use thiserror::Error;
use wincode::{SchemaRead, config::DefaultConfig};

/// Reasons an entry failed to parse from a byte slice.
#[derive(Debug, Error)]
pub enum CorruptionType {
    /// Slice is too short to contain a full header.
    #[error("slice too short for header and entry")]
    NotEnoughBytes,
    /// Magic bytes don't match the expected constant.
    #[error("missing magic bytes at entry boundary")]
    MagicBytesMismatch,
    /// CRC32 of the entry data doesn't match the stored checksum.
    #[error("checksum mismatch: entry data corrupted")]
    ChecksumMismatch,
    /// Entry data is present and checksums match, but wincode deserialization failed.
    #[error("failed to deserialize entry payload")]
    ParseError,
}

/// Stateless deserializer that validates and strips the on-disk header from a byte slice.
pub struct Deserializer;

/// Alias for [`Deserializer`]; prefer this name at call sites for symmetry with [`HeaderSerializer`].
///
/// [`HeaderSerializer`]: crate::log::header::serializer::HeaderSerializer
pub type HeaderDeserializer = Deserializer;

impl Deserializer {
    /// Parses a header-prefixed byte slice and returns the decoded value and total bytes consumed.
    ///
    /// Validates magic bytes and CRC32 before attempting deserialization. Returns the number of
    /// bytes consumed (`HEADER_LEN + entry_len`) so the caller can advance its read cursor.
    pub fn deserialize<'de, T>(value: &'de [u8]) -> Result<(T, usize), CorruptionType>
    where
        T: SchemaRead<'de, DefaultConfig, Dst = T>,
    {
        if value.len() <= HEADER_LEN as usize {
            return Err(CorruptionType::NotEnoughBytes);
        }
        let mut p: usize = 0;
        // magic
        let magic_bytes: [u8; 2] = value[p..p + 2].try_into().unwrap();
        let magic = u16::from_le_bytes(magic_bytes);
        if magic != MAGIC {
            return Err(CorruptionType::MagicBytesMismatch);
        }
        p += 2;
        // checksum
        let checksum_bytes: [u8; 4] = value[p..p + 4].try_into().unwrap();
        let checksum = u32::from_le_bytes(checksum_bytes);
        p += 4;
        // len
        let len_bytes: [u8; 4] = value[p..p + 4].try_into().unwrap();
        let len = u32::from_le_bytes(len_bytes);
        p += 4;
        // entry
        if value.len() < HEADER_LEN as usize + len as usize {
            return Err(CorruptionType::NotEnoughBytes);
        }
        let entry_bytes = &value[p..p + len as usize];
        if checksum != crc32fast::hash(entry_bytes) {
            return Err(CorruptionType::ChecksumMismatch);
        }
        let value: T = wincode::deserialize(entry_bytes).map_err(|_| CorruptionType::ParseError)?;
        Ok((value, HEADER_LEN as usize + len as usize))
    }
}
