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
- [x] `CorruptionType` enum — `NotEnoughBytes`, `MagicBytesMismatch`, `ChecksumMismatch`, `ParseError`
- [x] Corruption recovery — reader scans byte-by-byte past bad data to find next valid entry
- [x] BufWriter in merge — batched writes, single flush + sync before rename
- [x] Ratio-based merge trigger — `entry_count / unique_keys > 2`

## Phase 4 — SSTable / LSM-tree (COMPLETE)

### Step 1 — Memtable (COMPLETE)
- [x] Renamed `src/log/index.rs` → `src/log/memtable.rs`, `Index` → `MemTable`
- [x] `MemTable` is `BTreeMap<String, Entry>` — stores values directly, not offsets
- [x] `process()` — unified insert/remove + byte-level size tracking
- [x] `MemTable::from_file` — replays WAL on startup
- [x] `maybe_flush()` triggers `flush()` when `memtable.size() > 4MB`
- [x] Deleted `merge` — replaced by SSTable flush model

### Step 2 — SSTable flush (COMPLETE)
- [x] `flush()` — writes memtable sorted to `data/sstables/{timestamp:020}.sst`
- [x] Microsecond Unix timestamp, zero-padded — lexicographic = chronological sort
- [x] After flush: `sync_all()`, truncate WAL to 0, clear memtable
- [x] `maybe_flush()` wired into `Execute` after Set and Delete commands

### Step 3 — SSTable read path (COMPLETE)
- [x] `Log::get()` — memtable first, then linear scan SSTables newest-to-oldest
- [x] `Log::contains()` — delegates to `get()`, used in Delete to check both layers
- [x] `Execute` Delete arm uses `contains()` — tombstone written even for flushed keys

### Step 4 — SSTable compaction (COMPLETE)
- [x] `src/log/sstable/compact.rs` — `Log::compact()` k-way merge across all SSTables
- [x] Sorted files newest-to-oldest by timestamp filename; vec index = recency priority
- [x] Per-iteration: find global minimum key, newest participant wins, all participants advance cursor
- [x] Mid-loop `flush_to()` when compaction memtable hits 4MB threshold; guarded final flush
- [x] Original SSTables deleted after compacted output written
- [x] `MemTable::flush_to(path)` — extracted helper used by both `Log::flush()` and `compact()`
- [x] `MemTable::should_flush()` — 4MB threshold check, shared constant `FLUSH_THRESHOLD_MB`
- [x] `Log::flush_count` — initialized from existing SSTable count on startup, incremented each flush
- [x] Compaction triggered every `COMPACT_EVERY_N_FLUSHES = 10` flushes inside `flush()`

### Step 5 — Bloom filters (COMPLETE)

#### Design decisions
- One bloom filter per SSTable (not per block)
- Bloom filter stored as footer inside .sst file (not a separate file)
- Footer format: `[bloom_filter bytes (byte_count)][bit_count: 4B u32 LE]` — bit_count read first to derive byte_count
- xxh3 chosen for bloom hashing (better distribution); crc32fast stays for WAL header checksums
- k=7 hashes, 10 bits/key — standard config, ~1% false positive rate
- Double-hashing (Kirsch-Mitzenmacher): `pos = (hash1 + i * hash2) % bit_count` for i in 0..7
  - hash1 = `xxh3::hash64_with_seed(key, 0)`, hash2 = `xxh3::hash64_with_seed(key, 1)`

#### Completed
- [x] Write path — bloom filter footer appended to every SSTable on `flush_to()`
- [x] `BloomFilter` struct in `src/log/sstable/bloom_filter.rs` — bit array, insert, may_contain
- [x] `BloomFilterReader` blanket trait on `Read + Seek` — reads footer, restores cursor to 0
- [x] `SSTable` struct — `bloom_filter`, `bloom_filter_pos`, `file`
- [x] `SSTable::from_path` — opens file, reads bloom filter, validates non-empty
- [x] `SSTable::read_next_entry()` — enforces bloom filter boundary internally
- [x] `header_has_at_least_one` on `HeaderReader` — save/restore cursor, peek for valid entry
- [x] `get()` in `mod.rs` — bloom filter checked before scanning, entries returned owned
- [x] `compact()` uses SSTable — boundary enforced inside `read_next_entry()`
- [x] Full doc comments for `BloomFilter`, `BloomFilterReader`, `SSTable`, and all methods
- [x] All bloom filter and compact stubs filled in — 89 tests passing, 0 ignored

