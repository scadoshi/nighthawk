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

### Step 1 — Memtable (COMPLETE)
- [x] Renamed `src/log/index.rs` → `src/log/memtable.rs`, `Index` → `MemTable`
- [x] `MemTable` is `BTreeMap<String, Entry>` — stores values directly, not offsets
- [x] Startup replays WAL into memtable via `MemTable::from_file`
- [x] Write path: WAL write + `memtable.process(entry)`
- [x] Read path: `memtable.get(&key)` — no file seek
- [x] `process()` unifies insert/remove and tracks byte `size`
- [x] `maybe_flush()` triggers `flush()` when `memtable.size() > 4MB`
- [x] Renamed `Entry` fields `k`/`v` → `key`/`value`, methods `k()`/`v()` → `key()`/`value()` across all files
- [x] Deleted `merge` — replaced by SSTable flush model

### Step 2 — SSTable flush (COMPLETE)
- [x] `flush()` — writes memtable sorted to `data/sstables/{timestamp:020}.sst`
- [x] Timestamp filename: microsecond Unix epoch, zero-padded — lexicographic = chronological sort
- [x] After flush: `sync_all()`, truncate WAL, clear memtable
- [x] `maybe_flush()` wired into `Execute` after each command

### Step 3 — SSTable read path (COMPLETE)
- [x] `Log::get()` — checks memtable first, then scans SSTables newest-to-oldest
- [x] Linear scan per SSTable file using `read_next_entry_with_header`
- [x] `create_dir_all` guards `read_dir` so missing sstables dir doesn't panic
- [x] Sort descending by filename (lexicographic = newest first)
- [x] Wire `Log::get()` into `Execute` Get arm
- [x] Placeholder tests added for SSTable read path, flush, maybe_flush — fill in with `#[ignore]` stubs

### Step 4 — SSTable merge/compaction (COMPLETE)
- [x] Merge multiple SSTable files into one — sorted k-way merge in `src/log/compact.rs`
- [x] Drop deleted keys and superseded values during merge — newest SSTable wins on duplicate key
- [x] Trigger merge every N flushes — `flush_count` on `Log`, `COMPACT_EVERY_N_FLUSHES = 10`

### Step 4.5 — Leveled compaction (optional)
- [ ] Organize SSTables into levels (L0, L1, L2...) — L0 accepts direct flushes, L1+ enforce non-overlapping key ranges
- [ ] Compact L0 → L1 when L0 file count hits threshold (e.g. 4)
- [ ] Each level is 10x larger than the previous — controls read/write amplification tradeoff

### Step 5 — Bloom filters
- [ ] One bloom filter per SSTable — skip files that definitely don't contain the key
- [ ] Learn: `bloomfilter` or `fastbloom` crate, or implement from scratch

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
- `src/log/mod.rs` — `Log` struct with `write`/`read_next`/`get`/`flush`/`maybe_flush`, delegates to header traits
- `src/log/memtable.rs` — `MemTable` wrapping `BTreeMap<String, Entry>`, tracks byte `size`, `process()` for insert/remove
- `src/log/command.rs` — `Execute` trait on `Log`, REPL command handling
- `src/log/entry.rs` — `Entry` enum (Set/Delete)

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
- Bloom filters — probabilistic data structure for fast negative lookups (hash functions, false positive rate, bit array sizing)
- Sparse index / index block — how SSTables avoid indexing every key (binary search between index points)
