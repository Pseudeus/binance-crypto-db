## MODIFIED Requirements

### Requirement: Interactive Event Loop
The `TelegramService` SHALL implement an interactive event loop using `teloxide` to handle incoming commands alongside outgoing notifications.

#### Scenario: Handling multiple event types
- **WHEN** the service is running
- **THEN** it SHALL concurrently process incoming Telegram updates and internal broadcast messages.

### ADDED Requirements

### Requirement: IP Discovery Command
The `TelegramService` SHALL support the `/ip` command to trigger local IP retrieval.

#### Scenario: User triggers IP discovery
- **WHEN** user sends `/ip` to the bot
- **THEN** the bot SHALL execute the IP retrieval logic and reply with the result.
