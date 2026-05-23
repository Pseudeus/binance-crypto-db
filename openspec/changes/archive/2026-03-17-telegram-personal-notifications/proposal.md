## Why

The current Telegram notification implementation is primarily designed for group chats, which requires managing group settings and potentially exposing trade signals to multiple users. A personal chat ID implementation provides a more private, direct, and secure way for the bot owner to receive real-time alerts.

## What Changes

- **Personal Chat Discovery**: Add a simple command handler (e.g., `/start` or `/id`) to the `TelegramService` that replies with the user's personal `chat_id`.
- **Validation Enhancement**: Add validation to ensure the configured `TELEGRAM_CHAT_ID` is a valid personal ID (positive integer) if strict mode is enabled.
- **Onboarding Instructions**: Provide clear, step-by-step instructions in the implementation tasks for the user to set up the bot on the Telegram app side and retrieve their personal chat ID.

## Capabilities

### New Capabilities
- `telegram-discovery`: Ability for the bot to report the `chat_id` of an incoming private message to facilitate configuration.

### Modified Capabilities
- `telegram-service`: Update the service to optionally handle incoming messages for ID discovery while maintaining its primary notification role.

## Impact

- **TelegramService**: Small addition to the event loop to listen for specific commands.
- **Latency Impact**: Zero impact on the "Hot Path" (Data Ingestion/Inference). The Telegram service runs in its own actor and handles discovery messages asynchronously.
- **Success Metrics**: 
  - User can successfully retrieve their personal `chat_id` by messaging the bot.
  - Bot successfully sends notifications to the personal chat ID.
  - No disruption to existing notification logic.
