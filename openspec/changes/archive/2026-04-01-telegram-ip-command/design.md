## Context

The bot owner needs to know the device's local IP address for networking purposes. Since the device is headless, a Telegram command is an efficient way to retrieve this information.

## Goals / Non-Goals

**Goals:**
- **Retrieve Local IP**: Implementation of a method to find the primary local IP address of the Orange Pi.
- **Telegram Integration**: Expose this via a `/ip` command.

**Non-Goals:**
- **Public IP Retrieval**: This change focuses on the local network address.
- **Network Configuration**: The command is read-only.

## Decisions

### 1. IP Retrieval Method
**Choice**: Use the UDP Socket "connect" trick.
```rust
let socket = UdpSocket::bind("0.0.0.0:0").await?;
socket.connect("8.8.8.8:80").await?;
let local_addr = socket.local_addr()?;
```
**Rationale**: This is a standard, cross-platform way to determine the primary local IP without external dependencies or parsing shell output (like `ifconfig`). It doesn't actually send any packets.

### 2. Command Integration
**Choice**: Extend the existing `Command` enum in `TelegramService`.
**Rationale**: Keeps all interactive logic centralized.

## ASCII Diagram

```ascii
   ┌────────────────┐       ┌──────────────────┐
   │ Telegram App   │──────▶│ TelegramService  │ (Command: /ip)
   └────────────────┘       └─────────┬────────┘
                                      │
                                      ▼
   ┌────────────────┐       ┌──────────────────┐
   │ Orange Pi OS   │◀──────┤   IP Discovery   │ (UDP Socket Trick)
   └────────────────┘       └─────────┬────────┘
                                      │
                                      ▼
   ┌────────────────┐       ┌──────────────────┐
   │ Telegram App   │◀──────┤ TelegramService  │ (Reply: IP is 192.168.x.x)
   └────────────────┘       └──────────────────┘
```

## Risks / Trade-offs

- **[Risk] Connectivity** → If the device has no internet access at all (can't resolve/route to 8.8.8.8), the UDP trick might fail. **Mitigation**: Add a fallback to "127.0.0.1" or a clear error message.
- **[Trade-off] Dependency** → We avoid adding a new crate by using `std::net` / `tokio::net`.
