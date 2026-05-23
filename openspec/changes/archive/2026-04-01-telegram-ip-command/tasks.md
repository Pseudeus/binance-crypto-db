## 1. Core Logic & Commands

- [x] 1.1 Add `Ip` variant to the `Command` enum in `crates/executor/src/services/telegram_service.rs`.
- [x] 1.2 Implement a private utility function `get_local_ip` using the UDP socket trick (connect to 8.8.8.8).
- [x] 1.3 Add documentation for the new command and utility function using `///` (cargo doc standards), including time/memory complexity justifications.
- [x] 1.4 **Validation**: Write a unit test for `get_local_ip` to ensure it correctly returns an IP address format.

## 2. Command Handler Integration

- [x] 2.1 Update the `Command::repl` logic in `TelegramService::run` to handle the `Command::Ip` variant.
- [x] 2.2 Implement the reply logic to send the retrieved IP back to the user or an error message if retrieval fails.
- [x] 2.3 **Validation**: Manually verify the command by sending `/ip` to the bot in Telegram.

## 3. Finalization & Documentation

- [x] 3.1 Run `cargo doc --open` to verify that the new documentation is correctly rendered and includes the required complexity analysis.
- [x] 3.2 **Validation**: Run `cargo check` and `cargo test` to ensure no regressions were introduced.
