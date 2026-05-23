## ADDED Requirements

### Requirement: Centralized Storage Actor
The system SHALL implement a dedicated `StorageActor` responsible for all SQLite persistence operations, subscribing to the `MarketEvent` broadcast channel.

#### Scenario: Event Persistence
- **WHEN** a `MarketEvent::AggTrade` is broadcast on the central bus
- **THEN** the `StorageActor` SHALL receive the event and write it to the SQLite database asynchronously.

### Requirement: Decoupled Logic and Persistence
Application logic actors (e.g., `StrategyActor`) SHALL NOT perform direct database writes. They MUST emit events that the `StorageActor` captures for persistence.

#### Scenario: Non-Blocking Strategy Execution
- **WHEN** the `StrategyActor` generates a `TradeSignal`
- **THEN** it SHALL broadcast the signal immediately without waiting for any database confirmation, ensuring minimal execution latency.
