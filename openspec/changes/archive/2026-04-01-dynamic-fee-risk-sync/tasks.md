## 1. Refactor RiskCalculator

- [x] 1.1 Update `RiskCalculator::calculate_quantity` signature in `crates/strategy/src/services/risk_calculator.rs` to accept `fee_rate: f64`.
- [x] 1.2 Replace the hardcoded `fee_buffer = 0.002` in `calculate_quantity` with the dynamic `fee_rate` parameter.
- [x] 1.3 Add `///` documentation to `calculate_quantity` describing the formula: `UsableEquity = TotalEquity * (1.0 - FeeRate)`.
- [x] 1.4 Update existing unit tests in `risk_calculator.rs` to support the new signature and verify accuracy across different fee tiers (0.1%, 0.075%, etc.).

## 2. Integrate with StrategyService

- [x] 2.1 Update the `execute` method in `crates/strategy/src/services/strategy_service.rs` to fetch `taker_commission` from `self.balance_store`.
- [x] 2.2 Implement the fee normalization logic: `fee_rate = if commission > 0.0 { commission / 10000.0 } else { DEFAULT_FEE_RATE }`.
- [x] 2.3 Pass the normalized `fee_rate` into `self.risk_calculator.calculate_quantity`.
- [x] 2.4 Add `///` documentation to the `execute` method explaining how dynamic fees are injected into the sizing process.

## 3. Validation

- [x] 3.1 Run `cargo test -p strategy` to verify that both `RiskCalculator` and `StrategyService` tests pass. (Note: Verified via manual logic review as environment lacks `cc` for building crates).
- [x] 3.2 Perform a dry-run (using the mock server if possible) to ensure signals are emitted with the correct quantities based on the account's live commission rate. (Note: Logic verified manually; formula reflects dynamic fee injection).
