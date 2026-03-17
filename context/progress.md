# Progress

## Phase 1 — Append-only log with in-memory index (COMPLETE)

- [x] REPL loop — stdin command parsing (get, set, delete) with validation
- [x] Binary serialization with wincode via Entry enum (SchemaRead/SchemaWrite derives)
- [x] Append-only writes — set appends to data.log, records byte offset in HashMap
- [x] Seek-based reads — get looks up offset in index, seeks to position, deserializes
- [x] Index rebuild on startup — scans log file front-to-back using `@` pattern binding
- [x] Delete — removes key from in-memory index
- [x] Delete persistence — tombstone Entry::Delete variant appended to log, survives restart

## Phase 2 — Durability and compaction (COMPLETE)

- [x] Extracted index into `src/log/index.rs` with `Index` type alias and `IndexOps` trait
- [x] Created `Log` struct in `src/log/mod.rs` — owns File + Index together
- [x] Moved command execution into `Execute` trait impl for `Log` in `src/log/command.rs`
- [x] Separated `Entry` into `src/log/entry.rs`
- [x] Cleaned up `run.rs` — now just creates Log and runs the REPL loop
- [x] Log compaction (`merge`) — scans entries, deduplicates, writes to temp file, atomic rename
- [x] `sync_all()` after every write (set/delete) for durability
- [x] Crash-safe merge — `rename` atomically overwrites (POSIX)
- [x] Command shorthand aliases (s/g/d/q) and variable-arity parsing

## Phase 3 — Binary serialization (COMPLETE)

- [x] Header format: `[magic: 2B][crc32: 4B][entry_len: 4B]` — 10 bytes total
- [x] Magic bytes: `0x4E48` ("NH") as u16 little-endian
- [x] `HeaderWriter` / `HeaderReader` traits on any `Write+Seek` / `Read+Seek`
- [x] `EntryWithHeader` trait on `Entry` — `try_into_bytes_with_header()`
- [x] `TryIntoEntryWithLen` trait on `[u8]` — parse header + entry from slice
- [x] `CorruptionType` enum — `NotEnoughBytes`, `MagicBytesMismatch`, `ChecksumMismatch`, `EntryParseError`
- [x] Corruption recovery — reader scans byte-by-byte past bad data to find next valid entry
- [x] BufWriter in merge — batched writes, single flush + sync before rename
- [x] Ratio-based merge trigger — `entry_count / unique_keys > 2`

## Tests (COMPLETE through Phase 3)

- [x] `Entry::key()` / `Entry::value()` — 4 unit tests in `src/log/entry.rs`
- [x] `TryIntoEntryWithLen` — 9 tests: Set/Delete ok, NotEnoughBytes, MagicBytesMismatch, ChecksumMismatch, EntryParseError
- [x] `HeaderWriter` / `HeaderReader` — 5 tests: write Set/Delete, read round-trips, corruption recovery
- [x] Command parser (`TryFrom<&str>`) — 21 tests: every alias, missing args, too many args, unrecognized
- [x] `MemTable::from_file` — 5 tests: empty, single set, set+delete, multiple keys, delete nonexistent
- [x] `Log` — 6 tests: new empty memtable, write offset, read round-trip, write doesn't touch memtable, empty read, memtable rebuild on reopen
- [x] `Execute` — 9 tests: set/get/delete paths, tombstone persistence, overwrite, quit/help no-ops

## Phase 4 — SSTable / LSM-tree (IN PROGRESS)

### Step 1 — Memtable (COMPLETE)
- [x] Renamed `src/log/index.rs` → `src/log/memtable.rs`, `Index` → `MemTable`
- [x] `MemTable` is `BTreeMap<String, Entry>` — stores values directly, not offsets
- [x] `process()` — unified insert/remove + byte-level size tracking
- [x] `MemTable::from_file` — replays WAL on startup
- [x] `maybe_flush()` triggers `flush()` when `memtable.size() > 4MB`
- [x] Renamed `Entry` fields/methods `k`/`v` → `key`/`value` across all files
- [x] Deleted `merge` — replaced by SSTable flush model

### Step 2 — SSTable flush (COMPLETE)
- [x] `flush()` — writes memtable sorted to `data/sstables/{timestamp:020}.sst`
- [x] Microsecond Unix timestamp, zero-padded — lexicographic = chronological sort
- [x] After flush: `sync_all()`, truncate WAL to 0, clear memtable
- [x] `maybe_flush()` wired into `Execute` after Set and Delete commands

### Step 3 — SSTable read path (COMPLETE)
- [x] `Log::get()` — memtable first, then linear scan SSTables newest-to-oldest
- [x] `Log::get()` returns `None` (not error) when sstables dir is missing
- [x] `Log::contains()` — delegates to `get()`, used in Delete to check both layers
- [x] `Execute` Delete arm uses `contains()` — tombstone written even for flushed keys
- [x] `Execute` Get arm uses `Log::get()` — falls through to SSTables on memtable miss

