# Todo

## Phase 3 — Binary serialization (remaining)

### Done

Header format implemented and wired into all read/write paths. Corruption recovery
scans byte-by-byte for next valid magic + CRC match. See `src/log/header.rs`.

### Remaining tasks

- [ ] Consider: `std::io::BufWriter` for batching writes (optional optimization)
- [ ] Fix merge trigger — currently merges on every command once file hits 10MB of unique data.
  Use ratio-based trigger: merge when file_size / post-merge size > threshold.
- [ ] Add `tracing` and `tracing-subscriber` — instrument read/write/merge/index rebuild
  with structured logging to console. Replace ad-hoc printlns with proper log levels.
- [ ] Documentation pass — add doc comments (`///`) across all public types, traits, and methods
- [ ] Decide if Phase 3 is complete or if there's more to polish

### Architecture notes

Entry format on disk:
```
[magic: 2 bytes (0x4E48 "NH")][crc32: 4 bytes][entry_len: 4 bytes][wincode-serialized Entry]
```

Header is 10 bytes. Entry data is wincode-serialized, unchanged from Phase 2.

Key files:
- `src/log/header.rs` — `Header` trait on `File`, `parse_entry` standalone fn, `CorruptionType` enum
- `src/log/mod.rs` — `Log` struct with `write`/`read_next`/`merge`, delegates to header trait
- `src/log/index.rs` — `Index` trait on `HashMap<String, u64>`, rebuilds from file using header-aware reads
- `src/log/command.rs` — `Execute` trait on `Log`, REPL command handling
- `src/log/entry.rs` — `Entry` enum (Set/Delete), unchanged

## Future phases

- Phase 4: SSTable / LSM-tree (BTreeMap memtable, sorted segments, bloom filters)
- Phase 5: Network layer (TCP server, wire protocol)
- Phase 6: Concurrency (RwLock, tokio/threads, MVCC)

## Study list

- ~~`std::io::Seek`, `SeekFrom`, `stream_position()`~~ — learned in Phase 1
- ~~Bitcask paper~~ — read, using as model for Phase 2
- ~~`std::fs::metadata().len()`~~ — learned for size-based merge triggering
- ~~`std::fs::File::sync_all()`~~ — learned and implemented in Phase 2
- ~~CRC32 checksums (`crc32fast` crate)~~ — learned and implemented in Phase 3
- ~~`u32::to_le_bytes()` / `u32::from_le_bytes()`~~ — learned in Phase 3, little-endian byte encoding
- `BTreeMap` — sorted in-memory structure needed for Phase 4 memtable
- `std::io::BufWriter` — batching writes for better performance
