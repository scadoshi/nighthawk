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

## Phase 4 — SSTable / LSM-tree (IN PROGRESS)

### Steps 1–4 (COMPLETE)
See `context/progress.md` for full detail. Memtable, flush, read path, compaction all done.

### Step 5 — Bloom filters (NEARLY COMPLETE)

#### Done
- [x] Write path — bloom filter footer appended to every SSTable on `flush_to()`
- [x] `SSTable` struct in `src/log/sstable.rs` — encapsulates bloom filter, boundary position, file
- [x] `BloomFilterReader` blanket trait — reads footer, restores cursor to 0
- [x] `contains_entry_with_header` on `HeaderReader` — save/restore cursor, peek for valid entry
- [x] `get()` in `mod.rs` — bloom filter checked before scanning, entries returned owned
- [x] `compact()` in `compact.rs` — uses SSTable, boundary enforced inside `read_next_entry()`
- [x] All known bugs fixed (hash formula, bit_count vs byte_count, u32 vs usize, cursor restore)
- [x] 3 bloom filter tests passing in `memtable.rs`
- [x] 6 compact tests passing in `compact.rs`

#### Still TODO
- [ ] Fill in 4 `#[ignore]` bloom filter stubs in `src/log/memtable.rs`
- [ ] Fill in 2 `#[ignore]` compact stubs in `src/log/compact.rs`:
  - `compact_single_sstable_produces_one_output_and_deletes_original`
  - `compact_three_sstables_with_overlapping_keys`
- [ ] Integration tests in `mod.rs`:
  - `get_skips_sstable_when_bloom_filter_says_absent`
  - `get_finds_key_when_bloom_filter_says_maybe_present`
- [ ] Doc comments for `BloomFilter`, `BloomFilterReader`, `SSTable` in `src/log/sstable.rs`
- [ ] Clean up dead imports in `compact.rs`: `File`, `OpenOptions`, `HeaderReader`
- [ ] Fix `mod.rs`: `pub mod merge` → `pub mod compact`, `self.merge()` → `self.compact()`

### Step 6 — WalEntry / SstEntry type split (NEXT)

#### Why
`Entry::Delete` currently only lives in the WAL but the type system doesn't enforce this.
SSTables never contain tombstones, which causes a **tombstone resurrection bug**: delete "a" → WAL,
flush empty memtable → SSTable 2 (no "a"), compact → SSTable 1's "a" survives, key comes back.
The fix is to write tombstones into SSTables during flush so compact can suppress them.

#### What to build
- `WalEntry` enum — `Set { key, value }` | `Delete { key }` — WAL only
- `SstEntry` enum — `Set { key, value }` | `Delete { key }` — SSTable only (Delete = tombstone marker)
- Shared serialization trait so both types use the same `HeaderWriter`/`HeaderReader` machinery
- `MemTable::process(WalEntry)` — insert on Set, remove on Delete
- `flush_to()` writes tombstone `SstEntry::Delete` for any key removed from memtable since last flush
- `compact()` drops `SstEntry::Delete` from output (tombstones suppressed after merge)
- `Log::get()` treats `SstEntry::Delete` as definitive absence — stop searching older SSTables

#### TODO
- [ ] Define `WalEntry` and `SstEntry` (in `src/log/entry.rs` or separate files)
- [ ] Update `HeaderWriter` / `HeaderReader` machinery to work with both via trait
- [ ] Update `MemTable::process()` to accept `WalEntry`
- [ ] Update `flush_to()` to emit tombstones for deleted keys
- [ ] Update `compact()` to drop tombstones from output
- [ ] Update `Log::get()` to stop on `SstEntry::Delete` (short-circuit older SSTables)
- [ ] Update all tests to use new types
- [ ] Doc comments throughout

#### Testing targets
- [ ] Flush emits tombstone for deleted key
- [ ] `get()` returns None when newest SSTable has tombstone for key
- [ ] `get()` does not find key in older SSTable when newer SSTable has tombstone
- [ ] `compact()` does not include tombstones in output when no older version exists
- [ ] `compact()` still suppresses key when older SSTable has value and newer has tombstone

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
- `src/log/header.rs` — `HeaderWriter`, `HeaderReader`, `EntryWithHeader`, `TryIntoEntryWithLen`, `CorruptionType`
- `src/log/mod.rs` — `Log` struct: `write`, `get`, `contains`, `flush`, `maybe_flush`
- `src/log/memtable.rs` — `MemTable` wrapping `BTreeMap<String, Entry>`, `process()`, `flush_to()`, `should_flush()`
- `src/log/sstable.rs` — `BloomFilter`, `BloomFilterReader`, `SSTable`
- `src/log/compact.rs` — `Log::compact()` k-way merge
- `src/log/command.rs` — `Execute` trait on `Log`, REPL command handling
- `src/log/entry.rs` — `Entry` enum (Set/Delete) — to be split into WalEntry/SstEntry

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
- Tombstone propagation in LSM-trees — how deletes must flow through SSTable levels to avoid resurrection; compaction as the suppression point
- Rust trait objects vs generics for shared serialization — needed for WalEntry/SstEntry shared header write trait
- Sparse index / index block — how SSTables avoid indexing every key (binary search between index points); relevant after WalEntry/SstEntry
- TCP framing and wire protocols — needed for Phase 5; look at length-prefixed framing and simple request/response design
- `tokio` async runtime basics — needed for Phase 6 connection handling
