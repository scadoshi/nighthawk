# Progress

## Phase 1 — Append-only log with in-memory index (COMPLETE)

- [x] REPL loop — stdin command parsing (get, set, delete) with validation
- [x] Binary serialization with wincode via Entry enum (SchemaRead/SchemaWrite derives)
- [x] Append-only writes — set appends to data.log, records byte offset in HashMap
- [x] Seek-based reads — get looks up offset in index, seeks to position, deserializes
- [x] Index rebuild on startup — scans log file front-to-back using `@` pattern binding
- [x] Delete — removes key from in-memory index
- [x] Delete persistence — tombstone Entry::Delete variant appended to log, survives restart
