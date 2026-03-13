# Todo

## Phase 3 — Binary serialization (COMPLETE)

Header format, CRC32 checksums, corruption recovery, and doc comments all done.
See `src/log/header.rs` for on-disk format.

## Optional polish (pick any, or skip to Phase 4)

- [x] `std::io::BufWriter` — merge writes through BufWriter with single flush + sync
- [x] Refactor `merge` to accept paths — `merge` now derives temp path from `self.path`
- [x] Merge correctness tests — deduplication, delete handling, file shrink, empty log, index offset validity
- [x] Split `Header` into `HeaderWriter` (Write+Seek) and `HeaderReader` (Read+Seek)
- [x] Refactored write/index responsibilities — write is thin file layer, execute owns index
- [x] Merge trigger improvement — ratio-based (`entry_count / unique_keys > 2`) replacing flat 10MB check

## Phase 4 — SSTable / LSM-tree

- [ ] Replace `HashMap` index with `BTreeMap` memtable (sorted in-memory store)
- [ ] Flush memtable to sorted on-disk segments (SSTables) when it reaches a size threshold
- [ ] Read path: check memtable first, then search SSTables newest-to-oldest
- [ ] Merge/compact SSTables in background (sorted merge of multiple segment files)
- [ ] Bloom filters per SSTable for fast negative lookups (skip segments that definitely don't have the key)

## Phase 5 — Network layer

- [ ] TCP server with a simple wire protocol
- [ ] Client can connect and issue get/set/delete commands
- [ ] Request/response framing

## Phase 6 — Concurrency

- [ ] `RwLock` for concurrent readers, single writer
- [ ] Connection handling with tokio or std threads
- [ ] Explore MVCC if ambitious

## Architecture notes

Entry format on disk:
```
[magic: 2 bytes (0x4E48 "NH")][crc32: 4 bytes][entry_len: 4 bytes][wincode-serialized Entry]
```

Key files:
- `src/log/header.rs` — `HeaderWriter` (Write+Seek), `HeaderReader` (Read+Seek), `EntryWithHeader` trait on `Entry`, `TryIntoEntryWithLen` trait on `[u8]`, `CorruptionType` enum
- `src/log/mod.rs` — `Log` struct with `write`/`read_next`/`merge`, delegates to header traits
- `src/log/index.rs` — `Index` struct wrapping `HashMap<String, u64>` via `Deref`/`DerefMut`, tracks `entry_count` for ratio-based merge triggering
- `src/log/command.rs` — `Execute` trait on `Log`, REPL command handling
- `src/log/entry.rs` — `Entry` enum (Set/Delete)

## Study list

- ~~`std::io::Seek`, `SeekFrom`, `stream_position()`~~ — learned in Phase 1
- ~~Bitcask paper~~ — read, using as model for Phase 2
- ~~`std::fs::metadata().len()`~~ — learned for size-based merge triggering
- ~~`std::fs::File::sync_all()`~~ — learned and implemented in Phase 2
- ~~CRC32 checksums (`crc32fast` crate)~~ — learned and implemented in Phase 3
- ~~`u32::to_le_bytes()` / `u32::from_le_bytes()`~~ — learned in Phase 3
- `BTreeMap` — sorted in-memory structure needed for Phase 4 memtable
- ~~`std::io::BufWriter`~~ — learned and used in merge for batched writes
- SSTable format — sorted string table, on-disk sorted key-value segments
- LSM-tree architecture — how memtable flushes, levels, and compaction fit together
- Sorted merge (k-way merge) — merging multiple sorted SSTable files into one
- Bloom filters — probabilistic data structure for fast negative lookups (hash functions, false positive rate, bit array sizing)
- Sparse index / index block — how SSTables avoid indexing every key (binary search between index points)
