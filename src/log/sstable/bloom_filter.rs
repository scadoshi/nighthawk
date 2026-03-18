use std::{
    io::{Read, Seek, SeekFrom},
    ops::{Deref, DerefMut},
};

/// Probabilistic membership filter backed by a fixed-size bit array.
///
/// Uses double hashing (two xxh3 seeds) to derive `k = 7` bit positions per key.
/// A key is definitely absent if any of its positions is unset; it may be present
/// if all positions are set (false positives are possible, false negatives are not).
///
/// The filter is serialized as a footer at the end of each SSTable file:
/// `[filter_bytes][bit_count: u32 le]`.
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

impl DerefMut for BloomFilter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Returns an iterator over the `k = 7` bit positions for `key` within a filter of `bit_count` bits.
///
/// Uses enhanced double hashing: `pos_i = (h1 + i * h2) % bit_count`, where `h1` and `h2`
/// are xxh3-64 hashes of the key with seeds 0 and 1 respectively.
fn positions(key: &[u8], bit_count: usize) -> impl Iterator<Item = usize> {
    let h1 = xxh3::hash64_with_seed(key, 0);
    let h2 = xxh3::hash64_with_seed(key, 1);
    (0..7).map(move |i| (h1.wrapping_add((i as u64).wrapping_mul(h2)) % bit_count as u64) as usize)
}

impl BloomFilter {
    /// Creates a new empty filter sized for `bit_count` bits (rounded up to the nearest byte).
    ///
    /// For `k = 7` hash functions, `10 bits per key` gives a false-positive rate of ~0.8%.
    pub fn new(bit_count: usize) -> Self {
        let byte_count = bit_count.div_ceil(8);
        Self {
            bit_count,
            inner: vec![0; byte_count],
        }
    }

    /// Records `key` in the filter by setting all 7 of its bit positions.
    pub fn insert(&mut self, key: &[u8]) {
        for pos in positions(key, self.bit_count) {
            self[pos / 8] |= 1 << (pos % 8);
        }
    }

    /// Returns `true` if `key` **may** be in the set, `false` if it is **definitely absent**.
    ///
    /// A `true` result can be a false positive; a `false` result is always correct.
    pub fn may_contain(&self, key: &[u8]) -> bool {
        positions(key, self.bit_count).all(|pos| self[pos / 8] & 1 << (pos % 8) != 0)
    }
}

/// Extension trait for reading a [`BloomFilter`] from the footer of an SSTable file.
///
/// The footer layout (written from the end of the file backward) is:
/// - 4 bytes: `bit_count` as `u32` little-endian
/// - `bit_count.div_ceil(8)` bytes: the filter bit array
pub trait BloomFilterReader: Read + Seek {
    /// Reads the bloom filter footer from the current file.
    ///
    /// Returns `None` if the file is too small to hold a valid footer.
    /// On success resets the cursor to the start of the file.
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
