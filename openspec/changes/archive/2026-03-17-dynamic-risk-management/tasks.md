## 1. Data Models & Common Structures

- [x] 1.1 Define `AccountUpdate` and `ExecutionReport` structs in `common/src/models/mod.rs`
- [x] 1.2 Implement `RemoteResponse` for the new Binance User Stream payloads
- [x] 1.3 **Validation**: Run `cargo check` on `common` crate to ensure compilation

## 2. MarketGateway & ListenKey Management

- [x] 2.1 Add `get_listen_key` and `refresh_listen_key` methods to `BinanceClient`
- [x] 2.2 Implement `user_data_stream_loop` in `MarketGateway` to handle reconnection and ListenKey refreshes
- [x] 2.3 Integrate User Data Stream WebSocket connection into the main Gateway actor
- [x] 2.4 **Validation**: Log incoming `outboundAccountPosition` events from a real or mock Binance connection

## 3. Dynamic Position Sizing Logic

- [x] 3.1 Create a `BalanceStore` component to maintain a thread-safe map of available assets
- [x] 3.2 Implement the `RiskCalculator` function using the volatility-adjusted formula from the design
- [x] 3.3 Add `RiskFactor` and `RiskCeiling` configuration parameters (via `.env`)
- [x] 3.4 **Validation**: Add unit tests for `RiskCalculator` covering edge cases like zero volatility and negative balance

## 4. Strategy & Execution Integration

- [x] 4.1 Refactor `StrategyService` to update `BalanceStore` from incoming `AccountUpdate` events
- [x] 4.2 Modify `process_tick` in `StrategyService` to calculate `quantity` and verify balance before emitting a signal
- [x] 4.3 Update `ExecutionService` to respect the `quantity` field in the `TradeSignal`
- [x] 4.4 **Validation**: Run a simulation where the bot skips a signal due to simulated "Insufficient Funds" in the `BalanceStore`

## 5. Final System Integration

- [x] 5.1 Update the main entry point to initialize the new User Data actors and channels
- [x] 5.2 **Validation**: Final integration test using `test/mock_server.py` to confirm end-to-end signal-to-execution flow with dynamic sizing
