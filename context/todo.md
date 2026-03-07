# Todo

## Phase 3 — Binary serialization

### Approach: wrap wincode with a header

Keep using wincode for Entry serialization. Add a header around each serialized entry:

```
[magic: 2 bytes][crc32: 4 bytes][entry_len: 4 bytes][wincode-serialized Entry]
```

Header is 10 bytes total. Entry data is unchanged from Phase 2.

### Concepts

**Magic bytes** — a fixed constant (e.g. `0x4E48` for "NH") written at the start of every entry.
Purpose: recovery. After corruption, scan forward byte-by-byte looking for the magic bytes
to find where the next valid entry starts. Without this, one corrupt entry means
everything after it is unreadable.

**CRC32 checksum** — a 4-byte "fingerprint" computed from the entry data.
Written into the header when the entry is stored. On read, recompute the checksum from
the data and compare. If they don't match, the entry was corrupted (bit flip, partial write, etc).
The `crc32fast` crate does this: `let checksum: u32 = crc32fast::hash(&data);` — that's the whole API.

**entry_len** — the byte length of the wincode-serialized entry that follows.
Lets you skip entries without deserializing (jump ahead by entry_len bytes to the next header).
Also lets you know exactly how many bytes to read and pass to `wincode::deserialize`.

### Tasks

- [ ] Add `crc32fast` dependency
- [ ] Define header constants: magic bytes, header size (10 bytes)
- [ ] Write path — build header (magic + crc32 of entry bytes + entry_len), then write header + entry bytes
- [ ] Read path — read header, verify magic, read entry_len bytes, verify crc32, then `wincode::deserialize`
- [ ] Update index scan (`from_file`) to use new format
  - On bad magic or bad checksum: scan forward byte-by-byte for next magic bytes
  - This replaces the current `Err(_) => break` with actual recovery
- [ ] Update merge to write new format
- [ ] Consider: `std::io::BufWriter` for batching writes

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