### Step 6 — Entry type consolidation (COMPLETE)

#### Motivation
`WalEntry`/`SstEntry` split introduced to enforce SSTable-only-Set constraint created a
**tombstone resurrection bug**: deleting a flushed key cleared the memtable only; the SSTable
still contained the Set and `get()` would find it again. Fix: single `Entry` enum used everywhere;
tombstones survive into SSTables; `compact()` drops tombstone winners so they don't accumulate.

#### Completed
- [x] Deleted `src/log/wal/entry.rs` and `src/log/sstable/entry.rs`
- [x] Created `src/log/entry.rs` — single `Entry` enum (Set/Delete) used by all layers
- [x] `memtable.process(Entry::Delete)` — stores tombstone in BTreeMap instead of removing key
- [x] `flush_to()` — writes all entries (Set and Delete) to SSTable; tombstone keys in bloom filter
- [x] `Log::get()` — memtable tombstone hit returns `Ok(None)`; SSTable tombstone hit returns `Ok(None)` and stops search
- [x] `compact()` — uses `seen_keys: HashSet` to track winners; tombstone winners dropped from output (not written to compacted SSTable)
- [x] All tests updated; 6 tests renamed to reflect new tombstone-storage semantics
- [x] Resurrection bug fixed: set "a" → flush → delete "a" → get "a" returns None

## Phase 5 — Network layer (COMPLETE except integration test assertions)

### Completed
- [x] Restructured `src/` into `src/lib/` (library crate) and `src/bin/` (REPL + server binaries)
- [x] Updated visibility: `pub(crate)` → `pub` on `Log`, `Entry`, `Command`, `CommandError`, constants; re-exported via `log/mod.rs`
- [x] `Runner<R, W>` in `src/lib/run.rs` — generic over `BufRead + Write`; owns the read/write loop; `Log` passed in per `run()` call
- [x] `Execute` trait updated — `execute(&mut self, command, writer: &mut impl Write)` — responses written to generic writer, not stdout
- [x] `src/bin/repl.rs` — `Runner::new(BufReader<Stdin>, Stdout)`, calls `runner.run(&mut log)`; tui welcome stays in bin
- [x] `src/bin/server.rs` — `TcpListener::bind`, `listener.incoming()` loop; `Runner::new(BufReader<TcpStream>, BufWriter<TcpStream>)` per connection; single shared `Log`
- [x] `TcpStream::try_clone()` used to split stream into reader + writer halves
- [x] `writeln!` + `writer.flush()` after each response — correct for both stdout and TCP
- [x] `unfallible_get` removed — `Runner::run()` inlines the read/parse/respond loop, handles errors by writing `ERR` to writer
- [x] Integration test stubs in `tests/server.rs` — `start_server()` helper binds on port 0, spawns background thread; `send()` helper writes command and reads response; assertions to be filled in

### Current test count: 92 passing, 0 ignored

#### Test coverage by module
- `log::entry` — 4 tests (key/value accessors for Set and Delete)
- `log::header` — 11 tests (round-trips, has_at_least_one, corruption variants)
- `log::wal::memtable` — 19 tests (from_file, process, flush, bloom filter)
- `log::sstable::compact` — 9 tests (includes tombstone drop, overlapping keys, single-file case)
- `log::command` — 32 tests (parser aliases/errors, Execute trait paths)
- `log::tests` — 15 tests (includes resurrection regression, bloom filter integration, tombstone in memtable)
