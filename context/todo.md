# Todo

## Finish buf_merge in src/index.rs

- [ ] Remove line 95 (`*self = Self::from_buf(buf)?;`) — index is already rebuilt in the write loop above it, this line tries to re-read from a stale file handle and will fail
- [ ] Stale file handle — after rename, the `buf` passed into `buf_merge` still points at the old deleted file. Caller in `run.rs` needs to reopen `data.log` after merge. Options:
  - Return a new file handle from `buf_merge`
  - Have `buf_merge` take ownership and return a reopened handle
  - Reopen in `run.rs` after calling merge

## Remaining Phase 2

- [ ] Wire size-based merge triggering into `run.rs` — check `std::fs::metadata(DATA_PATH)?.len()` after writes, trigger merge when threshold exceeded
- [ ] Handle partial/corrupt entries — truncated writes at end of log should be skipped gracefully
- [ ] Crash recovery — what happens if the process dies mid-write?
  - Study: `fsync` / `File::sync_all()` — forces OS to flush buffers to disk
  - Study: write-ahead logging (WAL) — write intent before applying

## Study list

- ~~`std::io::Seek`, `SeekFrom`, `stream_position()`~~ — learned in Phase 1
- ~~Bitcask paper~~ — read, using as model for Phase 2
- `std::fs::File::sync_all()` — flush to disk for durability
- CRC32 checksums (`crc32fast` crate) — for detecting corrupt entries in Phase 3
- `BTreeMap` — sorted in-memory structure needed for Phase 4 memtable
- `std::io::BufWriter` — batching writes for better performance
