## ADDED Requirements

### Requirement: Futures Stream Consolidation
The system SHALL consolidate MarkPrice (1s) and ForceOrder events into a single supervised `FuturesStreamActor`.

#### Scenario: Futures Stream Connection
- **WHEN** the `FuturesStreamActor` initializes
- **THEN** it SHALL establish a single WebSocket connection to the Binance Futures Combined Stream URL.

### Requirement: MarkPrice Ingestion
The system SHALL ingest 1s MarkPrice events for all tracked symbols.

#### Scenario: MarkPrice Update
- **WHEN** a MarkPrice update arrives for "SOLUSDT"
- **THEN** it SHALL be broadcast as a `MarkPrice` event for mark-to-market calculations and inference.

### Requirement: Liquidation Ingestion
The system SHALL ingest `forceOrder` events to track liquidations.

#### Scenario: Liquidation Event Processing
- **WHEN** a Liquidation event (forceOrder) occurs for any tracked symbol
- **THEN** the system SHALL broadcast a `ForceOrder` event to the DB and Strategy engine.

### Requirement: Futures Stream Supervision
The `FuturesStreamActor` SHALL implement the `Actor` trait and send heartbeats every 500ms.

#### Scenario: Futures Stream Failure
- **WHEN** the `FuturesStreamActor` encounters a fatal network error
- **THEN** the `Supervisor` SHALL detect the missing heartbeat and restart the actor.
