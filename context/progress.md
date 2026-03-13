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
- [x] File handle reopen after merge via direct assignment (`self.file = ...`)
- [x] Size-based merge triggering — checks `megabytes()` after each command
- [x] `sync_all()` after every write (set/delete) for durability
- [x] `sync_all()` on temp file before rename in merge
- [x] Crash-safe merge — removed `remove_file`, `rename` atomically overwrites (POSIX)
- [x] Partial/corrupt entry handling — `Err(_) => break` in scan, corruption on read returns error
- [x] Quit command with clean loop exit (no `process::exit`)
- [x] Command shorthand aliases (s/g/d/q) and variable-arity parsing

## Phase 3 — Binary serialization (COMPLETE)

- [x] Added `crc32fast` dependency
- [x] Defined header format: `[magic: 2 bytes][crc32: 4 bytes][entry_len: 4 bytes]` — 10 bytes total
- [x] Magic bytes: `0x4E48` ("NH") as u16
- [x] New `src/log/header.rs` module — `Header` trait implemented on `File`
- [x] `write_entry_with_header` — serializes entry with wincode, writes magic + CRC32 + len + entry bytes, returns offset
- [x] `parse_entry(&[u8])` — standalone function for parsing header + entry from a byte slice, no I/O
- [x] `read_next_entry_with_header` — reads file once, scans in-memory byte-by-byte for next valid entry (corruption recovery)
- [x] Bounds checking — guards against partial writes (not enough bytes for header or entry data)
- [x] `CorruptionType` error enum — `NotEnoughBytes`, `MagicBytesMismatch`, `ChecksumMismatch`, `EntryParseError`
- [x] Updated `Log::write` and `Log::read_next` to delegate to header trait methods
- [x] Updated `Execute` impl — Set/Delete use `Log::write`, Get uses `Log::read_next`
- [x] Updated index rebuild (`from_file`) to use header-aware reading
- [x] Updated merge to use `read_next` for reading and `Log::write` for writing (headers on both sides)
- [x] `Log::new` takes `path` and `truncate` params — used by merge to create clean temp files
- [x] Learned `u32::to_le_bytes()` / `u32::from_le_bytes()` — little-endian byte encoding for header fields
- [x] Learned endianness — LE stores least significant byte at lowest address, convention for on-disk formats (x86/ARM native)

## Tests (COMPLETE)

- [x] `Entry::k()` and `Entry::v()` — 4 unit tests in `src/log/entry.rs`
- [x] Refactored `parse_entry` standalone fn into `TryIntoEntryWithLen` trait on `[u8]`
- [x] Extracted serialization into `EntryWithHeader` trait on `Entry` — `write_entry_with_header` now delegates to `try_into_bytes_with_header`
- [x] `TryIntoEntryWithLen` byte parsing — 9 tests: Set/Delete ok paths, NotEnoughBytes, MagicBytesMismatch, ChecksumMismatch, EntryParseError for both variants
- [x] `CorruptionType` updated: removed `HeaderNotFound`, added `NotEnoughBytes`, renamed `MagicBytesNotFound` to `MagicBytesMismatch`
- [x] Command parser (`TryFrom<&str>`) — 21 tests: every alias for set/get/delete/quit/help, plus MissingRequiredArguments, TooManyArguments, UnrecognizedCommand
- [x] File I/O round-trips via `tempfile` — write then read for both Set and Delete entries
- [x] Corruption recovery — write garbage bytes before a valid entry, assert reader scans past and finds it
- [x] Index rebuild (`from_file`) — 6 tests: empty, single set, set+delete, overwrite keeps latest offset, multiple keys, delete nonexistent key
- [x] `Log` integration tests — 14 tests: new creates empty index, write set/delete, read round-trip, overwrite updates offset, empty read returns None, megabytes, index rebuild on reopen, merge deduplication/delete handling/file shrink/empty/offset validity
- [x] `Execute` integration tests — 9 tests: set/get/delete with existing and missing keys, delete tombstone persistence, overwrite updates value, quit/help no-ops
- [x] Fixed delete-before-write bug — index removal now happens after tombstone write succeeds
- [x] Fixed `NotEnoughBytes` error for truncated entries (was `EntryParseError`)
- [x] Fixed `EntryParseError` tests to use correctly-sized garbage payloads
- [x] Refactored command parser from `if/else if` chain to `match` on `&str`
- [x] Added `tempfile` as dev-dependency for file I/O tests
- [x] Split `Header` into `HeaderWriter` (Write+Seek) and `HeaderReader` (Read+Seek) for generic I/O
- [x] Refactored write as thin file layer (append + sync, returns offset), index updates moved to Execute
- [x] BufWriter in merge — N entries buffered, single flush + sync before rename
- [x] Merge reopens via `Log::new` after rename to rebuild index from compacted file
- [x] Updated tests: write_returns_offset, write_does_not_modify_index, write_then_read_returns_entry
- [x] Refactored `Index` from trait on `HashMap` to struct with `Deref`/`DerefMut` — `inner` convention, `entry_count` tracking
- [x] Ratio-based merge triggering — `should_merge()` checks `entry_count / unique_keys > 2`, replacing flat 10MB `megabytes()` check
- [x] `track_write()` increments `entry_count` after each disk write, called from `Execute`
- [x] Index tests: `entry_count` includes all entries, `track_write` increments, `should_merge` empty/low/boundary/exceeds
- [x] 72 tests total, all passing
