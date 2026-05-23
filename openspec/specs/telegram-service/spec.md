# telegram-service Specification

## Purpose
TBD - created by archiving change telegram-personal-notifications. Update Purpose after archive.
## Requirements
### Requirement: Telegram Notification Service
The system SHALL maintain a background actor that listens for trade notifications and sends them to a configured Telegram chat.

#### Scenario: Successful notification
- **WHEN** a trade message is received from the strategy engine
- **THEN** the system SHALL attempt to send the message to the configured `TELEGRAM_CHAT_ID`.

### Requirement: Interactive Event Loop
The `TelegramService` SHALL implement an interactive event loop using `teloxide` to handle incoming commands alongside outgoing notifications.

#### Scenario: Handling multiple event types
- **WHEN** the service is running
- **THEN** it SHALL concurrently process incoming Telegram updates and internal broadcast messages.

### Requirement: IP Discovery Command
The `TelegramService` SHALL support the `/ip` command to trigger local IP retrieval.

#### Scenario: User triggers IP discovery
- **WHEN** user sends `/ip` to the bot
- **THEN** the bot SHALL execute the IP retrieval logic and reply with the result.

