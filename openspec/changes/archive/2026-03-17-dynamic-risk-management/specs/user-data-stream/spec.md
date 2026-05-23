## ADDED Requirements

### Requirement: ListenKey Management
The system SHALL obtain a ListenKey from the Binance REST API and implement a recurring refresh task (every 30 minutes) to prevent the User Data Stream from expiring.

#### Scenario: ListenKey Refresh Cycle
- **WHEN** the 30-minute timer for a ListenKey expires
- **THEN** the system SHALL send a PUT request to the Binance ListenKey endpoint to extend the session.

### Requirement: Real-time Balance Ingestion
The system SHALL subscribe to the Binance User Data Stream WebSocket and parse `outboundAccountPosition` (payload type `outboundAccountPosition`) events to track changes in asset balances.

#### Scenario: Balance Update Processing
- **WHEN** an `outboundAccountPosition` event is received for asset "USDT" with free="1500.50"
- **THEN** the system SHALL broadcast an internal `AccountUpdate` message and update the local `BalanceStore`.

### Requirement: Order Fill Synchronization
The system SHALL parse `executionReport` events from the User Data Stream to immediately update local balances upon order execution, ensuring the `BalanceStore` remains accurate between full account snapshots.

#### Scenario: Immediate Balance Update on Fill
- **WHEN** an `executionReport` indicates a "FILLED" BUY order for "SOLUSDT" at price 100 and quantity 10
- **THEN** the system SHALL immediately deduct 1000 USDT from the local `BalanceStore` and add 10 SOL.
