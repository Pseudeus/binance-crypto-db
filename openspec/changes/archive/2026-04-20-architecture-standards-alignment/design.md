## Context

The current architecture relies on two large components: `MarketGateway` (handling all stream types) and `StrategyService` (mixing indicator logic, risk management, and orchestration). This design refactors these into a layered actor model to improve fault isolation, testability, and performance on the RISC-V execution node.

## Goals / Non-Goals

**Goals:**
- **Decoupled Ingestion**: Separate connectivity concerns for public market data, user account data, and futures-specific data.
- **Async Persistence**: Move all database I/O into a dedicated `StorageActor` to ensure that strategy execution is never blocked by SQLite disk latency.
- **Mathematical Integrity**: Extract indicator calculations into pure, stateless functions using `NewType` wrappers for financial units.
- **Code Standards**: Refactor monolithic files to meet the 250-line limit or provide explicit justifications.

**Non-Goals:**
- **New Features**: This change does not add new trading strategies or indicators.
- **Protocol Changes**: We continue using Binance WebSockets and HMAC-SHA256 signing as currently implemented.
- **Hardware Expansion**: The system remains optimized for a single-device RISC-V architecture.

## Decisions

### 1. Ingestion Actor Split
**Choice**: Replace `MarketGateway` with three specialized actors: `PublicStreamActor`, `UserStreamActor`, and `FuturesStreamActor`.
**Rationale**: Ingestion failures (e.g., an expired `ListenKey` for user data) should not stop public market data ingestion. This split isolates connection state and crash risks.
**Alternatives**: Maintaining a single gateway with internal state machine. Rejected due to high code complexity and shared failure domain.

### 2. Centralized Event-Driven Storage
**Choice**: Implement a `StorageActor` that subscribes to the `MarketEvent` broadcast channel.
**Rationale**: By making storage a consumer of the broadcast bus, other actors (Strategy, Ingestion) can "fire and forget" events. This eliminates DB write latency from the "Hot Path" (Ingestion -> Inference).
**Alternatives**: Keeping direct `DataManager` access in every service. Rejected as it couples logic to persistence and makes unit testing harder.

### 3. Pure Mathematical Indicators
**Choice**: Extract all TA indicators (RSI, BB, etc.) into a stateless `crates/features` or a pure module within `strategy`.
**Rationale**: `StrategyService` should only orchestrate the flow. Moving math to pure functions allows for exhaustive unit testing with mock data without requiring actor setup.
**Alternatives**: Keeping indicators within `SymbolState`. Rejected as it makes the `StrategyService` file too large and hard to test in isolation.

### 4. Zero-Cost Type Safety (NewType)
**Choice**: Wrap `f64` primitives in `Price`, `Quantity`, `Usdt`, and `Bps` structs.
**Rationale**: Prevents accidental mixing of units (e.g., adding a price to a quantity). Rust's zero-cost abstractions ensure this has no runtime performance penalty.
**Alternatives**: Using plain `f64` with strict naming conventions. Rejected as it is prone to human error and doesn't leverage the compiler.

## Architecture Diagram

```ascii
   ┌─────────────────┐       ┌─────────────────┐       ┌─────────────────┐
   │ PublicStreamSvc │       │  UserStreamSvc  │       │ FuturesStreamSvc│
   │ (AggTrade, etc) │       │ (Account Info)  │       │ (MarkPrice, etc)│
   └────────┬────────┘       └────────┬────────┘       └────────┬────────┘
            │                         │                         │
            └───────────────┬─────────┴─────────────────────────┘
                            │
                            ▼ (Arc<MarketEvent> Broadcast)
   ┌──────────────────────────────────────────────────────────────────────┐
   │                         CENTRAL BROADCAST BUS                        │
   └───────┬──────────────────────────┬────────────────────────────┬──────┘
           │                          │                            │
           ▼                          ▼                            ▼
   ┌───────────────┐          ┌───────────────┐            ┌───────────────┐
   │  StorageActor │          │ StrategyActor │            │  AdminActor   │
   │ (Persistence) │          │ (Intelligence)│            │  (Telegram)   │
   └───────┬───────┘          └───────┬───────┘            └───────────────┘
           │                          │
           ▼                          ▼
   ┌───────────────┐          ┌───────────────┐
   │ SQLite (DB)   │          │  Execution    │
   └───────────────┘          └───────────────┘
```

## Risks / Trade-offs

- **[Risk] Channel Congestion** → **Mitigation**: The `MarketEvent` broadcast channel capacity is set to 10,000. We will monitor `Lagged` errors in `StorageActor` and `StrategyActor` to ensure the bus isn't overwhelmed during high volatility.
- **[Risk] RISC-V Threading** → **Mitigation**: Increasing the actor count adds thread overhead. However, the Orange Pi's 8-core CPU is under-utilized in the current monolithic model; the decentralized model will better distribute the load across cores.
- **[Trade-off] Boilerplate** → Using `NewType` wrappers requires manual implementation of `Add`, `Sub`, etc., or using crates like `derive_more`. We will use explicit manual implementations for the core math to maintain maximum control over rounding and precision.
