use super::{entry::Entry, header::HeaderReader};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    ops::Deref,
    path::Path,
};

#[derive(Debug)]
pub struct BloomFilter {
    bit_count: usize,
    inner: Vec<u8>,
}

impl Deref for BloomFilter {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl BloomFilter {
    pub fn blank(bit_count: usize) -> Self {
        let byte_count = bit_count.div_ceil(8);
        Self {
            bit_count,
            inner: vec![0; byte_count],
        }
    }

    pub fn bit_count(&self) -> usize {
        self.bit_count
    }
}

trait BloomFilterReader: Read + Seek {
    fn read_bloom_filter(&mut self) -> anyhow::Result<Option<BloomFilter>>;
}

impl<R: Read + Seek> BloomFilterReader for R {
    fn read_bloom_filter(&mut self) -> anyhow::Result<Option<BloomFilter>> {
        self.seek(SeekFrom::End(0))?;
        let size = self.stream_position()?;
        if size < 4 {
            return Ok(None);
        }
        self.seek(SeekFrom::End(-4))?;
        let (bit_count, byte_count) = {
            let mut bytes = Vec::<u8>::new();
            self.read_to_end(&mut bytes)?;
            let bit_count = u32::from_le_bytes(bytes.as_slice().try_into().unwrap()) as usize;
            let byte_count = bit_count.div_ceil(8);
            (bit_count, byte_count)
        };
        if size < 4 + byte_count as u64 {
            return Ok(None);
        }
        self.seek(SeekFrom::End(-(byte_count as i64) - 4))?;
        let inner: Vec<u8> = {
            let mut bytes = Vec::<u8>::new();
            self.read_to_end(&mut bytes)?;
            bytes.into_iter().take(byte_count).collect()
        };
        self.seek(SeekFrom::Start(0))?;
        Ok(Some(BloomFilter { bit_count, inner }))
    }
}

#[derive(Debug)]
pub struct SSTable {
    bloom_filter: BloomFilter,
    bloom_filter_pos: u64,
    file: File,
}

impl SSTable {
    pub fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Option<Self>> {
        let mut file = File::open(path.as_ref())?;
        let Some(bloom_filter) = file.read_bloom_filter()? else {
            return Ok(None);
        };
        let bloom_filter_pos = file.metadata()?.len() - bloom_filter.len() as u64 - 4;
        if !file.contains_entry_with_header()? {
            return Ok(None);
        }
        Ok(Some(Self {
            bloom_filter,
            bloom_filter_pos,
            file,
        }))
    }

    pub fn bloom_filter(&self) -> &BloomFilter {
        &self.bloom_filter
    }

    pub fn bloom_filter_pos(&self) -> u64 {
        self.bloom_filter_pos
    }

    pub fn file(&self) -> &File {
        &self.file
    }

    pub fn read_next_entry(&mut self) -> anyhow::Result<Option<Entry>> {
        if self.file.stream_position()? > self.bloom_filter_pos {
            return Ok(None);
        }
        self.file.read_next_entry_with_header()
    }
}
