## 1. Fix Naming Inconsistencies in market_data

- [x] 1.1 Rename `FuturesStreamActor` trait implementation and references in `crates/market_data/src/services/futures_stream_service.rs`.
- [x] 1.2 Rename `PublicStreamActor` trait implementation and references in `crates/market_data/src/services/public_stream_service.rs`.
- [x] 1.3 Rename `UserStreamActor` trait implementation and references in `crates/market_data/src/services/user_stream_service.rs`.
- [x] 1.4 Add `///` doc comments with complexity analysis (O(N) for stream processing) and ensure no `unwrap()` in WebSocket loops for all renamed actors.
- [x] 1.5 Validation: Run `cargo check -p market_data` to ensure the crate compiles in isolation.

## 2. Clean up and Fix executor main.rs

- [x] 2.1 Remove stale imports for `ForceOrderService`, `MarkPriceService`, and `OpenInterestService` in `crates/executor/src/main.rs`.
- [x] 2.2 Correctly import the renamed consolidated actors (`PublicStreamActor`, `FuturesStreamActor`, `UserStreamActor`) from `market_data`.
- [x] 2.3 Update `supervisor.register_actor` calls to use the correct struct names and factory closures.
- [x] 2.4 Validation: Run `cargo check -p executor` to verify that the entry point correctly integrates with the `market_data` actors.

## 3. System-wide Verification

- [x] 3.1 Validation: Run `cargo build` for the entire workspace to ensure all dependencies and cross-crate links are functional.
- [x] 3.2 Validation: Run existing integration tests (if any) or a short "dry run" with the mock server to confirm actors start and heartbeats are received by the supervisor.
