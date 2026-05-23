# system-info-discovery Specification

## ADDED Requirements

### Requirement: Local IP Retrieval
The system SHALL retrieve the primary local IP address of the host machine by attempting a non-blocking UDP connection to a well-known public IP (e.g., 8.8.8.8).

#### Scenario: Successful IP retrieval
- **WHEN** the user sends `/ip` to the bot
- **THEN** the system SHALL return the local IP address (e.g., "192.168.1.50")

### Requirement: IP Retrieval Error Handling
The system SHALL handle cases where the local IP cannot be determined (e.g., no network interface) by returning a clear error message.

#### Scenario: Network interface unavailable
- **WHEN** the IP retrieval fails due to a network error
- **THEN** the bot SHALL reply with "Error: Could not retrieve local IP address."
