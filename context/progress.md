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

### Current test count: 77 passing, 0 ignored

### Next: Step 4 — SSTable merge/compaction
