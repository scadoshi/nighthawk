use crate::log::entry::Entry;
use std::io::{Read, Seek, SeekFrom, Write};
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

/// Write entries with the on-disk header format:
/// `[magic: 2B][crc32: 4B][entry_len: 4B][wincode-serialized Entry]`
pub trait HeaderWriter {
    /// Appends an entry with header to end of file.
    fn write_entry_with_header(&mut self, entry: &Entry) -> anyhow::Result<()>;
}

impl<W: Write + Seek> HeaderWriter for W {
    fn write_entry_with_header(&mut self, entry: &Entry) -> anyhow::Result<()> {
        self.seek(SeekFrom::End(0))?;
        self.write_all(entry.try_into_bytes_with_header()?.as_slice())?;
        Ok(())
    }
}

/// Read entries with the on-disk header format:
/// `[magic: 2B][crc32: 4B][entry_len: 4B][wincode-serialized Entry]`
pub trait HeaderReader {
    /// Reads the next valid entry from the current cursor position.
    /// Scans byte-by-byte on corruption to find the next valid magic + checksum match.
    fn read_next_entry_with_header(&mut self) -> anyhow::Result<Option<Entry>>;
    fn contains_entry_with_header(&mut self) -> anyhow::Result<bool>;
}

impl<R: Read + Seek> HeaderReader for R {
    fn read_next_entry_with_header(&mut self) -> anyhow::Result<Option<Entry>> {
        let pos = self.stream_position()?;
        let buf_len = {
            self.seek(SeekFrom::End(0))?;
            let buf_len = self.stream_position()?;
            self.seek(SeekFrom::Start(pos))?;
            buf_len
        };
        if pos >= buf_len {
            return Ok(None);
        }
        let mut bytes = Vec::<u8>::new();
        self.read_to_end(&mut bytes)?;
        let mut p: usize = 0;
        while p < bytes.len() {
            match bytes[p..].try_into_entry_with_len() {
                Ok((entry, len)) => {
                    self.seek(SeekFrom::Start(pos + p as u64 + len as u64))?;
                    return Ok(Some(entry));
                }
                // Corruption recovery: advance one byte and retry.
                Err(_) => p += 1,
            }
        }
        Ok(None)
    }

    fn contains_entry_with_header(&mut self) -> anyhow::Result<bool> {
        let pos = self.stream_position()?;
        self.seek(SeekFrom::Start(0))?;
        let has_entry = self.read_next_entry_with_header()?.is_some();
        self.seek(SeekFrom::Start(pos))?;
        Ok(has_entry)
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
    /// Returns the parsed entry and total length of header + entry, or a [`CorruptionType`] on failure.
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
        assert!(matches!(
            0_u32.to_le_bytes().try_into_entry_with_len(),
            Err(CorruptionType::NotEnoughBytes)
        ));
    }

    #[test]
    fn try_into_entry_with_header_set_ok() {
        let set = Entry::set("a", "1");
        let set_bytes = wincode::serialize(&set).unwrap();
        let checksum = crc32fast::hash(&set_bytes);
        let len = set_bytes.len() as u32;
        let bytes = entry_bytes_from_parts(MAGIC, checksum, len, set_bytes.as_slice());
        let result = bytes.try_into_entry_with_len();
        assert!(matches!(result, Ok((Entry::Set { .. }, _))));
        let (resulting_entry, consumed) = result.unwrap();
        assert_eq!(resulting_entry.key(), set.key());
        assert_eq!(resulting_entry.value(), set.value());
        assert_eq!(consumed, len as usize + HEADER_LEN as usize);
    }

    #[test]
    fn try_into_entry_with_header_set_err_magic_bytes_mismatch() {
        let set = Entry::set("a", "1");
        let set_bytes = wincode::serialize(&set).unwrap();
        let checksum = crc32fast::hash(&set_bytes);
        let len = set_bytes.len() as u32;
        let bytes = entry_bytes_from_parts(0_u16, checksum, len, set_bytes.as_slice());
        assert!(matches!(
            bytes.try_into_entry_with_len(),
            Err(CorruptionType::MagicBytesMismatch)
        ));
    }

    #[test]
    fn try_into_entry_with_header_set_err_checksum_mismatch() {
        let set = Entry::set("a", "1");
        let set_bytes = wincode::serialize(&set).unwrap();
        let len = set_bytes.len() as u32;
        let bytes = entry_bytes_from_parts(MAGIC, 0_u32, len, set_bytes.as_slice());
        assert!(matches!(
            bytes.try_into_entry_with_len(),
            Err(CorruptionType::ChecksumMismatch)
        ));
    }

