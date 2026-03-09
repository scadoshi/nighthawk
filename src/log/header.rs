use crate::log::entry::Entry;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};
use thiserror::Error;

pub const HEADER_LEN: u64 = 10;
pub const MAGIC: u16 = 0x4E48; // NH

#[derive(Debug, Error)]
pub enum CorruptionType {
    #[error("Header not found")]
    HeaderNotFound,
    #[error("Magic bytes not found")]
    MagicNotFound,
    #[error("Checksum does not match")]
    ChecksumNotMatch,
    #[error("Failed to parse `Entry`")]
    EntryParseError,
}

fn parse_entry(bytes: &[u8]) -> Result<(Entry, usize), CorruptionType> {
    if bytes.len() < HEADER_LEN as usize {
        return Err(CorruptionType::HeaderNotFound);
    }
    let mut offset: usize = 0;
    // magic
    let magic_bytes: [u8; 2] = bytes[offset..offset + 2].try_into().unwrap();
    let magic = u16::from_le_bytes(magic_bytes);
    if magic != MAGIC {
        return Err(CorruptionType::MagicNotFound);
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
        return Err(CorruptionType::ChecksumNotMatch);
    }
    let entry: Entry =
        wincode::deserialize(entry_bytes).map_err(|_| CorruptionType::EntryParseError)?;
    Ok((entry, HEADER_LEN as usize + len as usize))
}

pub trait Header {
    fn write_entry_with_header(&mut self, entry: &Entry) -> anyhow::Result<u64>;
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
                Err(_) => offset += 1,
            }
        }
        Ok(None)
    }
}
