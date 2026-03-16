# nighthawk

Log-structured key-value store in Rust.

Named after the Common Nighthawk — a bird that persists through change.

## Setup

```
cargo build
cargo run
```

Data is stored in `data/` in the working directory, created on first run.

## Commands

```
set <key> <value>    (alias: s)
get <key>            (alias: g)
delete <key>         (alias: d, del)
quit                 (alias: q, exit)
```

## How it works

Nighthawk is an LSM-tree (Log-Structured Merge-tree) key-value store.

**Write path**
- Writes append to a write-ahead log (WAL) at `data/memtable` and update an in-memory `BTreeMap` (the memtable)
- Each entry is wrapped in a 10-byte header: `[magic: 2B][crc32: 4B][entry_len: 4B]`
- Magic bytes (`0x4E48` / "NH") mark entry boundaries for corruption recovery
- CRC32 checksums detect corrupt entries; the reader scans byte-by-byte past bad data
- `sync_all()` is called after every write for durability
- When the memtable exceeds 4MB, it is flushed to a new timestamped SSTable file in `data/sstables/`

**Read path**
- Reads check the memtable first, then scan SSTable files newest-to-oldest
- SSTable filenames are microsecond Unix timestamps, zero-padded — lexicographic order = chronological order

**Compaction**
- Every 10 flushes, all SSTables are merged via a k-way merge
- Files are processed in sorted key order simultaneously; the newest SSTable wins on duplicate keys
- Original SSTables are replaced by the compacted output
