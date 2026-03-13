use crate::log::entry::Entry;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};
use thiserror::Error;

/// Size of the entry header in bytes: magic (2) + crc32 (4) + entry_len (4).
pub const HEADER_LEN: u64 = 10;
/// Magic bytes written at the start of every entry, used to locate entry boundaries.
/// (String translation: "NH")
pub const MAGIC: u16 = 0x4E48;

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
    /// Entry data is present but wincode deserialization failed.
    #[error("failed to deserialize entry payload")]
    EntryParseError,
}

/// Read/write entries with the on-disk header format:
/// `[magic: 2B][crc32: 4B][entry_len: 4B][wincode-serialized Entry]`
pub trait Header {
    /// Appends an entry with header to end of file. Returns the byte offset where it was written.
    fn write_entry_with_header(&mut self, entry: &Entry) -> anyhow::Result<u64>;
    /// Reads the next valid entry from the current cursor position.
    /// Scans byte-by-byte on corruption to find the next valid magic + checksum match.
    fn read_next_entry_with_header(&mut self) -> anyhow::Result<Option<Entry>>;
}

impl Header for File {
    fn write_entry_with_header(&mut self, entry: &Entry) -> anyhow::Result<u64> {
        self.seek(SeekFrom::End(0))?;
        let wrote_at = self.stream_position()?;
        self.write_all(entry.try_into_bytes_with_header()?.as_slice())?;
        self.sync_all()?;
        Ok(wrote_at)
    }

    fn read_next_entry_with_header(&mut self) -> anyhow::Result<Option<Entry>> {
        let start = self.stream_position()?;
        if start >= self.metadata()?.len() {
            return Ok(None);
        }
        let mut bytes = Vec::<u8>::new();
        self.read_to_end(&mut bytes)?;
        let mut p: usize = 0;
        while p < bytes.len() {
            match bytes[p..].try_into_entry_with_len() {
                Ok((entry, len)) => {
                    self.seek(SeekFrom::Start(start + p as u64 + len as u64))?;
                    return Ok(Some(entry));
                }
                // Corruption recovery: advance one byte and retry.
                Err(_) => p += 1,
            }
        }
        Ok(None)
    }
}

/// Create binary encoded entries with the on-disk header format:
/// `[magic: 2B][crc32: 4B][entry_len: 4B][wincode-serialized Entry]`
pub trait EntryWithHeader {
    /// Converts a reference to an entry to a binary encoded entry with a proper header
    fn try_into_bytes_with_header(&self) -> anyhow::Result<Vec<u8>>;
}

impl EntryWithHeader for Entry {
    fn try_into_bytes_with_header(&self) -> anyhow::Result<Vec<u8>> {
        let entry_bytes = wincode::serialize(&self)?;
        let checksum = crc32fast::hash(&entry_bytes);
        let len = entry_bytes.len() as u32;
        let mut bytes = Vec::<u8>::new();
        bytes.extend(MAGIC.to_le_bytes());
        bytes.extend(checksum.to_le_bytes());
        bytes.extend(len.to_le_bytes());
        bytes.extend(entry_bytes);
        Ok(bytes)
    }
}

/// Parses a header + entry from a byte slice.
trait TryIntoEntryWithLen {
    /// Returns the parsed entry and total bytes consumed, or a [`CorruptionType`] on failure.
    fn try_into_entry_with_len(&self) -> Result<(Entry, usize), CorruptionType>;
}

