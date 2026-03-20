# Todo

## Phase 3 — Binary serialization (COMPLETE)

Header format, CRC32 checksums, corruption recovery, and doc comments all done.
See `src/log/header/` for on-disk format.

## Phase 4 — SSTable / LSM-tree (COMPLETE)

Steps 1–6 all done. See `context/progress.md` for full detail.
89 tests passing, 0 ignored.

### Remaining polish (optional)
- [ ] `compact` REPL command — expose compaction manually alongside the auto-trigger every 10 flushes
- [ ] `scan <start> <end>` REPL command — range query over memtable + SSTables using sorted order

### Step 4.5 — Leveled compaction (optional, after Step 6)
- [ ] Organize SSTables into levels (L0, L1, L2...) — L0 accepts direct flushes, L1+ enforce non-overlapping key ranges
- [ ] Compact L0 → L1 when L0 file count hits threshold (e.g. 4)
- [ ] Each level is 10x larger than the previous — controls read/write amplification tradeoff

## Phase 5 — Network layer

### Backburner (do after Phase 5 if desired)
- [ ] `compact` REPL command — manual compaction trigger
- [ ] `scan <start> <end>` REPL command — range query using sorted BTreeMap order
- [ ] Leveled compaction — L0 flush target, L1+ non-overlapping key ranges, 10x size ratio per level

---

### Learning regimen — complete before writing any Phase 5 code

Work through these in order. Each builds on the last.

#### 1. TCP fundamentals in Rust (std::net)
**Goal:** understand how a TCP server accepts and talks to clients using only the standard library.
- Read: Rust Book Chapter 20 — "Final Project: Building a Multithreaded Web Server"
  — focus on the TCP parts (ignore the HTTP/thread pool parts for now, those come in Phase 6)
- Key types to understand: `TcpListener`, `TcpStream`, `TcpListener::bind()`, `listener.incoming()`
- Key insight: `TcpStream` implements `Read + Write` — you already know these traits from file I/O.
  Reading from a socket and reading from a file are the same interface.
- Exercise: write a toy echo server (read bytes from stream, write them back) before touching nighthawk

#### 2. Framing — how do you know where one message ends?
**Goal:** understand why raw TCP streams need framing and know the two main approaches.
- Key insight: TCP is a *stream* of bytes, not a stream of messages. If a client sends
  "SET a 1" and "GET a" back to back, the server might receive them as one blob. You need
  framing to know where one command ends and the next begins.
- **Approach A — length-prefixed framing**: every message is preceded by a fixed-size integer
  (e.g. 4 bytes) saying how many bytes follow. Server reads 4 bytes, then reads exactly that many.
  Simple, binary-friendly, easy to implement.
- **Approach B — delimiter framing**: messages end with a sentinel (e.g. `\n`). Server reads
  until it hits the sentinel. Simpler to debug (human-readable), but breaks if payload contains
  the sentinel. Redis uses this.
- Recommendation for nighthawk: **length-prefixed** — consistent with the binary header format
  already in place; no sentinel collision risk; already familiar from `entry_len` in the header.
- Read: https://docs.rs/tokio/latest/tokio/io/index.html — the framing section (conceptual,
  don't need to use tokio yet)

#### 3. RESP — study Redis's wire protocol as a reference
**Goal:** see a real, simple, well-documented wire protocol before designing your own.
- Read: https://redis.io/docs/reference/protocol-spec/ — RESP2 only, skip RESP3
- RESP is delimiter-framed (`\r\n`) with type prefixes (`+` = simple string, `-` = error,
  `:` = integer, `$` = bulk string, `*` = array)
- Example: `SET foo bar` sent as `*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n`
- You don't need to implement RESP — just read it to understand what a real protocol looks like
  and what decisions it made (type tagging, error vs success framing, null bulk strings)
- Key takeaway: a protocol is just a contract. Define it clearly before writing code.

#### 4. BufReader / BufWriter on TcpStream
**Goal:** understand why you must wrap TcpStream in buffered I/O.
- A raw `TcpStream::read()` may return 1 byte at a time (syscall per byte = expensive).
  `BufReader` batches reads into a buffer, dramatically reducing syscalls.
