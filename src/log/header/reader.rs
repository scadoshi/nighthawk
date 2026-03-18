use std::io::{Read, Seek, SeekFrom};
use wincode::{SchemaRead, config::DefaultConfig};
use super::deserializer::HeaderDeserializer;

/// Read entries with the on-disk header format:
/// `[magic: 2B][crc32: 4B][entry_len: 4B][wincode-serialized Entry]`
pub(crate) trait HeaderReader<T>
where
    T: for<'de> SchemaRead<'de, DefaultConfig, Dst = T>,
{
    /// Reads the next valid entry from the current cursor position.
    /// Scans byte-by-byte on corruption to find the next valid magic + checksum match.
    fn header_read_next(&mut self) -> anyhow::Result<Option<T>>;
    /// Returns `true` if the file contains at least one valid entry, without moving the cursor.
    fn header_has_at_least_one(&mut self) -> anyhow::Result<bool>;
}

impl<R, T> HeaderReader<T> for R
where
    R: Read + Seek,
    T: for<'de> SchemaRead<'de, DefaultConfig, Dst = T>,
{
    fn header_read_next(&mut self) -> anyhow::Result<Option<T>> {
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
            match HeaderDeserializer::deserialize(&bytes[p..]) {
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

    fn header_has_at_least_one(&mut self) -> anyhow::Result<bool> {
        let pos = self.stream_position()?;
        self.seek(SeekFrom::Start(0))?;
        let value: Option<T> = self.header_read_next()?;
        self.seek(SeekFrom::Start(pos))?;
        Ok(value.is_some())
    }
}
