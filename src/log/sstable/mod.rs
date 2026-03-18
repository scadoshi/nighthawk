pub mod bloom_filter;
pub mod compact;

use self::bloom_filter::{BloomFilter, BloomFilterReader};
use crate::log::{entry::Entry, header::reader::HeaderReader};
use std::{fs::File, io::Seek, path::Path};

/// An immutable, on-disk sorted table of key-value entries produced by flushing the memtable.
///
/// Each file contains wincode-encoded [`Entry`] records prefixed with the standard on-disk header,
/// followed by a [`BloomFilter`] footer used to skip files that cannot contain a queried key.
#[derive(Debug)]
pub struct SSTable {
    bloom_filter: BloomFilter,
    bloom_filter_pos: u64,
    file: File,
}

impl SSTable {
    /// Opens an SSTable at `path`, reads its bloom-filter footer, and positions the cursor at
    /// the first entry. Returns `None` if the file is too small or contains no valid entries.
    pub fn from_path(path: impl AsRef<Path>) -> anyhow::Result<Option<Self>> {
        let mut file = File::open(path.as_ref())?;
        let Some(bloom_filter) = file.read_bloom_filter()? else {
            return Ok(None);
        };
        let bloom_filter_pos = file.metadata()?.len() - bloom_filter.len() as u64 - 4;
        if !HeaderReader::<Entry>::header_has_at_least_one(&mut file)? {
            return Ok(None);
        }
        Ok(Some(Self {
            bloom_filter,
            bloom_filter_pos,
            file,
        }))
    }

    /// Returns a reference to the bloom filter loaded from this file's footer.
    pub fn bloom_filter(&self) -> &BloomFilter {
        &self.bloom_filter
    }

    /// Reads and returns the next entry, or `None` once the bloom-filter footer is reached.
    pub fn read_next_entry(&mut self) -> anyhow::Result<Option<Entry>> {
        if self.file.stream_position()? > self.bloom_filter_pos {
            return Ok(None);
        }
        self.file.header_read_next()
    }
}
