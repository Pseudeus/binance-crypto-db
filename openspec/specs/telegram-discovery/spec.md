# telegram-discovery Specification

## Purpose
TBD - created by archiving change telegram-personal-notifications. Update Purpose after archive.
## Requirements
### Requirement: Chat ID Discovery Command
The system SHALL provide a `/id` command that responds with the sender's numeric `chat_id`.

#### Scenario: User requests their ID
- **WHEN** user sends `/id` to the bot in a private chat
- **THEN** the bot SHALL reply with a message containing "Your Chat ID is: <id>"

### Requirement: Startup Welcome Message
The system SHALL provide a `/start` command that welcomes the user and explains how to get their ID.

#### Scenario: User starts the bot
- **WHEN** user sends `/start` to the bot
- **THEN** the bot SHALL reply with a welcome message and instructions to use `/id` for configuration.

### Requirement: Private Notification Delivery
The system SHALL send all trade notifications to the `TELEGRAM_CHAT_ID` configured in the environment, supporting both positive (personal) and negative (group) IDs.

#### Scenario: Trade notification delivery
- **WHEN** a new trade signal is received via the broadcast channel
- **THEN** the bot SHALL send a message to the configured `TELEGRAM_CHAT_ID`.

