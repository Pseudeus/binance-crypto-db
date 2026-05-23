## Context

The bot currently uses `teloxide` to send notifications to a configured `TELEGRAM_CHAT_ID`. This ID is often a group ID (negative integer). The user wants to switch to a personal chat ID (positive integer) and needs a way to discover this ID easily from the Telegram app.

## Goals / Non-Goals

**Goals:**
- **Easy ID Discovery**: Enable the user to find their personal chat ID by sending a message to the bot.
- **Direct Messaging**: Ensure the bot can send notifications directly to the discovered personal ID.
- **Clear Onboarding**: Document the process of creating a bot and getting the ID.

**Non-Goals:**
- **Multi-user Support**: The bot is designed for a single owner.
- **Complex Command System**: We are only adding discovery commands, not a full TUI via Telegram.

## Decisions

### 1. Unified Event Loop in TelegramService
**Choice**: Use `teloxide`'s `repl` or a custom loop that handles both incoming commands and outgoing notifications (via the existing `broadcast::Receiver`).
**Rationale**: Keeps the service simple while enabling interactivity.
**Alternatives**: Running two separate tasks (one for REPL, one for Notifications), but sharing the `Bot` instance is cleaner in a single unified loop.

### 2. Discovery Command
**Choice**: Implement `/id` and `/start`.
**Rationale**: Standard practice for Telegram bots. `/start` is the first thing a user clicks.

### 3. Data Flow

```ascii
   ┌────────────────┐       ┌──────────────────┐
   │ Telegram App   │──────▶│ TelegramService  │ (Command: /id)
   └────────────────┘       └─────────┬────────┘
                                      │
                                      ▼
   ┌────────────────┐       ┌──────────────────┐
   │ Telegram App   │◀──────┤ TelegramService  │ (Reply: Your ID is X)
   └────────────────┘       └──────────────────┘
                                      ▲
                                      │ (broadcast: Notification)
                            ┌─────────┴────────┐
                            │  Strategy Engine │
                            └──────────────────┘
```

## Risks / Trade-offs

- **[Risk] Multiple Users** → **[Mitigation]** The bot will reply to *anyone* who sends `/id` with their ID, but it will *only* send trade notifications to the ID configured in `.env`.
- **[Trade-off] Poll vs Webhook** → We use long-polling (default `teloxide` REPL) for simplicity on the Orange Pi, as it doesn't require a public IP/domain.

## Data Structures (Memory Footprint)

No significant new data structures are introduced. We reuse the existing `ChatId` and `Bot` types from `teloxide`.
