## ADDED Requirements

### Requirement: Public Stream Consolidation
The system SHALL consolidate AggTrade, Depth20 (100ms), and Klines (1s, 1m, 1h) into a single supervised `PublicStreamActor`.

#### Scenario: Multi-Stream Connection
- **WHEN** the `PublicStreamActor` initializes
- **THEN** it SHALL establish a single WebSocket connection to the Binance Combined Stream URL containing all requested symbol streams.

### Requirement: Order Book Snapshots
The system SHALL ingest depth20@100ms snapshots for all tracked symbols.

#### Scenario: Order Book Update
- **WHEN** a Depth event is received for "BTCUSDT"
- **THEN** it SHALL be broadcast as an `OrderBook` event to internal consumers (DB, Strategy).

### Requirement: Kline Stream Ingestion
The system SHALL ingest 1s, 1m, and 1h Klines to support multi-horizon strategy inference.

#### Scenario: Kline Data Arrival
- **WHEN** a Kline event is received for "ETHUSDT" at the 1m interval
- **THEN** it SHALL be broadcast as a `Kline` event.

### Requirement: Public Stream Supervision
The `PublicStreamActor` SHALL implement the `Actor` trait and send heartbeats every 500ms.

#### Scenario: WebSocket Connection Loss
- **WHEN** the WebSocket connection for public data drops
- **THEN** the `PublicStreamActor` SHALL report an error and rely on the `Supervisor` for a restart if heartbeats stop.
