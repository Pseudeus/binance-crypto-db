## MODIFIED Requirements

### Requirement: Volatility-Adjusted Quantity Calculation
The system SHALL calculate the trade quantity for a `TradeSignal` dynamically using the formula: `Quantity = (UsableEquity * RiskPerTrade) / (SymbolVolatility * CurrentPrice)`, where `UsableEquity = TotalEquity * (1.0 - FeeRate)`. The calculation MUST utilize the `Price`, `Quantity`, and `Usdt` NewType wrappers to ensure arithmetic correctness.

#### Scenario: Trade Quantity Calculation
- **WHEN** a signal is triggered with Equity=1000 Usdt, RiskPerTrade=0.01, Volatility=0.02 (2%), and Price=100 Price
- **THEN** the system SHALL calculate `Quantity = (1000 * 0.01) / (0.02 * 100) = 5.0 Quantity`.

#### Scenario: Trade Quantity Calculation with Dynamic Fees
- **WHEN** a signal is triggered with Equity=1000 Usdt, RiskPerTrade=0.01, Volatility=0.02, Price=100 Price, and FeeRate=0.001 (0.1%)
- **THEN** the system SHALL calculate `UsableEquity = 999.0 Usdt` and `Quantity = (999.0 * 0.01) / (0.02 * 100) = 4.995 Quantity`.
