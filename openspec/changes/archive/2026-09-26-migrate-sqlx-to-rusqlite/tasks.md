# Tasks

## 1. Dependencies and Configuration

- [x] 1.1 Add `rusqlite` with `bundled` feature to `Cargo.toml` workspace dependencies and `crates/storage/Cargo.toml`, remove `sqlx`, and clean up `common/src/logger.rs`. Verify by running `cargo check -p common`.
- [x] 1.2 Validate dependency build and target compilation across the workspace. Verify by running `cargo check -p storage`.

## 2. Dedicated Writer Thread & Channel Protocol

- [x] 2.1 Implement `DbWriteBatch` enum, channel protocol, and `DbWriter` background worker thread in `crates/storage/src/db.rs` with bare-metal PRAGMA configuration (`synchronous = OFF`, `journal_mode = WAL`, `mmap_size = 1GB`, `cache_size = -64000`, `temp_store = MEMORY`, `locking_mode = EXCLUSIVE`) and ISO weekly rotation. Verify by running `cargo check -p storage`.
- [x] 2.2 Validate `DbWriter` initialization, PRAGMA verification, and weekly database file rotation with unit tests. Verify by running `cargo test -p storage --lib db`.

## 3. Repositories Migration to Rusqlite

- [x] 3.1 Refactor repository implementations (`aggtrade_repo`, `orderbook_repo`, `klines_repo`, `markprice_repo`, `openinterest_repo`, `forceorder_repo`) in `crates/storage/src/repositories/` to execute batch inserts via `tx.prepare_cached()` in explicit transactions. Verify by running `cargo check -p storage`.
- [x] 3.2 Validate batch inserts and parameter bindings across all six repositories using in-memory SQLite connections. Verify by running `cargo test -p storage --lib repositories`.

## 4. Storage Actor & Dual-Trigger Buffering

- [x] 4.1 Refactor `crates/storage/src/actors/storage_actor.rs` to maintain typed in-memory buffers with dual-trigger flushing (capacity threshold of 5,000 items or 1.0-second interval ticker), dispatching `DbWriteBatch` vectors over bounded `mpsc` channels without `tokio::spawn` micro-tasks. Verify by running `cargo check -p storage`.
- [x] 4.2 Validate `StorageActor` dual-trigger flushing (capacity and timeout triggers) and graceful channel shutdown with integration tests. Verify by running `cargo test -p storage --test storage_actor_broadcast`.

## 5. System Integration & End-to-End Verification

- [x] 5.1 Update `crates/executor/src/main.rs` to spawn the `DbWriter` thread and instantiate `StorageActor` with the new channel architecture. Verify by running `cargo check -p executor`.
- [x] 5.2 Validate workspace compilation, zero-warning lints, and ensure no `unwrap()` or `panic!()` in the ingestion and persistence pathways. Verify by running `cargo clippy --workspace --all-targets -- -D warnings` and `cargo test -p storage`.
