## Why

To address critical naming mismatches and compilation errors in `market_data` services and clean up outdated imports in `executor/src/main.rs`. This ensures the system correctly uses the "Combined Stream Actors" architecture for more efficient, supervised ingestion.

## What Changes

- **BREAKING**: Rename `PublicStreamActor`, `UserStreamActor`, and `FuturesStreamActor` structs and their `Actor` trait implementations to resolve the `Service` vs `Actor` naming inconsistencies.
- Update `executor/src/main.rs` to correctly import and initialize the consolidated actor structs.
- Remove redundant individual service imports (`ForceOrderService`, `MarkPriceService`, `OpenInterestService`) in `main.rs` that have been replaced by combined streams.
- Ensure all ingestion actors correctly implement the `Actor` trait hooks for supervisor heartbeats and self-healing.

## Capabilities

### New Capabilities
- `public-market-data`: Combined ingestion of AggTrade, Depth, and Klines for Spot markets into a single supervised actor.
- `futures-market-data`: Combined ingestion of MarkPrice and ForceOrder events for Futures markets into a single supervised actor.

### Modified Capabilities
- `user-data-stream`: Update the implementation model to use the consolidated `UserStreamActor` and ensure correct registration with the global supervisor.

## Impact

- **Codebase**: Affects `crates/market_data/src/services/*.rs` and `crates/executor/src/main.rs`.
- **System Architecture**: Strengthens the Actor-based supervision model for high availability.
- **Latency Impact**: No direct impact on data processing latency; maintains zero-copy broadcast efficiency.
- **RISC-V Impact**: Negligible; minor memory savings from reduced actor count.
- **Hot Path**: Corrects the initialization of the primary ingestion pipeline.
- **Success Metrics**: 
  - Zero compilation errors in `market_data` and `executor` crates.
  - Verified heartbeat pulses in the `Supervisor` logs for all three ingestion actors.
  - Continuous, error-free data flow to the `StorageActor`.