    #[test]
    fn try_into_entry_with_header_set_err_entry_parse_error() {
        // Garbage payload the same length as the real entry so it passes the length check.
        let set = Entry::set("a", "1");
        let real_len = wincode::serialize(&set).unwrap().len();
        let garbage = vec![0xFF; real_len];
        let checksum = crc32fast::hash(&garbage);
        let bytes = entry_bytes_from_parts(MAGIC, checksum, real_len as u32, &garbage);
        assert!(matches!(
            bytes.try_into_entry_with_len(),
            Err(CorruptionType::EntryParseError)
        ));
    }

    #[test]
    fn try_into_entry_with_header_delete_ok() {
        let delete = Entry::delete("a");
        let delete_bytes = wincode::serialize(&delete).unwrap();
        let checksum = crc32fast::hash(&delete_bytes);
        let len = delete_bytes.len() as u32;
        let bytes = entry_bytes_from_parts(MAGIC, checksum, len, delete_bytes.as_slice());
        let result = bytes.try_into_entry_with_len();
        assert!(matches!(result, Ok((Entry::Delete { .. }, _))));
        let (resulting_entry, consumed) = result.unwrap();
        assert_eq!(resulting_entry.key(), delete.key());
        assert_eq!(resulting_entry.value(), None);
        assert_eq!(consumed, len as usize + HEADER_LEN as usize);
    }

    #[test]
    fn try_into_entry_with_header_delete_err_magic_bytes_mismatch() {
        let delete = Entry::delete("a");
        let delete_bytes = wincode::serialize(&delete).unwrap();
        let checksum = crc32fast::hash(&delete_bytes);
        let len = delete_bytes.len() as u32;
        let bytes = entry_bytes_from_parts(0_u16, checksum, len, delete_bytes.as_slice());
        assert!(matches!(
            bytes.try_into_entry_with_len(),
            Err(CorruptionType::MagicBytesMismatch)
        ));
    }

    #[test]
    fn try_into_entry_with_header_delete_err_checksum_mismatch() {
        let delete = Entry::delete("a");
        let delete_bytes = wincode::serialize(&delete).unwrap();
        let len = delete_bytes.len() as u32;
        let bytes = entry_bytes_from_parts(MAGIC, 0_u32, len, delete_bytes.as_slice());
        assert!(matches!(
            bytes.try_into_entry_with_len(),
            Err(CorruptionType::ChecksumMismatch)
        ));
    }

    #[test]
    fn try_into_entry_with_header_delete_err_entry_parse_error() {
        let delete = Entry::delete("a");
        let real_len = wincode::serialize(&delete).unwrap().len();
        let garbage = vec![0xFF; real_len];
        let checksum = crc32fast::hash(&garbage);
        let bytes = entry_bytes_from_parts(MAGIC, checksum, real_len as u32, &garbage);
        assert!(matches!(
            bytes.try_into_entry_with_len(),
            Err(CorruptionType::EntryParseError)
        ));
    }

    #[test]
    fn write_entry_with_header_set_ok() {
        let mut file = tempfile::tempfile().unwrap();
        let set = Entry::set("a", "1");
        file.write_entry_with_header(&set).unwrap();
    }

    #[test]
    fn write_entry_with_header_set_ok_then_read() {
        let mut file = tempfile::tempfile().unwrap();
        let set = Entry::set("a", "1");
        file.write_entry_with_header(&set).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        let result = file.read_next_entry_with_header().unwrap().unwrap();
        assert_eq!(result.key(), set.key());
        assert_eq!(result.value(), set.value());
    }

    #[test]
    fn write_entry_with_header_delete_ok() {
        let mut file = tempfile::tempfile().unwrap();
        let delete = Entry::delete("a");
        file.write_entry_with_header(&delete).unwrap();
    }

    #[test]
    fn write_entry_with_header_delete_ok_then_read() {
        let mut file = tempfile::tempfile().unwrap();
        let delete = Entry::delete("a");
        file.write_entry_with_header(&delete).unwrap();
        file.seek(SeekFrom::Start(0)).unwrap();
        let result = file.read_next_entry_with_header().unwrap().unwrap();
        assert_eq!(result.key(), delete.key());
    }

    #[test]
    fn read_next_entry_with_header_recovers_past_corruption() {
        let mut file = tempfile::tempfile().unwrap();
        let set = Entry::set("a", "1");
        // Write 7 bytes of garbage before a valid entry.
        let garbage = [0xFF; 7];
        file.write_all(&garbage).unwrap();
        file.write_entry_with_header(&set).unwrap();
        // Seek to start — reader should skip garbage and find the entry.
        file.seek(SeekFrom::Start(0)).unwrap();
        let result = file.read_next_entry_with_header().unwrap().unwrap();
        assert_eq!(result.key(), set.key());
        assert_eq!(result.value(), set.value());
    }
}
