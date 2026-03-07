# Todo

## Phase 3 — Binary serialization

- [ ] Add fixed-size entry headers (magic bytes + CRC32 checksum + key/value lengths)
  - Enables reliable corruption detection and recovery
  - Can scan forward to next magic bytes after corrupt entry instead of breaking
- [ ] CRC32 checksums per entry — verify integrity on read
- [ ] Consider: `crc32fast` crate for checksum implementation

## Future phases

- Phase 4: SSTable / LSM-tree (BTreeMap memtable, sorted segments, bloom filters)
- Phase 5: Network layer (TCP server, wire protocol)
- Phase 6: Concurrency (RwLock, tokio/threads, MVCC)

## Study list

- ~~`std::io::Seek`, `SeekFrom`, `stream_position()`~~ — learned in Phase 1
- ~~Bitcask paper~~ — read, using as model for Phase 2
- ~~`std::fs::metadata().len()`~~ — learned for size-based merge triggering
- ~~`std::fs::File::sync_all()`~~ — learned and implemented in Phase 2
- CRC32 checksums (`crc32fast` crate) — for detecting corrupt entries in Phase 3
- `BTreeMap` — sorted in-memory structure needed for Phase 4 memtable
- `std::io::BufWriter` — batching writes for better performance
