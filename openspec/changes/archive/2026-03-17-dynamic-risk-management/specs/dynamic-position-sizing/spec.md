## ADDED Requirements

### Requirement: Volatility-Adjusted Quantity Calculation
The system SHALL calculate the trade quantity for a `TradeSignal` dynamically using the formula: `Quantity = (TotalEquity * RiskPerTrade) / (SymbolVolatility * CurrentPrice)`.

#### Scenario: Trade Quantity Calculation
- **WHEN** a signal is triggered with Equity=1000, RiskPerTrade=0.01, Volatility=0.02 (2%), and Price=100
- **THEN** the system SHALL calculate `Quantity = (1000 * 0.01) / (0.02 * 100) = 5.0`.

### Requirement: Available Balance Validation
The system SHALL verify that the available `free` balance in the `BalanceStore` is sufficient to cover the calculated quantity for the quote asset (e.g., USDT) before emitting a `TradeSignal`.

#### Scenario: Insufficient Balance Check
- **WHEN** a calculated quantity for SOLUSDT is 5.0 (worth 500 USDT) but the `BalanceStore` shows only 100 USDT available
- **THEN** the system SHALL log a warning and skip the trade signal to prevent order failure.

### Requirement: Risk per Trade Ceiling
The system SHALL enforce a maximum risk per trade ceiling (e.g., 2% of equity) to prevent any single calculation error or volatility spike from over-leveraging the account.

#### Scenario: Risk Ceiling Enforcement
- **WHEN** a calculation results in a risk per trade of 5% due to an error in volatility input
- **THEN** the system SHALL clamp the trade quantity to the predefined 2% risk limit.
