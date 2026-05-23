## Context

The system currently executes trades based on hardcoded quantities, which is a major risk for a high-performance bot. To trade professionally, we must know exactly how much capital we have and how much we are willing to lose on each trade. This requires a real-time bridge between our account state on Binance and our strategy engine.

## Goals / Non-Goals

**Goals:**
- **Real-time Account State**: Ingest account updates (balances, order fills) with sub-second latency.
- **Volatility-Adjusted Sizing**: Automatically scale position sizes based on current market volatility and available equity.
- **ListenKey Management**: Automate the lifecycle of the Binance User Data Stream (Keep-Alive logic).

**Non-Goals:**
- **Portfolio Rebalancing**: This is out of scope for the execution-focused bot.
- **Tax/Accounting Reporting**: Focus is strictly on execution-time sizing.

## Decisions

### 1. Actor-Based Balance Tracking
**Choice**: Use a new broadcast channel `broadcast::Sender<Arc<AccountUpdate>>`.
**Rationale**: Aligns with the existing zero-copy broadcast architecture used for market data. The `StrategyService` will subscribe to these updates to maintain a local `BalanceStore`.
**Alternatives**: Shared `Arc<RwLock>` was considered, but it introduces locking contention in the hot path.

### 2. Position Sizing Algorithm
**Choice**: Volatility-Adjusted Risk Units.
`Quantity = (Equity * RiskFactor) / (CurrentVolatility * Price)`
**Rationale**: This formula ensures that the bot's risk per trade remains constant regardless of whether the market is calm or chaotic.

### 3. UserDataStream Lifecycle
**Choice**: The `MarketGateway` will manage the `ListenKey`.
**Rationale**: `MarketGateway` already handles WebSocket connections. It will now gain a `listen_key_loop` that pings Binance every 30 minutes to prevent session expiration.

## ASCII Diagram

```ascii
   ┌─────────────────┐       ┌──────────────────┐
   │ Binance REST API│◄─────▶│  MarketGateway   │ (ListenKey Refresh)
   └─────────────────┘       └─────────┬────────┘
                                       │
                                       ▼ (Arc<AccountUpdate>)
   ┌─────────────────┐       ┌──────────────────┐
   │ StrategyService │◄──────┤   BalanceStore   │ (Local State)
   └────────┬────────┘       └──────────────────┘
            │
            ▼ (TradeSignal { quantity: calculated_val })
   ┌─────────────────┐
   │ ExecutionService│
   └─────────────────┘
```

## Risks / Trade-offs

- **[Risk] WebSocket Disconnect** → **[Mitigation]** The `BalanceStore` should default to 0.0 or a "safe" mode if updates haven't been received in N minutes.
- **[Risk] Partial Fills** → **[Mitigation]** The system must listen for `executionReport` events to update local balances immediately rather than waiting for the next account update.
- **[Trade-off] REST Latency** → We use REST for the initial balance sync on startup, then switch entirely to WebSockets to minimize latency.

## Data Model (Memory Footprint)

```rust
pub struct AccountUpdate {
    pub asset: String,
    pub free: f64,
    pub locked: f64,
}
// Compact representation to minimize serialization overhead.
```
