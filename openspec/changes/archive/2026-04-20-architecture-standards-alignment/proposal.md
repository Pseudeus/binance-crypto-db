## Why

The current architecture is accumulating "structural gravity" where `MarketGateway` and `StrategyService` are becoming monolithic. This violates the Single Responsibility Principle, increases testing complexity, and creates unnecessary coupling between ingestion, intelligence, and persistence. Aligning with high-performance standards is critical for the long-term stability of the RISC-V execution node.

## What Changes

- **Decomposed Ingestion**: Split `MarketGateway` into specialized actors (`PublicStreamActor`, `UserStreamActor`) to isolate connectivity risks.
- **Centralized Persistence**: Introduce a `StorageActor` that consumes `MarketEvent` broadcasts, decoupling data processing from SQLite write latency.
- **Pure Feature Engineering**: Extract indicator logic from `StrategyService` into a pure-math module/crate to ensure 100% testability.
- **Type-Safe Finance**: Implement `NewType` patterns for `Price`, `Quantity`, `Bps`, and `Usdt` to eliminate unit-mismatch bugs.
- **Standards Alignment**: Refactor all files exceeding 250 lines (e.g., `MarketGateway`, `StrategyService`) or provide explicit complexity justifications.

## Capabilities

### New Capabilities
- `type-safe-finance`: Implementation of NewType wrappers for all financial units to enforce compile-time safety.
- `decoupled-ingestion`: Specialized ingestion actors for different Binance stream types.
- `event-driven-storage`: A centralized storage actor to handle all persistence needs.

### Modified Capabilities
- `dynamic-position-sizing`: Update the sizing logic to utilize the new `Price` and `Qty` types.

## Latency Impact
- **Positive**: Moving indicator logic to pure functions allows for SIMD-like batching and easier performance profiling. Decoupling DB writes from the strategy loop ensures that SQLite I/O bursts do not delay inference.
- **Neutral**: Switching to `NewType` patterns has zero runtime cost in Rust (zero-cost abstractions).
- **Critical Path**: Refactoring the Ingestion -> Features -> Inference pipeline (the "Hot Path") to ensure zero-copy is maintained despite the actor split.

## Success Metrics
- **Maintainability**: No single file exceeds 250 lines without a technical justification.
- **Safety**: 100% of financial calculations use `NewType` wrappers.
- **Performance**: Maintain <10ms latency from `MarketEvent` receipt to `TradeSignal` emission on the Orange Pi RV2.

## RISC-V Impact Analysis (Orange Pi RV2)
- **CPU**: Increased actor count (threads) is well-supported by the X1-8core CPU.
- **RAM**: 8GB LPDDR4 is sufficient; however, we must ensure `Arc<>` clones are minimized to keep the memory footprint stable.

## Hot Path Impact
- **YES**: This change directly modifies the critical path from WebSocket ingestion to Inference. We must verify that the broadcast channel capacity (10,000) remains sufficient for the multi-actor split.

## ML/ONNX Vector Impact
- **NONE**: The internal feature engineering logic changes, but the input vector to the ONNX model remains `[RSI, OBI, TFI, Volatility]`.
