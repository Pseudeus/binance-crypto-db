## Context

The system currently manages position sizing via the `RiskCalculator`. However, this calculator uses a hardcoded `fee_buffer = 0.002` (0.2%) to account for Binance fees. Meanwhile, the `BalanceStore` already synchronizes the actual account commission rates (`taker_commission`) from the Binance API. This design aims to connect these components, ensuring that position sizing reflects the user's actual fee tier.

## Goals / Non-Goals

**Goals:**
- **Dynamic Fee Injection**: Pass the real-time fee rate from `BalanceStore` into the `RiskCalculator`.
- **Stateless Calculator**: Maintain `RiskCalculator` as a pure logic component, avoiding direct dependencies on other services.
- **Improved Accuracy**: Ensure "usable equity" calculation in the sizing formula is based on actual costs.

**Non-Goals:**
- **Real-time Fee Deduction**: This change does not implement immediate balance updates based on `ExecutionReport` fees (reserved for a future change).
- **BNB Fee Logic**: We do not yet handle cases where fees are paid in a separate asset (like BNB) for the sizing calculation.

## Decisions

### 1. Functional Parameter Injection
**Choice**: Modify the `RiskCalculator::calculate_quantity` method signature to include `fee_rate: f64`.
**Rationale**: This keeps the `RiskCalculator` decoupled from the `BalanceStore` and makes it trivial to unit test with various fee scenarios.
**Alternatives**: We considered passing the `BalanceStore` into the `RiskCalculator`'s constructor, but that would make the calculator harder to test and more coupled to the system's state.

### 2. Fee Rate Normalization
**Choice**: The `StrategyService` will perform the conversion from basis points (bps) to a decimal rate (e.g., `10.0 bps -> 0.001`).
**Rationale**: `BalanceStore` stores the raw bps values from the API. Centralizing the normalization in the service layer keeps the calculator's math clean (standard multipliers).

### 3. Safety Defaults
**Choice**: If `taker_commission` is 0.0 or uninitialized, the system will use a default value (0.1% or a value from `DEFAULT_FEE_RATE` env var).
**Rationale**: Prevents under-calculating fees if the initial account sync fails or hasn't completed.

## ASCII Diagram

```ascii
   ┌─────────────────┐          ┌─────────────────┐
   │  BalanceStore   │          │ StrategyService │
   │ (taker_comm: 10)├─────────▶│ (Sync & Execute)│
   └─────────────────┘          └────────┬────────┘
                                         │
                                         │ (fee_rate: 0.001)
                                         ▼
                                ┌─────────────────┐
                                │ RiskCalculator  │
                                │ (pure math)     │
                                └─────────────────┘
```

## Risks / Trade-offs

- **[Risk] Sync Latency** → **Mitigation**: On startup, `StrategyService` blocks briefly to fetch account info. If it fails, the safe default (0.1%) is used.
- **[Trade-off] Simplicity over Precision** → We use the "Taker Fee" as a conservative default for all calculations, even if some orders might execute at "Maker" rates. This is a safer default for risk management.
