## Why

The `RiskCalculator` currently uses a hardcoded 0.2% fee buffer (`fee_buffer = 0.002`), which is inaccurate for many users (e.g., those with VIP tiers or BNB fee discounts). This inaccuracy leads to suboptimal position sizing and potential "Insufficient Funds" errors if the actual fee is higher than expected.

## What Changes

- **Dynamic Fee Input**: Refactor `RiskCalculator` to accept a dynamic `fee_rate` instead of using a hardcoded constant.
- **Strategy Engine Integration**: Update `StrategyService` to fetch the live `taker_commission` from `BalanceStore` and pass it to the `RiskCalculator`.
- **Improved Sizing Logic**: The `RiskCalculator` will use the actual exchange fee to calculate the "usable equity" more precisely.

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `dynamic-position-sizing`: Update the quantity calculation requirement to use dynamic fee rates instead of static buffers.

## Impact

- **RiskCalculator**: Change to `calculate_quantity` signature and implementation.
- **StrategyService**: Update the `execute` flow to provide the fee rate.
- **Latency Impact**: Zero. Passing an additional `f64` in the strategy loop has no measurable impact on execution latency.
- **Hot Path**: Yes. This affects the trade execution logic, but only in terms of calculation accuracy, not performance.
- **Success Metrics**:
  - `RiskCalculator` unit tests verify correct sizing across multiple fee tiers (e.g., 0.1%, 0.075%, 0.02%).
  - Complete removal of the hardcoded `0.002` fee buffer from the codebase.
