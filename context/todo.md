# Todo

## Phase 3 — Binary serialization

### Entry format

Design a fixed-layout entry format to replace raw wincode-serialized tuples:

```
[magic: 2 bytes][crc32: 4 bytes][key_len: 4 bytes][val_len: 4 bytes][tag: 1 byte][key][value]
```

- `magic` — constant bytes (e.g. `0x4E48` for "NH") to identify entry boundaries
- `crc32` — checksum over everything after the checksum field (key_len + val_len + tag + key + value)
- `key_len` / `val_len` — fixed-size u32 lengths so you can skip entries without deserializing
- `tag` — 0 for Set, 1 for Delete (replaces the wincode enum discriminant)
- `key` / `value` — raw bytes, length determined by header

### Tasks

- [ ] Define entry header struct with fixed-size fields
- [ ] Write serialization — build the header + payload bytes manually (no more `wincode::serialize` for entries)
- [ ] Write CRC32 checksum over payload when writing entries
  - Study: `crc32fast` crate
- [ ] Read deserialization — parse header, verify checksum, extract key/value
- [ ] Verify checksum on every `get` — return corruption error if mismatch
- [ ] Update index scan (`from_file`) to use new format
  - Use magic bytes to find entry boundaries
  - Skip entries with bad checksums instead of breaking
- [ ] Update merge to write new format
- [ ] Update `serialized_size` usage — entry size is now `header_size + key_len + val_len`
- [ ] Consider: `std::io::BufWriter` for batching writes instead of writing + syncing each entry individually

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
- `u32::to_le_bytes()` / `u32::from_le_bytes()` — manual binary encoding for header fields
- `BTreeMap` — sorted in-memory structure needed for Phase 4 memtable
- `std::io::BufWriter` — batching writes for better performance
