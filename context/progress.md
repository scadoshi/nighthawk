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
