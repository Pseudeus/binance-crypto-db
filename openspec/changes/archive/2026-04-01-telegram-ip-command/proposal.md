## Why

As the bot runs on an Orange Pi RV2 (RISC-V) in a potentially changing network environment, it is useful for the owner to quickly verify the device's local network identity without needing SSH access. This command provides a quick health check for connectivity and accessibility within the local network.

## What Changes

- **Telegram IP Discovery**: Add a new `/ip` command to the `TelegramService` that retrieves the device's local IP address and returns it to the user.
- **Dependency Addition (Optional)**: If a native Rust method isn't used, a small crate like `local_ipaddress` or a shell command call may be introduced.

## Capabilities

### New Capabilities
- `system-info-discovery`: Ability for the bot to report system-level metadata (like IP address) via Telegram.

### Modified Capabilities
- `telegram-service`: Update the service to handle the `/ip` command.

## Impact

- **TelegramService**: Expansion of the `Command` enum and handler.
- **Latency Impact**: Zero. This is an administrative command handled outside the hot path.
- **Success Metrics**:
  - User receives the correct local IP (e.g., `192.168.x.x`) upon sending `/ip`.
  - No impact on trade notification latency.
