# nighthawk

Log-structured key-value store in Rust.

Named after the Common Nighthawk — a bird that persists through change.

## Setup

```
cargo build
cargo run
```

Data is stored in `data.log` in the working directory, created on first run.

## Commands

```
set <key> <value>    (alias: s)
get <key>            (alias: g)
delete <key>         (alias: d, del)
quit                 (alias: q, exit)
```

## How it works

- All writes append to a single log file on disk using binary serialization (wincode)
- Each entry is wrapped in a 10-byte header: `[magic: 2B][crc32: 4B][entry_len: 4B]`
- Magic bytes (`0x4E48` / "NH") mark entry boundaries for corruption recovery
- CRC32 checksums (via `crc32fast`) detect corrupt entries on read
- An in-memory index (HashMap) maps keys to byte offsets for O(1) lookups
- Deletes append tombstone entries and remove the key from the index
- The index is rebuilt by scanning the log on startup, skipping corrupt entries
- Log compaction (merge) runs automatically when the file exceeds 10MB, deduplicating entries
- `sync_all()` is called after every write for durability