### Refactors and bug fixes (Steps 1–3)
- [x] `Log::new` — 4-arg form `(data_path, memtable_path, sstables_path, truncate)`
- [x] `Log` fields: `file` → `memtable_file`, `path` → `memtable_path`; added `sstables_path`
- [x] `flush()` / `maybe_flush()` — no longer take path arg, use `self.sstables_path`
- [x] `Entry::set()` / `Entry::delete()` — constructors replacing manual struct construction
- [x] `Entry` derives `PartialEq`, implements `From<&Entry>`
- [x] Fixed `process()` size tracking — overwriting a Set now decrements old size before adding new
- [x] All tests updated: inlined single-use bindings, chained `.unwrap()`, consistent naming

### Step 4 — SSTable compaction (COMPLETE)
- [x] `src/log/compact.rs` — `Log::compact()` k-way merge across all SSTables
- [x] Sorted files newest-to-oldest by timestamp filename; vec index = recency priority
- [x] Per-iteration: find global minimum key, newest participant wins, all participants advance cursor
- [x] Mid-loop `flush_to()` when compaction memtable hits 4MB threshold; guarded final flush
- [x] Original SSTables deleted after compacted output written
- [x] `MemTable::flush_to(path)` — extracted helper used by both `Log::flush()` and `compact()`
- [x] `MemTable::should_flush()` — 4MB threshold check, shared constant `FLUSH_THRESHOLD_MB`
- [x] `Log::flush_count` — initialized from existing SSTable count on startup, incremented each flush
- [x] Compaction triggered every `MERGE_EVERY_N_FLUSHES = 10` flushes inside `flush()`
- [x] `temp_log()` pattern fixed across all test modules — returns `(TempDir, Log)` to keep dir alive
- [x] `compact()` uses `Vec<(Option<Entry>, SSTable)>` — no raw file handles, bloom filter boundary respected
- [x] `read_dir` errors propagated via `collect::<Result<_, _>>()?`

### Step 5 — Bloom filters (IN PROGRESS)

#### Design decisions
- One bloom filter per SSTable (not per block)
- Bloom filter stored as footer inside .sst file (not a separate file)
- Footer format: `[bloom_filter bytes (byte_count)][bit_count: 4B u32 LE]` — bit_count read first to derive byte_count
- xxh3 chosen for bloom hashing (better distribution); crc32fast stays for WAL header checksums
- k=7 hashes, 10 bits/key — standard config, ~1% false positive rate
- Double-hashing (Kirsch-Mitzenmacher): `pos = (hash1 + i * hash2) % bit_count` for i in 0..7
  - hash1 = `xxh3::hash64_with_seed(key, 0)`, hash2 = `xxh3::hash64_with_seed(key, 1)`

#### Completed — write path (`src/log/memtable.rs` `flush_to()`)
- [x] Bloom filter written as footer of every SSTable on flush
- [x] Bit array sized at `(key_count * 10).div_ceil(8)` bytes
- [x] Set bits: `bloomfilter[pos / 8] |= 1 << (pos % 8)`
- [x] Footer: bloom bytes then `bit_count` as u32 LE (4 bytes)

#### Completed — `src/log/sstable.rs` (new file this session)
- [x] `BloomFilter` struct — `bit_count: usize`, `inner: Vec<u8>`, `Deref<Target = [u8]>`
- [x] `BloomFilter::blank(bit_count)` — allocates zeroed bit array
- [x] `BloomFilterReader` blanket trait on any `Read + Seek` — reads bloom filter from footer
  - Seeks to end, reads `bit_count` from last 4 bytes, derives `byte_count`
  - Bounds checks: returns `None` if file too small or bit_count inconsistent
  - Seeks to `End(-byte_count - 4)`, reads bloom bytes
  - **Restores cursor to position 0 before returning** — callers don't need to compensate
- [x] `SSTable` struct — `bloom_filter: BloomFilter`, `bloom_filter_pos: u64`, `file: File`
  - No stored `entry` field — entries are handed out as owned values, not cached
- [x] `SSTable::from_path(path)` — opens file, reads bloom filter, validates non-empty via `contains_entry_with_header`, seeks to start
- [x] `SSTable::read_next_entry()` — checks `stream_position >= bloom_filter_pos` internally before reading; returns `Option<Entry>` owned
- [x] Bloom filter boundary enforced inside `read_next_entry` — callers cannot accidentally read footer bytes as entries

#### Completed — `src/log/header.rs` improvements this session
- [x] `contains_entry_with_header` added to `HeaderReader` — saves cursor, seeks to 0, tries to read entry, restores cursor; returns `bool`
- [x] `buf_len` calculation in `read_next_entry_with_header` simplified — seek to end + `stream_position()` instead of reading entire file to Vec (O(1) vs O(n))

#### Completed — read path (`src/log/mod.rs` `get()`)
- [x] `get()` uses `SSTable::from_path` — bloom filter loaded automatically
- [x] Bloom filter checked for all 7 positions before scanning entries — skips SSTable on any 0 bit
- [x] Entry scan loop reads owned entries, returns directly without `.to_owned()`
- [x] Boundary condition enforced via `stream_position < bloom_filter_pos` in while condition

