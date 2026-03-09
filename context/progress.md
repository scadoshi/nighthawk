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

## Phase 3 — Binary serialization (IN PROGRESS)

- [x] Added `crc32fast` dependency
- [x] Defined header format: `[magic: 2 bytes][crc32: 4 bytes][entry_len: 4 bytes]` — 10 bytes total
- [x] Magic bytes: `0x4E48` ("NH") as u16
- [x] New `src/log/header.rs` module — `Header` trait implemented on `File`
- [x] `write_entry_with_header` — serializes entry with wincode, writes magic + CRC32 + len + entry bytes, returns offset
- [x] `parse_entry(&[u8])` — standalone function for parsing header + entry from a byte slice, no I/O
- [x] `read_next_entry_with_header` — reads file once, scans in-memory byte-by-byte for next valid entry (corruption recovery)
- [x] Bounds checking — guards against partial writes (not enough bytes for header or entry data)
- [x] `CorruptionType` error enum — distinguishes HeaderNotFound, MagicNotFound, ChecksumNotMatch, EntryParseError
- [x] Updated `Log::write` and `Log::read_next` to delegate to header trait methods
- [x] Updated `Execute` impl — Set/Delete use `Log::write`, Get uses `Log::read_next`
- [x] Updated index rebuild (`from_file`) to use header-aware reading
- [x] Updated merge to use `read_next` for reading and `Log::write` for writing (headers on both sides)
- [x] `Log::new` takes `path` and `truncate` params — used by merge to create clean temp files
- [x] Learned `u32::to_le_bytes()` / `u32::from_le_bytes()` — little-endian byte encoding for header fields
- [x] Learned endianness — LE stores least significant byte at lowest address, convention for on-disk formats (x86/ARM native)
