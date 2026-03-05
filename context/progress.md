# Progress

## Phase 1 — Append-only log with in-memory index

- [x] REPL loop — stdin command parsing (get, set, delete) with validation
- [x] Binary serialization with wincode for log entries as (String, String) tuples
- [x] Append-only writes — set appends to data.log, records byte offset in HashMap
- [x] Seek-based reads — get looks up offset in index, seeks to position, deserializes
- [x] Index rebuild on startup — scans log file front-to-back, rebuilds HashMap<String, u64>
- [x] Delete — removes key from in-memory index
- [ ] Delete persistence — tombstone entries so deletes survive restart