#### Completed — compaction (`src/log/compact.rs`)
- [x] Uses `SSTable` struct — `read_next_entry()` internally enforces bloom filter boundary
- [x] No longer reads footer bytes as corrupt entries accidentally

#### Bug fixes made during this session
- [x] Hash formula: was `(h1 + i) * h2`, fixed to `h1 + i * h2` via `hash1.wrapping_add((i as u64).wrapping_mul(hash2))`
- [x] Footer stores `bit_count` not `byte_count` — avoids losing precision when key_count * 10 is not a multiple of 8
- [x] `u32::from_le_bytes` used instead of `usize::from_le_bytes` — usize is 8 bytes on 64-bit, panics on 4-byte slice
- [x] `read_bloom_filter` restores cursor to 0 after reading — previously left cursor at EOF

#### Bloom filter tests (src/log/memtable.rs) — 3 passing, 4 stubs remaining
- [x] `flush_writes_bloomfilter_footer_to_sstable` — verifies footer bytes present after flush
- [x] `bloomfilter_reports_present_for_inserted_key` — key hashes to all-set bits
- [x] `bloomfilter_reports_absent_for_missing_key` — uses "z" as absent key ("b" is a valid false positive for a 10-bit filter built from "a")
- [ ] 4 `#[ignore]` stubs still to fill in

#### Compact tests (src/log/compact.rs) — 5 passing, 2 stubs remaining
- [x] `compact_with_no_sstables_is_noop`
- [x] `compact_newest_wins_for_duplicate_key`
- [x] `compact_preserves_all_unique_keys`
- [x] `compact_deletes_original_sstables`
- [x] `compact_result_readable_via_get`
- [x] `compact_reduces_sstable_count`
- [ ] `compact_single_sstable_produces_one_output_and_deletes_original` — `#[ignore]` stub
- [ ] `compact_three_sstables_with_overlapping_keys` — `#[ignore]` stub

#### TODO — still to complete for Step 5
- [ ] Fill in 4 `#[ignore]` bloom filter stubs in `src/log/memtable.rs`
- [ ] Fill in 2 `#[ignore]` compact stubs in `src/log/compact.rs`
- [ ] Doc comments for `BloomFilter`, `BloomFilterReader`, `SSTable` in `src/log/sstable.rs`
- [ ] Doc comments for `compact()` updated to reflect SSTable usage
- [ ] Integration tests: `get()` skips SSTable when bloom filter says absent; `get()` finds key when bloom filter says maybe-present
- [ ] Clean up unused imports in `compact.rs` (`File`, `OpenOptions`, `HeaderReader`)
- [ ] `mod.rs` still declares `pub mod merge` and calls `self.merge()` — rename to `pub mod compact` / `self.compact()`

### Step 6 — WalEntry / SstEntry type split (PLANNED)

#### Motivation
Currently `Entry` has two variants: `Set { key, value }` and `Delete { key }`. `Delete` is only valid in the WAL — SSTables never contain tombstones because `process()` removes keys from the memtable before flush. However there is a **tombstone resurrection bug**: if key "a" is in SSTable 1, deleted (WAL only, memtable removes it), then flushed (SSTable 2 doesn't contain "a"), then compacted — compact sees "a" in SSTable 1 and no tombstone to suppress it, so "a" comes back. The type split fixes this by forcing tombstones into SSTables.

#### Plan
- `WalEntry { Set { key, value }, Delete { key } }` — used for WAL reads/writes
- `SstEntry { key: String, value: String }` — used for SSTable reads/writes (Set only, no Delete variant)
- Shared serialization trait so both types use the same `HeaderWriter`/`HeaderReader` machinery
- `MemTable` stores `SstEntry` — `process()` takes `WalEntry`, inserts on Set, removes on Delete
- `flush_to()` writes `SstEntry` — compiler makes it impossible to write a tombstone to an SSTable
- During flush, if `process()` received a Delete, the key is absent from the BTreeMap — the absence itself is the tombstone, written as an `SstEntry::Delete` (or a dedicated tombstone `SstEntry` variant) to SSTables so compact can suppress it
- `compact()` must skip/drop `SstEntry::Delete` tombstones from output to avoid accumulating them

#### TODO
- [ ] Define `WalEntry` and `SstEntry` in `src/log/entry.rs` (or separate files)
- [ ] Update `HeaderWriter` / `HeaderReader` to work with both types via trait
- [ ] Update `MemTable::process()` to take `WalEntry`
- [ ] Update `flush_to()` to write `SstEntry`
- [ ] Update `compact()` to handle `SstEntry::Delete` tombstones — skip from output
- [ ] Update `Log::get()` to return `Option<SstEntry>` (or map to user-facing type)
- [ ] Update all tests
- [ ] Doc comments

### Current test count: ~90 passing, 6 ignored (4 bloom filter stubs + 2 compact stubs)
