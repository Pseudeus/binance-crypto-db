## ADDED Requirements

### Requirement: Specialized Ingestion Actors
The `MarketGateway` SHALL be decomposed into specialized actors, each responsible for a single Binance stream source to improve isolation and fault tolerance:
- `PublicStreamActor`: Ingests public market data (aggTrade, depth, klines).
- `UserStreamActor`: Ingests account-specific data (AccountUpdate, ExecutionReport).
- `FuturesStreamActor`: Ingests futures-specific market data (markPrice, forceOrder).

#### Scenario: Isolated Ingestion Failure
- **WHEN** the `UserStreamActor` crashes due to an invalid ListenKey
- **THEN** the `PublicStreamActor` SHALL continue ingesting market data without interruption.

### Requirement: Normalized MarketEvents
All ingestion actors SHALL map raw JSON responses from Binance into a common `MarketEvent` enum before broadcasting to the system.

#### Scenario: Broadcasting AggTrade
- **WHEN** a raw `aggTrade` message is received by the `PublicStreamActor`
- **THEN** it SHALL be parsed and broadcast as a `MarketEvent::AggTrade` wrapped in an `Arc<>`.
