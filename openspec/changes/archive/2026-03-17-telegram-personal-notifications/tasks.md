## 1. Telegram Service Refactor

- [x] 1.1 Update `TelegramService::run` to handle incoming commands using `teloxide`'s command system.
- [x] 1.2 Implement `/start` and `/id` command handlers to reply with the sender's `chat_id`.
- [x] 1.3 Ensure the notification listener (broadcast channel) continues to run concurrently with the command handler.
- [x] 1.4 **Validation**: Send a trade notification through the broadcast channel while the command handler is active to verify concurrency.

## 2. Onboarding & App-side Configuration

- [ ] 2.1 **App Side**: Open Telegram and search for `@BotFather`.
- [ ] 2.2 **App Side**: Use `/newbot` to create a new bot and copy the `API TOKEN`.
- [ ] 2.3 **App Side**: Search for your new bot by username and click `START`.
- [ ] 2.4 **App Side**: Send `/id` to the bot (once the code is updated) to get your personal `chat_id`.
- [ ] 2.5 Update the `.env` file with the new `TELEGRAM_BOT_TOKEN` and the discovered `TELEGRAM_CHAT_ID`.
- [ ] 2.6 **Validation**: Send a test notification to ensure it arrives in your personal chat.

## 3. Unit Testing & Robustness

- [x] 3.1 Add a unit test or mock scenario for the command handler to ensure it correctly extracts the `chat_id`.
- [x] 3.2 Implement a check to log a clear error if the bot fails to send a message (e.g., bot blocked by user).
- [x] 3.3 **Validation**: Block the bot temporarily and verify that the system logs an error but does not crash.
