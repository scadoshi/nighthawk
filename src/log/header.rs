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
    /// Not enough bytes remaining for a complete header.
    #[error("incomplete header: not enough bytes")]
    HeaderNotFound,
    /// Magic bytes don't match the expected constant.
    #[error("missing magic bytes at entry boundary")]
    MagicBytesNotFound,
    /// CRC32 of the entry data doesn't match the stored checksum.
    #[error("checksum mismatch: entry data corrupted")]
    ChecksumMismatch,
    /// Entry data is present but wincode deserialization failed.
    #[error("failed to deserialize entry payload")]
    EntryParseError,
}

/// Parses a header + entry from a byte slice. Returns the entry and total bytes consumed.
/// Pure function — no I/O.
fn parse_entry(bytes: &[u8]) -> Result<(Entry, usize), CorruptionType> {
    if bytes.len() < HEADER_LEN as usize {
        return Err(CorruptionType::HeaderNotFound);
    }
    let mut offset: usize = 0;
    // magic
    let magic_bytes: [u8; 2] = bytes[offset..offset + 2].try_into().unwrap();
    let magic = u16::from_le_bytes(magic_bytes);
    if magic != MAGIC {
        return Err(CorruptionType::MagicBytesNotFound);
    }
    offset += 2;
    // checksum
    let checksum_bytes: [u8; 4] = bytes[offset..offset + 4].try_into().unwrap();
    let checksum = u32::from_le_bytes(checksum_bytes);
    offset += 4;
    // len
    let len_bytes: [u8; 4] = bytes[offset..offset + 4].try_into().unwrap();
    let len = u32::from_le_bytes(len_bytes);
    offset += 4;
    // entry
    if bytes.len() < HEADER_LEN as usize + len as usize {
        return Err(CorruptionType::EntryParseError);
    }
    let entry_bytes = &bytes[offset..offset + len as usize];
    if checksum != crc32fast::hash(entry_bytes) {
        return Err(CorruptionType::ChecksumMismatch);
    }
    let entry: Entry =
        wincode::deserialize(entry_bytes).map_err(|_| CorruptionType::EntryParseError)?;
    Ok((entry, HEADER_LEN as usize + len as usize))
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
        let bytes = wincode::serialize(&entry)?;
        self.seek(SeekFrom::End(0))?;
        let wrote_at = self.stream_position()?;
        // magic
        self.write_all(&MAGIC.to_le_bytes())?;
        // checksum
        let checksum = crc32fast::hash(bytes.as_slice());
        self.write_all(&checksum.to_le_bytes())?;
        // len
        let len = u32::try_from(bytes.len())?;
        self.write_all(&len.to_le_bytes())?;
        // entry
        self.write_all(bytes.as_slice())?;
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

        let mut offset: usize = 0;
        while offset < bytes.len() {
            match parse_entry(&bytes[offset..]) {
                Ok((entry, consumed)) => {
                    self.seek(SeekFrom::Start(start + offset as u64 + consumed as u64))?;
                    return Ok(Some(entry));
                }
                // Corruption recovery: advance one byte and retry.
                Err(_) => offset += 1,
            }
        }
        Ok(None)
    }
}
