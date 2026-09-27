# sqlite-storage Specification

## Purpose

High-throughput, append-only SQLite persistence layer providing asynchronous batching, dedicated single-threaded writer execution, and weekly database rotation for market time-series data.

## Requirements

### Requirement: Asynchronous Dual-Trigger Batch Persistence
The persistence layer SHALL buffer incoming market events in memory and commit them to SQLite in bulk transactions. A flush MUST occur whenever a buffer reaches its configured capacity threshold OR when a 1.0-second interval has elapsed since the earliest buffered record, whichever occurs first.

#### Scenario: Capacity threshold reached
- **WHEN** the number of buffered events of a specific event type reaches the configured capacity limit (e.g. 5,000 records)
- **THEN** the persistence layer flushes the entire batch to SQLite within a single transaction without blocking subsequent incoming events

#### Scenario: Interval timeout flush
- **WHEN** the buffer has uncommitted records and the 1.0-second timer elapses before the capacity limit is reached
- **THEN** the persistence layer flushes all currently buffered records to SQLite within a single transaction

#### Scenario: Clean shutdown flush
- **WHEN** the storage service receives a termination signal or broadcast channel closure
- **THEN** all remaining buffered events across all market data types are committed to SQLite before the service exits

### Requirement: Dedicated Single-Thread Writer Isolation
All SQLite write transactions SHALL execute exclusively on a dedicated OS background worker thread owning the SQLite connection. Communication between asynchronous Tokio actors and the writer thread MUST use bounded message channels transferring vector ownership with zero cross-thread database locking or Tokio worker thread starvation.

#### Scenario: Tokio non-blocking dispatch
- **WHEN** market event streams experience high-frequency bursts exceeding 50,000 records per second
- **THEN** the Tokio async tasks append events to local buffers and dispatch batches over channel without spawning tasks per event or stalling the async scheduler

#### Scenario: Transaction serialization
- **WHEN** multiple event types (e.g. trade and order book batches) are ready for persistence simultaneously
- **THEN** the dedicated writer processes each batch sequentially in its own transaction without encountering `SQLITE_BUSY` errors

### Requirement: Bare-Metal Performance PRAGMA Initialization
Every SQLite database connection opened by the persistence layer SHALL be initialized with performance-optimized PRAGMAs: `journal_mode = WAL`, `synchronous = OFF`, `cache_size = -64000` (64MB RAM), `mmap_size = 1073741824` (1GB memory mapping), `temp_store = MEMORY`, and `locking_mode = EXCLUSIVE`.

#### Scenario: Connection startup pragma application
- **WHEN** a new weekly SQLite database file is created or connected
- **THEN** all specified PRAGMAs are executed and verified before accepting write transactions

### Requirement: ISO Weekly Database File Rotation
The persistence layer SHALL partition time-series data into weekly SQLite files named `crypto_<YEAR>_<WEEK>.db` based on the ISO year and week of current UTC time. When the ISO week changes, the writer MUST close the current database file, create the new week's database file with the required schema, and signal the supervisor to trigger archival of the previous week's file.

#### Scenario: ISO week boundary transition
- **WHEN** UTC time crosses into a new ISO week during continuous operation
- **THEN** pending writes for the outgoing week are committed, the database connection is transitioned to `crypto_<YEAR>_<NEW_WEEK>.db`, and a backup actor spawn message is dispatched to the supervisor