- `BufReader<TcpStream>` gives you `read_line()`, `read_exact()` etc. — same as file I/O.
- Important: you cannot wrap a single `TcpStream` in both `BufReader` and `BufWriter` because
  that would require two mutable borrows. Fix: `stream.try_clone()` to get a second handle,
  wrap one in `BufReader` and one in `BufWriter`. Or use a crate like `std::io::split` equivalent.
- Read: https://doc.rust-lang.org/std/io/struct.BufReader.html

#### 5. Design the nighthawk wire protocol
**Goal:** write down the protocol *before* coding it. Keep it simple.
- Recommended design — length-prefixed binary:
  ```
  Request:  [command: 1B][key_len: 4B][key: key_len bytes][val_len: 4B][val: val_len bytes]
  Response: [status: 1B][val_len: 4B][val: val_len bytes]
  ```
  - command byte: `0x01` = SET, `0x02` = GET, `0x03` = DELETE
  - status byte: `0x00` = OK, `0x01` = NOT_FOUND, `0x02` = ERROR
  - val_len = 0 and no val bytes for GET-not-found, DELETE-ok, SET-ok responses
- Alternative: newline-delimited text protocol (like Redis RESP but simpler):
  ```
  SET key value\n  →  OK\n
  GET key\n        →  value\n  or  NIL\n
  DEL key\n        →  OK\n     or  NOT_FOUND\n
  ```
  Easier to test with `nc` / `telnet`, but less consistent with existing binary format.
- **Decision: text protocol chosen.** Newline-delimited, human-readable. Easy to test with `nc`.
  Commands map directly onto the existing `Command` enum (`TryFrom<&str>` reused as-is).
  ```
  SET key value\n  →  OK\n
  GET key\n        →  value\n  or  NIL\n
  DEL key\n        →  OK\n     or  NOT_FOUND\n
  ERR message\n   (server-side errors)
  ```

#### 6. Connection handling pattern (sync, one thread per connection)
**Goal:** understand the basic sync server loop before adding concurrency in Phase 6.
- Pattern:
  ```rust
  let listener = TcpListener::bind("127.0.0.1:6379")?;
  for stream in listener.incoming() {
      let stream = stream?;
      handle_connection(stream, &log); // blocking — one client at a time for now
  }
  ```
- Phase 5 is deliberately single-threaded / single-client. Phase 6 adds `thread::spawn` or tokio.
- `handle_connection` reads one request, dispatches to `Log::get/write/delete`, writes response, loops.
- The `Log` struct will need to be shareable across threads in Phase 6 — worth keeping that in
  mind during Phase 5 design but don't over-engineer it yet.

#### 7. Testing a TCP server
**Goal:** know how to test your server without building a full client first.
- `nc` (netcat) — send raw bytes/text to your server from the terminal: `echo "GET foo" | nc localhost 6379`
- `telnet localhost 6379` — interactive text session
- Integration tests: spawn the server in a background thread, connect with `TcpStream::connect()`,
  send bytes, assert on the response. Same `temp_log()` pattern you already use.

---

### Phase 5 build plan

**Protocol decision: newline-delimited text.** No client binary planned — protocol is the contract,
clients are free to be anything (`nc`, custom, etc.). Server is a second binary alongside the
existing REPL; `Command::TryFrom<&str>` parsing is reused as-is.

- [x] Decide: text wire protocol (newline-delimited)
- [x] `src/bin/server.rs` — `TcpListener` loop, `Runner` per connection
- [x] Request parsing — `BufReader::read_line()` → `Command::try_from(&str)` (reused as-is)
- [x] Response serialisation — `writeln!` to `BufWriter<TcpStream>`, flush after each response
- [x] Wire up `Log` via `Execute` trait inside `Runner::run()`
- [x] `Runner<R, W>` — generic over `BufRead + Write`, shared by REPL and server
- [x] `src/bin/repl.rs` — uses `Runner` with `BufReader<Stdin> + Stdout`
- [ ] Integration tests — stubs written in `tests/server.rs`, assertions to be filled in

## Phase 5.5 — Configuration

