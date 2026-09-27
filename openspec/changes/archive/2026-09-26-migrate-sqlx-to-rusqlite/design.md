# Design: Dedicated Worker Rusqlite Persistence Architecture

## Context

The persistence layer in `crates/storage` records six market event streams (`agg_trades`, `order_books`, `klines_1s`, `funding_rates`, `open_interest`, `liquidations`) to weekly partitioned SQLite databases. Currently, `sqlx` introduces threadpool and channel overhead, while `StorageActor` spawns a Tokio task per market event, degrading performance on the 8-core RISC-V Orange Pi RV2. 

Because SQLite enforces single-writer serialization even in WAL mode, this design introduces a dedicated single-threaded writer architecture (Approach B) using `rusqlite` with zero mutex contention and aggressive bare-metal PRAGMA optimizations. See `proposal.md` for background and motivation.

```
+--------------------------------------------------------------------------+
|                           Tokio Async Runtime                            |
|                                                                          |
|  [Market Broadcast Bus]                                                  |
|           |                                                              |
|           v (Arc<MarketEvent>)                                           |
|  +--------------------------------------------------------------------+  |
|  | StorageActor (Single Actor Task)                                   |  |
|  |   - In-memory table buffers (Vec<T>)                               |  |
|  |   - Interval ticker (1.0s)                                         |  |
|  |   - Flush trigger: len >= CAPACITY || ticker.tick()                |  |
|  +--------------------------------------------------------------------+  |
|           |                                                              |
|           | mpsc::Sender<DbWriteBatch> (Transfer Vec ownership)          |
+-----------|--------------------------------------------------------------+
            |
            v
+--------------------------------------------------------------------------+
|                       Dedicated OS Background Thread                     |
|                                                                          |
|  +--------------------------------------------------------------------+  |
|  | DbWriter Worker Loop                                               |  |
|  |   - Exclusively owns rusqlite::Connection (No Arc/Mutex)           |  |
|  |   - Checks ISO week rotation                                       |  |
|  |   - Opens explicit transaction: BEGIN                              |  |
|  |   - Executes cached prepared statements in tight loops             |  |
|  |   - Commits: COMMIT                                                |  |
|  +--------------------------------------------------------------------+  |
|           |                                                              |
|           v (Direct OS Page Cache / MMAP / synchronous=OFF)              |
|  [NVMe SSD: crypto_YYYY_WW.db]                                           |
+--------------------------------------------------------------------------+
```

## Goals / Non-Goals

**Goals:**
* **Zero Tokio Runtime Starvation**: Completely eliminate `tokio::spawn` micro-tasks from the market event storage path.
* **Zero Lock Contention**: Pin SQLite connection ownership to a single background OS thread, eliminating `Arc<Mutex<Connection>>` and `SQLITE_BUSY` races.
* **Dual-Trigger Batching**: Buffer records in RAM and commit in bulk (e.g. 5,000–10,000 records or 1.0 second timeout).
* **Bare-Metal PRAGMA Tuning**: Run SQLite at near-RAM speed using `synchronous = OFF`, 1GB memory mapping, and exclusive file locking.
* **Seamless Weekly Rotation**: Automatically transition database files on ISO week boundaries and notify the supervisor for backup archival.

**Non-Goals:**
* Modifying SQLite schemas or database column layouts (schemas remain 100% compatible with existing training pipelines).
* Adding read queries or connection pools (SQLite remains an append-only time-series data sink).
* Modifying the supervisor actor model or backup script execution logic.

## Decisions

### 1. Dedicated OS Background Thread vs. `tokio::task::spawn_blocking`
* **Choice**: Dedicated OS thread spawned via `std::thread::Builder::new().name("db-writer")` listening on a bounded `tokio::sync::mpsc::Receiver`.
* **Rationale**: SQLite allows only one writer at a time. Spawning blocking tasks across Tokio's threadpool introduces scheduling latency and requires synchronization over a shared connection mutex. A dedicated OS thread keeps the `rusqlite::Connection` and statement bytecode permanently cached in the CPU L1/L2 cache of a single RISC-V core.
* **Alternatives Considered**: `tokio::task::spawn_blocking` with `Arc<Mutex<Connection>>`. Rejected due to threadpool queue latency, mutex contention between distinct table buffers, and cache thrashing.

### 2. Batch Execution: Cached Prepared Statements in Explicit Transactions
* **Choice**: Use `tx.prepare_cached(...)` and execute parameterized statements in tight loops within a single transaction:
  ```rust
  let mut tx = conn.transaction()?;
  {
      let mut stmt = tx.prepare_cached(
          "INSERT INTO agg_trades (time, symbol_id, price, quantity, is_buyer_maker) VALUES (?, ?, ?, ?, ?)"
      )?;
      for item in batch {
          stmt.execute(rusqlite::params![item.time, item.symbol.0, item.price.0, item.quantity.0, item.is_buyer_maker])?;
      }
  }
  tx.commit()?;
  ```
