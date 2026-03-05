# Todo

## Phase 2 — Durability and compaction

- [ ] Handle partial/corrupt entries — truncated writes at end of log should be skipped gracefully
  - Most likely real bug to hit right now
- [ ] Log compaction — scan the log, keep only the latest entry per key, rewrite to a new file
  - Study: how Bitcask handles merge/compaction
- [ ] Crash recovery — what happens if the process dies mid-write?
  - Study: `fsync` / `File::sync_all()` — forces OS to flush buffers to disk
  - Study: write-ahead logging (WAL) — write intent before applying

## Study list

- `std::fs::File::sync_all()` — flush to disk for durability
- Bitcask paper — the design nighthawk Phase 1-2 is based on
- CRC32 checksums (`crc32fast` crate) — for detecting corrupt entries in Phase 3
- `BTreeMap` — sorted in-memory structure needed for Phase 4 memtable
- `std::io::BufWriter` — batching writes for better performance
