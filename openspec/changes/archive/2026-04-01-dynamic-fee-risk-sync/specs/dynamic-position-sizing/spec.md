## MODIFIED Requirements

### Requirement: Volatility-Adjusted Quantity Calculation
The system SHALL calculate the trade quantity for a `TradeSignal` dynamically using the formula: `Quantity = (UsableEquity * RiskPerTrade) / (SymbolVolatility * CurrentPrice)`, where `UsableEquity = TotalEquity * (1.0 - FeeRate)`. The `FeeRate` MUST be provided dynamically from the `BalanceStore` commission settings.

#### Scenario: Trade Quantity Calculation with Dynamic Fees
- **WHEN** a signal is triggered with Equity=1000, RiskPerTrade=0.01, Volatility=0.02, Price=100, and FeeRate=0.001 (0.1%)
- **THEN** the system SHALL calculate `UsableEquity = 999.0` and `Quantity = (999.0 * 0.01) / (0.02 * 100) = 9.99 / 2 = 4.995`.
