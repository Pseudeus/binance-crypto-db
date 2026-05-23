## ADDED Requirements

### Requirement: Type-Safe Financial Units
The system SHALL use NewType wrappers for all financial units to prevent unit-mismatch errors and ensure compile-time safety. Specifically, it MUST implement:
- `Price`: Wraps `f64` for market prices.
- `Quantity`: Wraps `f64` for trade amounts.
- `Usdt`: Wraps `f64` for quote asset balances.
- `Bps`: Wraps `f64` for basis points (e.g., commissions).

#### Scenario: Enforcing Type Safety
- **WHEN** a function expects a `Quantity` but is passed a `Price`
- **THEN** the Rust compiler SHALL reject the code, preventing a logic error.

### Requirement: Operator Overloading for Financial Units
The `NewType` wrappers SHALL implement standard arithmetic operators (Add, Sub, Mul, Div) where mathematically sound, allowing for intuitive calculations while maintaining type safety.

#### Scenario: Multiplying Price and Quantity
- **WHEN** a `Price` is multiplied by a `Quantity`
- **THEN** the result SHALL be a `Usdt` value representing the total cost.

### Requirement: ExecutionActor Implementation
The system SHALL implement an `ExecutionActor` responsible for executing trade orders via the Binance API when receiving `TradeSignal` broadcasts.

#### Scenario: Order Execution
- **WHEN** a `TradeSignal` is broadcast to the execution channel
- **THEN** the `ExecutionActor` SHALL execute the order via `BinanceClient::post_order` and log the result.

#### Scenario: Execution Failure Handling
- **WHEN** an order placement fails via the Binance API
- **THEN** the `ExecutionActor` SHALL log the error and NOT retry the order to avoid duplicate executions.

#### Scenario: Balance Verification
- **WHEN** an order signal arrives
- **THEN** the `ExecutionActor` SHALL verify account balance via `BinanceClient::get_account` and log available balances for auditing.

### Requirement: RiskCalculator Type Safety
The `RiskCalculator` SHALL use NewType wrappers for all financial calculations:
- **UsableEquity**: Calculated as `TotalEquity * (1.0 - FeeRate)` where `FeeRate` is converted from `Bps`
- **RiskAmount**: Calculated as `UsableEquity * effective_risk_factor`
- **Quantity**: Final result as `RiskAmount / (Volatility * Price)`

#### Scenario: RiskCalculator Type Safety
- **WHEN** `RiskCalculator::calculate_quantity` is called
- **THEN** it SHALL accept `Usdt`, `f64` (volatility fraction), `Price`, and `Bps` (fee) parameters
- **THEN** it SHALL return a `Quantity` result
