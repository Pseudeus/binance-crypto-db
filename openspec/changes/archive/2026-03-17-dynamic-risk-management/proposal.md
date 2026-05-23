## Why

Currently, the trading bot uses hardcoded quantities for a limited set of symbols, which fails to account for real-time account balances or market volatility. This creates significant risk of over-leveraging in high-volatility regimes or failing to execute due to insufficient funds, especially as the number of tracked symbols grows.

## What Changes

- **User Data Stream Ingestion**: Implement authenticated WebSocket connection to Binance to receive real-time account updates (balance changes, order fills).
- **Dynamic Position Sizing**: Replace hardcoded symbol quantities with a calculation engine that considers account equity, model confidence, and current market volatility.
- **Stateful Balance Tracking**: Maintain an in-memory view of available assets to ensure trade signals are only generated when capital is available.
- **Execution Layer Refactor**: Update the `ExecutionService` to accept and respect dynamically calculated quantities from the strategy engine.

## Capabilities

### New Capabilities
- `user-data-stream`: Authenticated ingestion of Binance account updates via ListenKey and WebSocket.
- `dynamic-position-sizing`: Mathematical logic for calculating trade quantities based on equity and risk parameters.

### Modified Capabilities
- `market-gateway`: Expanded to support authenticated private streams alongside public market data.

## Impact

- **MarketGateway**: Added complexity for ListenKey management (REST + WebSocket).
- **ExecutionService**: Now reacts to real-time balance state rather than assuming liquidity.
- **StrategyService**: Logic added to calculate `quantity` before emitting a `TradeSignal`.
- **Latency Impact**: Ingestion of balance updates is asynchronous and does not block the "Hot Path" of price ingestion/inference. However, the quantity calculation adds a few microseconds to the strategy loop.

## Success Metrics
- **Zero "Insufficient Balance" failures**: System should know its limits before sending an order.
- **Dynamic Risk Scaling**: Trade sizes should automatically decrease during high volatility regimes.
- **Balance Sync Latency**: Account updates reflected in the internal state within 500ms of the Binance event.