* **Rationale**: Stepping pre-compiled bytecode in memory takes 1–2µs per record in C/Rust without parsing or allocating query strings, and completely avoids SQLite's variable number limit (`SQLITE_MAX_VARIABLE_NUMBER`).
* **Alternatives Considered**: Dynamic multi-row SQL string generation (`INSERT INTO t VALUES (...), (...)`). Rejected due to string allocation overhead, re-parsing penalty, and variable binding limits.

### 3. Dual-Trigger Buffer Management in `StorageActor`
* **Choice**: `StorageActor` manages typed in-memory vectors directly (e.g. `Vec<AggTradeInsert>`). It flushes a batch when:
  1. `buffer.len() >= BUFFER_CAPACITY` (e.g., 5,000 items), or
  2. A 1.0-second interval timer fires for any non-empty buffer.
* **Rationale**: Moving full `Vec<T>` ownership down an MPSC channel involves zero memory cloning. Dual-trigger ensures high-throughput tables flush efficiently at scale while low-frequency tables (e.g., `liquidations`) do not linger in memory for hours.
* **Alternatives Considered**: Immediate per-event writes (rejected due to disk I/O amplification) or pure count-based buffering (rejected due to stale data risk on rare events).

### 4. PRAGMA Configuration for Orange Pi RV2
* **Choice**:
  * `PRAGMA synchronous = OFF;` - Disables `fsync` barriers, delegating disk write-back to the Linux kernel page cache.
  * `PRAGMA journal_mode = WAL;` - Sequential write-ahead logging.
  * `PRAGMA mmap_size = 1073741824;` - 1GB memory-mapped I/O.
  * `PRAGMA cache_size = -64000;` - 64MB RAM page cache.
  * `PRAGMA temp_store = MEMORY;` - Memory storage for temporary structures.
  * `PRAGMA locking_mode = EXCLUSIVE;` - Holds database lock across transactions, saving lock/unlock system calls.
  * `PRAGMA wal_autocheckpoint = 10000;` - Defers WAL checkpoints to reduce I/O spikes.
* **Rationale**: The user has external power-loss protection (UPS/battery). This setup yields near-RAM persistence throughput on NVMe SSD storage.

### 5. Rotation and Connection Management
* **Choice**: The `DbWriter` maintains the active ISO year/week packed integer. Before processing batches, it checks if `current_iso_packed() != active_iso_packed()`. If changed:
  1. Any uncommitted transaction is finalized.
  2. The current connection is closed.
  3. A new connection to `crypto_<YEAR>_<WEEK>.db` is initialized with PRAGMAs and schema.
  4. A `ControlMessage::Spawn(Box::new(BackupOneShotActor::new()))` message is sent to the supervisor.

## Risks / Trade-offs

* **[Risk: Channel Backpressure during I/O Spikes]** $\rightarrow$ **Mitigation**: Use a bounded channel (e.g., 256 batches). If the channel fills, `StorageActor` logs a warning and drops or yields, protecting the Tokio runtime from unbounded memory growth.
* **[Risk: Uncommitted Memory Loss on Hard Crash]** $\rightarrow$ **Mitigation**: With `synchronous = OFF` and a 1.0-second timeout, up to 1 second of buffered market data could be lost on abrupt kernel crash. Accepted by user as power cuts are handled externally and data is historical/time-series.
* **[Risk: Prepared Statement Invalidation on Rotation]** $\rightarrow$ **Mitigation**: Prepared statements are tied to `Connection`. When rotating database files, statements are re-prepared on the new connection.

## Migration Plan

1. **Cargo Configuration**: Add `rusqlite` (bundled feature) to root and crate `Cargo.toml`. Remove `sqlx`.
2. **DbWriter Implementation**: Create `crates/storage/src/db.rs` with `DbWriter` worker loop, channel protocol (`DbWriteBatch`), and connection lifecycle.
3. **Repository Refactoring**: Update repositories in `crates/storage/src/repositories/` to execute `rusqlite` transactions using cached prepared statements.
4. **StorageActor Refactoring**: Refactor `crates/storage/src/actors/storage_actor.rs` to aggregate events into local buffers and dispatch batches via `mpsc`.
5. **Supervisor & Logger Cleanup**: Remove `sqlx` directives in `crates/common/src/logger.rs` and update actor initialization in `crates/executor/src/main.rs`.
6. **Verification**: Run unit and integration tests (`cargo test -p storage`) and benchmark write latency under synthetic event loads.
