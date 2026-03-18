# Todo

## Phase 3 — Binary serialization (COMPLETE)

Header format, CRC32 checksums, corruption recovery, and doc comments all done.
See `src/log/header/` for on-disk format.

## Phase 4 — SSTable / LSM-tree (COMPLETE)

Steps 1–6 all done. See `context/progress.md` for full detail.
89 tests passing, 0 ignored.

### Remaining polish (optional)
- [ ] `compact` REPL command — expose compaction manually alongside the auto-trigger every 10 flushes
- [ ] `scan <start> <end>` REPL command — range query over memtable + SSTables using sorted order

### Step 4.5 — Leveled compaction (optional, after Step 6)
- [ ] Organize SSTables into levels (L0, L1, L2...) — L0 accepts direct flushes, L1+ enforce non-overlapping key ranges
- [ ] Compact L0 → L1 when L0 file count hits threshold (e.g. 4)
- [ ] Each level is 10x larger than the previous — controls read/write amplification tradeoff

## Phase 5 — Network layer

- [ ] TCP server with a simple wire protocol
- [ ] Client can connect and issue get/set/delete commands
- [ ] Request/response framing

## Phase 6 — Concurrency

- [ ] `RwLock` for concurrent readers, single writer
- [ ] Connection handling with tokio or std threads
- [ ] Explore MVCC if ambitious

## Architecture notes

SSTable file layout:
```
[entry 0 with header][entry 1 with header]...[bloom_filter bytes][bit_count: 4B u32 LE]
```

Entry header format:
```
[magic: 2 bytes (0x4E48 "NH")][crc32: 4 bytes][entry_len: 4 bytes][wincode-serialized Entry]
```

Key files:
- `src/log/entry.rs` — `Entry` enum (Set/Delete) — single type used by all layers
- `src/log/header/` — `HeaderWriter`, `HeaderReader`, `HeaderSerializer`, `HeaderDeserializer`, `CorruptionType`
- `src/log/mod.rs` — `Log` struct: `write`, `get`, `contains`, `flush`, `maybe_flush`
- `src/log/wal/memtable.rs` — `MemTable` wrapping `BTreeMap<String, Entry>`, `process()`, `flush_to()`, `should_flush()`
- `src/log/sstable/mod.rs` — `SSTable` struct: bloom filter, boundary position, entry iteration
- `src/log/sstable/bloom_filter.rs` — `BloomFilter`, `BloomFilterReader`
- `src/log/sstable/compact.rs` — `Log::compact()` k-way merge; tombstone winners dropped
- `src/log/command.rs` — `Execute` trait on `Log`, REPL command handling

## Study list

- ~~`std::io::Seek`, `SeekFrom`, `stream_position()`~~ — learned in Phase 1
- ~~Bitcask paper~~ — read, using as model for Phase 2
- ~~`std::fs::metadata().len()`~~ — learned for size-based merge triggering
- ~~`std::fs::File::sync_all()`~~ — learned and implemented in Phase 2
- ~~CRC32 checksums (`crc32fast` crate)~~ — learned and implemented in Phase 3
- ~~`u32::to_le_bytes()` / `u32::from_le_bytes()`~~ — learned in Phase 3
- ~~`BTreeMap`~~ — sorted in-memory structure, understood as ordered map for memtable
- ~~`std::io::BufWriter`~~ — learned and used in merge for batched writes
- ~~SSTable format — sorted string table, on-disk sorted key-value segments~~ — learned and implemented
- ~~LSM-tree architecture — how memtable flushes, levels, and compaction fit together~~ — learned and implemented
- ~~Sorted merge (k-way merge) — merging multiple sorted SSTable files into one~~ — learned and implemented
- ~~Bloom filters — probabilistic data structure for fast negative lookups~~ — learned and implemented: k=7 hashes, 10 bits/key, double-hashing, ~1% FP rate, xxh3
- ~~Tombstone propagation in LSM-trees — how deletes must flow through SSTable levels to avoid resurrection; compaction as the suppression point~~ — learned and implemented
- Rust trait objects vs generics for shared serialization — relevant for future type design
- Sparse index / index block — how SSTables avoid indexing every key (binary search between index points)
- TCP framing and wire protocols — needed for Phase 5; look at length-prefixed framing and simple request/response design
- `tokio` async runtime basics — needed for Phase 6 connection handling
