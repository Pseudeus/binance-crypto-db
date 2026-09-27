# Proposal: Migrate Storage Layer from sqlx to rusqlite

## Why

The current SQLite storage layer relies on `sqlx`, an async SQLite wrapper that dispatches operations across an internal worker thread pool with dynamic query building, and a storage actor that spawns a Tokio task for every incoming market event. On the Orange Pi RV2 (RISC-V 8-core, 8GB LPDDR4), this task churn and async runtime contention wastes CPU cycles and risks scheduler starvation on the trading hot path. 

Migrating to `rusqlite` using a dedicated single-threaded writer thread and bounded batch channels provides bare-metal control over SQLite, unlocks aggressive PRAGMA tuning (`synchronous = OFF`, 1GB `mmap`, 64MB page cache), and increases in-memory batch buffers, delivering maximum write throughput with minimal disk I/O and zero Tokio scheduler interference.

## What Changes

* **Dependency Migration**: Replace `sqlx` with `rusqlite` (bundled / linked via `libsqlite3-sys`) across `crates/storage` and the root `Cargo.toml`. Remove `sqlx` directives from `common/src/logger.rs`.
* **Dedicated Writer Architecture (Approach B)**: Introduce a dedicated single-thread DB worker that exclusively owns the `rusqlite::Connection`. Async actors communicate via bounded `mpsc` channels passing pre-allocated batch vectors, eliminating all Mutex locking, connection pooling, and cross-thread lock contention.
* **Prepared Statement Batching**: Replace dynamic multi-row SQL construction (`sqlx::QueryBuilder`) in all repositories (`aggtrade_repo`, `orderbook_repo`, `klines_repo`, `markprice_repo`, `openinterest_repo`, `forceorder_repo`) with reusable, cached prepared statements executed inside explicit transactions.
* **Dual-Trigger Buffer Flush**: Update `StorageWriteBuffer` to flush batches based on count (e.g., 5,000–10,000 records) or time interval (e.g., 1.0 second), preventing low-frequency streams (liquidations, funding rates) from stalling in memory.
* **Bare-Metal PRAGMA Tuning**: Configure SQLite for maximum write performance and minimal disk I/O:
  * `PRAGMA synchronous = OFF;` (leveraging external UPS/power-cut management to bypass disk `fsync` bottlenecks).
  * `PRAGMA journal_mode = WAL;`
  * `PRAGMA mmap_size = 1073741824;` (1GB memory-mapped I/O).
  * `PRAGMA cache_size = -64000;` (64MB dedicated RAM page cache).
  * `PRAGMA temp_store = MEMORY;`
  * `PRAGMA locking_mode = EXCLUSIVE;`
  * `PRAGMA wal_autocheckpoint = 10000;`
* **Rotation Management**: Retain ISO weekly database rotation (`crypto_YYYY_WW.db`) and supervisor-triggered backup snapshots, managed synchronously on the dedicated writer thread.
* **BREAKING**: Replaces public `storage::db::RotatingPool` and `storage::repositories::Repository` traits with `rusqlite`-native interfaces.

## Capabilities

### New Capabilities
- `sqlite-storage`: Dedicated single-threaded SQLite persistence layer with bare-metal `rusqlite` control, batching, PRAGMA tuning, and weekly file rotation.

### Modified Capabilities
<!-- None: existing functional specifications (e.g. strategy, discovery, telegram) have no requirement changes -->

## Latency Impact & Systems Analysis

* **Zero-Copy vs. Cloning**: Incoming `Arc<MarketEvent>` items broadcast from market data actors are cloned into in-memory batch buffers. When a buffer flushes, the entire batch `Vec<T>` ownership is transferred into the MPSC channel with zero memory copies.
* **Hot-Path Isolation**: SQLite storage is strictly off the hot path (Ingestion -> Feature Engineering -> Inference -> Execution). Migrating to a dedicated writer completely removes `tokio::spawn` micro-tasks from `StorageActor`, ensuring zero scheduler interference with `StrategyService`.
* **RISC-V Hardware Footprint (Orange Pi RV2)**:
  * **CPU**: Pinned execution on a dedicated OS thread keeps SQLite bytecode, statement handles, and memory pages hot in the L1/L2 cache of a single RISC-V core.
  * **RAM**: 64MB SQLite page cache + 1GB mmap + in-memory batch buffers consume ~1.1GB virtual memory, remaining well within the 8GB LPDDR4 budget.
* **MLOps & Training Data**: SQLite tables and schemas remain 100% identical. Offline `.sql.zst` extraction to the Fedora training node and ONNX input feature vectors are completely unchanged.

## Success Metrics

* **Write Throughput**: Sustained ingestion > 50,000 records/sec during high-volatility market bursts without channel lag.
* **Tokio Runtime Health**: Zero tasks spawned per market event in `StorageActor`, reducing Tokio scheduler overhead to zero for database operations.
* **Disk I/O Reduction**: Elimination of per-transaction `fsync` calls via `synchronous = OFF` and memory-mapped writes.

## Impact

* **Crates Affected**:
  * `crates/storage`: Full internal rewrite of connection management, repositories, buffers, and actors.
  * `crates/common`: Update logger filters to remove `sqlx`.
  * `crates/executor`: Update `main.rs` actor initialization for `StorageActor`.
* **Dependencies**: Remove `sqlx` (0.8.6). Add `rusqlite` (bundled feature). Retain `libsqlite3-sys`.
