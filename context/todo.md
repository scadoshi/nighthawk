# Todo

## Finish Phase 1

- [ ] Tombstone entries for delete — append a marker to the log so deletes persist across restarts
  - Design choice: use an enum like `Entry::Set(k, v)` / `Entry::Delete(k)` instead of raw tuples
  - On startup scan, if a tombstone is found, remove the key from the index

## Phase 2 — Durability and compaction

- [ ] Log compaction — scan the log, keep only the latest entry per key, rewrite to a new file
  - Study: how Bitcask handles merge/compaction
- [ ] Crash recovery — what happens if the process dies mid-write?
  - Study: `fsync` / `File::sync_all()` — forces OS to flush buffers to disk
  - Study: write-ahead logging (WAL) — write intent before applying
- [ ] Handle partial/corrupt entries — truncated writes at end of log should be skipped gracefully

## Study list

- `std::io::Seek`, `SeekFrom`, `stream_position()` — already using, keep practicing
- `std::fs::File::sync_all()` — flush to disk for durability
- Bitcask paper — the design nighthawk Phase 1-2 is based on
- CRC32 checksums (`crc32fast` crate) — for detecting corrupt entries in Phase 3
- `BTreeMap` — sorted in-memory structure needed for Phase 4 memtable
- `std::io::BufWriter` — batching writes for better performance
