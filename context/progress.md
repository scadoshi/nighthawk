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

### Step 4 — SSTable merge/compaction (COMPLETE)
- [x] `src/log/compact.rs` — `Log::compact()` k-way merge across all SSTables
- [x] Sorted files newest-to-oldest by timestamp filename; vec index = recency priority
- [x] Per-iteration: find global minimum key, newest participant wins, all participants advance cursor
- [x] Mid-loop `flush_to()` when compaction memtable hits 4MB threshold; guarded final flush
- [x] Original SSTables deleted after compacted output written
- [x] `MemTable::flush_to(path)` — extracted helper used by both `Log::flush()` and `compact()`
- [x] `MemTable::should_flush()` — 4MB threshold check, shared constant `FLUSH_THRESHOLD_MB`
- [x] `Log::flush_count` — initialized from existing SSTable count on startup, incremented each flush
- [x] Compaction triggered every `COMPACT_EVERY_N_FLUSHES = 10` flushes inside `flush()`
- [x] `temp_log()` pattern fixed across all test modules — returns `(TempDir, Log)` to keep dir alive

### Step 5 — Bloom filters (IN PROGRESS)

#### Completed
- [x] Bloom filter write path in `flush_to()` (`src/log/memtable.rs`)
- [x] SSTable file layout: `[entries...][bloom_filter bytes][bit_count: 4B (u32 LE)]`
- [x] Hashing: `xxh3::hash64_with_seed` with seeds 0 and 1, double-hashing to derive k=7 positions
- [x] Bit array: `Vec<u8>`, sized at `(key_count * 10).div_ceil(8)` bytes (10 bits/key, ~1% FP rate)
- [x] Bit manipulation: `bloomfilter[pos / 8] |= 1 << (pos % 8)` to set, `& (1 << ...)` to check
- [x] 4 bloom filter test stubs added (`#[ignore]`) in `src/log/memtable.rs`

#### Known bug to fix
- [ ] Line 96 in `flush_to()`: double-hashing formula has wrong precedence — `hash1.wrapping_add(i).wrapping_mul(hash2)` computes `(h1+i)*h2` but should be `h1 + i*h2` → fix to `hash1.wrapping_add((i as u64).wrapping_mul(hash2))`

#### TODO — read path
- [ ] `Log::get()` in `src/log/mod.rs` — before scanning SSTable entries, read bloom filter from footer:
  - Seek to EOF-4, read `bit_count` as u32 LE
  - Seek to EOF-4-byte_count, read bloom filter bytes
  - Hash the lookup key with same xxh3 double-hashing, check all 7 bit positions
  - If any bit is 0 → skip this SSTable entirely (key definitely not present)
  - If all bits are 1 → proceed with linear scan of entries
- [ ] Bound entry reading region: stop reading entries at `file_len - byte_count - 4` so bloom filter bytes aren't misinterpreted as entries
- [ ] Same boundary logic needed in `merge` (`src/log/merge.rs`) when reading SSTables for compaction — skip bloom filter bytes, don't need to check them

#### TODO — testing
- [ ] Fill in the 4 `#[ignore]` bloom filter test stubs in `src/log/memtable.rs`
- [ ] Integration test: `get()` skips SSTable when bloom filter says absent
- [ ] Integration test: `get()` still finds key when bloom filter says maybe-present

#### Design decisions made
- One bloom filter per SSTable (not per block)
- Bloom filter stored as footer inside .sst file (not separate file)
- Footer format: bloom bytes followed by 4-byte bit_count — read bit_count first to know how far back to read
- xxh3 chosen over crc32 for bloom hashing (better distribution); crc32fast stays for WAL checksums
- k=7 hashes, 10 bits/key is the standard config (~1% false positive rate)

### Current test count: 81 passing, 4 ignored (bloom filter stubs)