impl TryIntoEntryWithLen for [u8] {
    fn try_into_entry_with_len(&self) -> Result<(Entry, usize), CorruptionType> {
        if self.len() <= HEADER_LEN as usize {
            return Err(CorruptionType::NotEnoughBytes);
        }
        let mut p: usize = 0;
        // magic
        let magic_bytes: [u8; 2] = self[p..p + 2].try_into().unwrap();
        let magic = u16::from_le_bytes(magic_bytes);
        if magic != MAGIC {
            return Err(CorruptionType::MagicBytesMismatch);
        }
        p += 2;
        // checksum
        let checksum_bytes: [u8; 4] = self[p..p + 4].try_into().unwrap();
        let checksum = u32::from_le_bytes(checksum_bytes);
        p += 4;
        // len
        let len_bytes: [u8; 4] = self[p..p + 4].try_into().unwrap();
        let len = u32::from_le_bytes(len_bytes);
        p += 4;
        // entry
        if self.len() < HEADER_LEN as usize + len as usize {
            return Err(CorruptionType::NotEnoughBytes);
        }
        let entry_bytes = &self[p..p + len as usize];
        if checksum != crc32fast::hash(entry_bytes) {
            return Err(CorruptionType::ChecksumMismatch);
        }
        let entry: Entry =
            wincode::deserialize(entry_bytes).map_err(|_| CorruptionType::EntryParseError)?;
        Ok((entry, HEADER_LEN as usize + len as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_entry_set_with_expected_parts() -> (Entry, String, String) {
        let k = "k".to_string();
        let v = "v".to_string();
        let entry = Entry::Set {
            k: k.clone(),
            v: v.clone(),
        };
        (entry, k, v)
    }

    fn create_entry_delete_with_expected_parts() -> (Entry, String) {
        let k = "k".to_string();
        let entry = Entry::Delete { k: k.clone() };
        (entry, k)
    }

    fn entry_bytes_from_parts(magic: u16, checksum: u32, len: u32, entry_bytes: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::<u8>::new();
        bytes.extend(magic.to_le_bytes());
        bytes.extend(checksum.to_le_bytes());
        bytes.extend(len.to_le_bytes());
        bytes.extend(entry_bytes);
        bytes
    }

    #[test]
    fn try_into_entry_with_header_err_not_enough_bytes() {
        let result = 0_u32.to_le_bytes().try_into_entry_with_len();
        assert!(matches!(result, Err(CorruptionType::NotEnoughBytes)));
    }

    #[test]
    fn try_into_entry_with_header_set_ok() {
        let (entry, k, v) = create_entry_set_with_expected_parts();
        let entry_bytes = wincode::serialize(&entry).unwrap();
        let checksum = crc32fast::hash(&entry_bytes);
        let len = entry_bytes.len() as u32;
        let bytes = entry_bytes_from_parts(MAGIC, checksum, len, entry_bytes.as_slice());
        let result = bytes.try_into_entry_with_len();
        assert!(matches!(result, Ok((Entry::Set { .. }, _))));
        let (resulting_entry, consumed) = result.unwrap();
        assert_eq!(resulting_entry.k(), k.as_str());
        assert_eq!(resulting_entry.v(), Some(v.as_str()));
        assert_eq!(consumed, len as usize + HEADER_LEN as usize);
    }

    #[test]
    fn try_into_entry_with_header_set_err_magic_bytes_mismatch() {
        let (entry, _, _) = create_entry_set_with_expected_parts();
        let entry_bytes = wincode::serialize(&entry).unwrap();
        let checksum = crc32fast::hash(&entry_bytes);
        let len = entry_bytes.len() as u32;
        let bytes = entry_bytes_from_parts(0_u16, checksum, len, entry_bytes.as_slice());
        let result = bytes.try_into_entry_with_len();
        assert!(matches!(result, Err(CorruptionType::MagicBytesMismatch)));
    }

    #[test]
    fn try_into_entry_with_header_set_err_checksum_mismatch() {
        let (entry, _, _) = create_entry_set_with_expected_parts();
        let entry_bytes = wincode::serialize(&entry).unwrap();
        let incorrect_checksum = 0_u32;
        let len = entry_bytes.len() as u32;
        let bytes = entry_bytes_from_parts(MAGIC, incorrect_checksum, len, entry_bytes.as_slice());
        let result = bytes.try_into_entry_with_len();
        assert!(matches!(result, Err(CorruptionType::ChecksumMismatch)));
    }

    #[test]
    fn try_into_entry_with_header_set_err_entry_parse_error() {
        // Garbage payload the same length as the real entry so it passes the length check.
        let (entry, _, _) = create_entry_set_with_expected_parts();
        let real_len = wincode::serialize(&entry).unwrap().len();
        let garbage = vec![0xFF; real_len];
        let checksum = crc32fast::hash(&garbage);
        let bytes = entry_bytes_from_parts(MAGIC, checksum, real_len as u32, &garbage);
        let result = bytes.try_into_entry_with_len();
        assert!(matches!(result, Err(CorruptionType::EntryParseError)));
    }

    #[test]
    fn try_into_entry_with_header_delete_ok() {
        let (entry, k) = create_entry_delete_with_expected_parts();
        let entry_bytes = wincode::serialize(&entry).unwrap();
        let checksum = crc32fast::hash(&entry_bytes);
        let len = entry_bytes.len() as u32;
        let bytes = entry_bytes_from_parts(MAGIC, checksum, len, entry_bytes.as_slice());
        let result = bytes.try_into_entry_with_len();
        assert!(matches!(result, Ok((Entry::Delete { .. }, _))));
        let (resulting_entry, consumed) = result.unwrap();
        assert_eq!(resulting_entry.k(), k.as_str());
        assert_eq!(resulting_entry.v(), None);
        assert_eq!(consumed, len as usize + HEADER_LEN as usize);
    }

    #[test]
    fn try_into_entry_with_header_delete_err_magic_bytes_mismatch() {
        let (entry, _) = create_entry_delete_with_expected_parts();
        let entry_bytes = wincode::serialize(&entry).unwrap();
        let checksum = crc32fast::hash(&entry_bytes);
        let len = entry_bytes.len() as u32;
        let bytes = entry_bytes_from_parts(0_u16, checksum, len, entry_bytes.as_slice());
        let result = bytes.try_into_entry_with_len();
        assert!(matches!(result, Err(CorruptionType::MagicBytesMismatch)));
    }

    #[test]
    fn try_into_entry_with_header_delete_err_checksum_mismatch() {
        let (entry, _) = create_entry_delete_with_expected_parts();
        let entry_bytes = wincode::serialize(&entry).unwrap();
        let incorrect_checksum = 0_u32;
        let len = entry_bytes.len() as u32;
        let bytes = entry_bytes_from_parts(MAGIC, incorrect_checksum, len, entry_bytes.as_slice());
        let result = bytes.try_into_entry_with_len();
        assert!(matches!(result, Err(CorruptionType::ChecksumMismatch)));
    }

    #[test]
    fn try_into_entry_with_header_delete_err_entry_parse_error() {
        let (entry, _) = create_entry_delete_with_expected_parts();
        let real_len = wincode::serialize(&entry).unwrap().len();
        let garbage = vec![0xFF; real_len];
        let checksum = crc32fast::hash(&garbage);
        let bytes = entry_bytes_from_parts(MAGIC, checksum, real_len as u32, &garbage);
        let result = bytes.try_into_entry_with_len();
        assert!(matches!(result, Err(CorruptionType::EntryParseError)));
    }

    #[test]
    fn write_entry_with_header_set_ok() {
        let mut file = tempfile::tempfile().unwrap();
        let (entry, _, _) = create_entry_set_with_expected_parts();
        let result = file.write_entry_with_header(&entry);
        assert!(result.is_ok());
        let wrote_at = result.unwrap();
        assert_eq!(wrote_at, 0);
    }

    #[test]
    fn write_entry_with_header_set_ok_then_read() {
        let mut file = tempfile::tempfile().unwrap();
        let (entry, k, v) = create_entry_set_with_expected_parts();
        let wrote_at = file.write_entry_with_header(&entry).unwrap();
        assert_eq!(wrote_at, 0);
        file.seek(SeekFrom::Start(wrote_at)).unwrap();
        let result = file.read_next_entry_with_header().unwrap().unwrap();
        assert_eq!(result.k(), k);
        assert_eq!(result.v(), Some(v.as_str()));
    }

    #[test]
    fn write_entry_with_header_delete_ok() {
        let mut file = tempfile::tempfile().unwrap();
        let (entry, _) = create_entry_delete_with_expected_parts();
        let result = file.write_entry_with_header(&entry);
        assert!(result.is_ok());
        let wrote_at = result.unwrap();
        assert_eq!(wrote_at, 0);
    }

    #[test]
    fn write_entry_with_header_delete_ok_then_read() {
        let mut file = tempfile::tempfile().unwrap();
        let (entry, k) = create_entry_delete_with_expected_parts();
        let wrote_at = file.write_entry_with_header(&entry).unwrap();
        assert_eq!(wrote_at, 0);
        file.seek(SeekFrom::Start(wrote_at)).unwrap();
        let result = file.read_next_entry_with_header().unwrap().unwrap();
        assert_eq!(result.k(), k);
    }

    #[test]
    fn read_next_entry_with_header_recovers_past_corruption() {
        let mut file = tempfile::tempfile().unwrap();
        // Write 7 bytes of garbage before a valid entry.
        let garbage = [0xFF; 7];
        file.write_all(&garbage).unwrap();
        let (entry, k, v) = create_entry_set_with_expected_parts();
        file.write_entry_with_header(&entry).unwrap();
        // Seek to start — reader should skip garbage and find the entry.
        file.seek(SeekFrom::Start(0)).unwrap();
        let result = file.read_next_entry_with_header().unwrap().unwrap();
        assert_eq!(result.k(), k);
        assert_eq!(result.v(), Some(v.as_str()));
    }
}