- [ ] `.env` file loading — read `BIND_ADDRESS` and `BIND_PORT` at server startup (use `dotenvy` crate)
- [ ] Fall back to defaults (`127.0.0.1:3000`) if `.env` absent or values missing
- [ ] Pass resolved bind address into `TcpListener::bind()` in `server.rs`

## Phase 6 — Concurrency

- [ ] `RwLock` for concurrent readers, single writer
- [ ] Connection handling with tokio or std threads
- [ ] Explore MVCC if ambitious

## Architecture notes

SSTable file layout:
```
[entry 0 with header][entry 1 with header]...[bloom_filter bytes][bit_count: 4B u32 LE]
```

Entry header format:
```
[magic: 2 bytes (0x4E48 "NH")][crc32: 4 bytes][entry_len: 4 bytes][wincode-serialized Entry]
```

Key files:
- `src/lib/log/entry.rs` — `Entry` enum (Set/Delete) — single type used by all layers
- `src/lib/log/header/` — `HeaderWriter`, `HeaderReader`, `HeaderSerializer`, `HeaderDeserializer`, `CorruptionType`
- `src/lib/log/mod.rs` — `Log` struct: `write`, `get`, `contains`, `flush`, `maybe_flush`
- `src/lib/log/memtable.rs` — `MemTable` wrapping `BTreeMap<String, Entry>`, `process()`, `flush_to()`, `should_flush()`
- `src/lib/log/sstable/mod.rs` — `SSTable` struct: bloom filter, boundary position, entry iteration
- `src/lib/log/sstable/bloom_filter.rs` — `BloomFilter`, `BloomFilterReader`
- `src/lib/log/sstable/compact.rs` — `Log::compact()` k-way merge; tombstone winners dropped
- `src/lib/log/command.rs` — `Execute` trait on `Log`, command dispatch, `writeln!` responses
- `src/lib/run.rs` — `Runner<R, W>` generic over `BufRead + Write`; shared by REPL and server
- `src/bin/repl.rs` — REPL entry point; `Runner` with stdin/stdout
- `src/bin/server.rs` — TCP server entry point; `TcpListener` loop, `Runner` per connection
- `tests/server.rs` — integration test stubs for TCP server (assertions pending)

## Study list

- ~~`std::io::Seek`, `SeekFrom`, `stream_position()`~~ — learned in Phase 1
- ~~Bitcask paper~~ — read, using as model for Phase 2
- ~~`std::fs::metadata().len()`~~ — learned for size-based merge triggering
- ~~`std::fs::File::sync_all()`~~ — learned and implemented in Phase 2
- ~~CRC32 checksums (`crc32fast` crate)~~ — learned and implemented in Phase 3
- ~~`u32::to_le_bytes()` / `u32::from_le_bytes()`~~ — learned in Phase 3
- ~~`BTreeMap`~~ — sorted in-memory structure, understood as ordered map for memtable
- ~~`std::io::BufWriter`~~ — learned and used in merge for batched writes
- ~~SSTable format — sorted string table, on-disk sorted key-value segments~~ — learned and implemented
- ~~LSM-tree architecture — how memtable flushes, levels, and compaction fit together~~ — learned and implemented
- ~~Sorted merge (k-way merge) — merging multiple sorted SSTable files into one~~ — learned and implemented
- ~~Bloom filters — probabilistic data structure for fast negative lookups~~ — learned and implemented: k=7 hashes, 10 bits/key, double-hashing, ~1% FP rate, xxh3
- ~~Tombstone propagation in LSM-trees — how deletes must flow through SSTable levels to avoid resurrection; compaction as the suppression point~~ — learned and implemented
- ~~TCP framing and wire protocols — newline-delimited text chosen; `BufReader::read_line()` for framing, `writeln!` + `flush()` for responses; `TcpStream::try_clone()` to split into reader/writer halves~~ — learned and implemented in Phase 5
- Rust trait objects vs generics for shared serialization — relevant for future type design
- Sparse index / index block — how SSTables avoid indexing every key (binary search between index points)
- `tokio` async runtime basics — needed for Phase 6 connection handling
- `Arc<Mutex<Log>>` — needed for Phase 6 to share Log across threads
