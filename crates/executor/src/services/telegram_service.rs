use common::actors::{Actor, ActorType, ControlMessage};
use futures_util::future::BoxFuture;
use std::env;
use teloxide::prelude::*;
use teloxide::utils::command::BotCommands;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info};
use uuid::Uuid;

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "These commands are supported:"
)]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "get your chat id.")]
    Id,
    #[command(description = "get the device local ip.")]
    Ip,
    #[command(description = "start the bot.")]
    Start,
}

pub struct TelegramService {
    id: Uuid,
    bot: Bot,
    chat_id: ChatId,
    rx: broadcast::Receiver<String>,
}

impl TelegramService {
    pub fn new(rx: broadcast::Receiver<String>) -> Self {
        // We expect these to be present. If not, the service will panic at startup, which is fine for critical config.
        let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set in .env");
        let chat_id_str = env::var("TELEGRAM_CHAT_ID").expect("TELEGRAM_CHAT_ID not set in .env");
        let chat_id = chat_id_str
            .parse::<i64>()
            .expect("TELEGRAM_CHAT_ID must be a number");

        let bot = Bot::new(token);

        Self {
            id: Uuid::new_v4(),
            bot,
            chat_id: ChatId(chat_id),
            rx,
        }
    }
}

impl Actor for TelegramService {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> ActorType {
        ActorType::Dynamic
    }

    fn run(
        &mut self,
        supervisor_tx: mpsc::Sender<ControlMessage>,
    ) -> BoxFuture<'_, anyhow::Result<()>> {
        let _heartbeat_handle = self.spawn_heartbeat(supervisor_tx.clone());
        info!("Starting Telegram Notification Service");

        let bot_clone = self.bot.clone();
        let target_chat_id = self.chat_id;
        let mut rx = self.rx.resubscribe();

        // Notification Task
        let notification_handle = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(msg) => {
                        if let Err(e) = bot_clone.send_message(target_chat_id, msg).await {
                            match e {
                                teloxide::RequestError::Api(api_err) => {
                                    error!(
                                        "Telegram API Error: {} (Are you sure the bot is started and not blocked?)",
                                        api_err
                                    );
                                }
                                _ => error!("Failed to send Telegram message: {}", e),
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        error!("Telegram service lagged behind. Missed {} messages.", n);
                    }
                    Err(_) => {
                        info!("Telegram notification channel closed. Stopping notification task.");
                        break;
                    }
                }
            }
        });

        // Command Task
        let bot = self.bot.clone();
        Box::pin(async move {
            let command_task = Command::repl(
                bot,
                |bot: Bot, msg: Message, cmd: Command| async move {
                    match cmd {
                        Command::Help => {
                            bot.send_message(msg.chat.id, Command::descriptions().to_string())
                                .await?;
                        }
                        Command::Id => {
                            bot.send_message(
                                msg.chat.id,
                                format!("Your Chat ID is: {}", msg.chat.id),
                            )
                            .await?;
                        }
                        Command::Ip => {
                            let response = match get_local_ip().await {
                                Ok(ip) => format!("Device Local IP: {}", ip),
                                Err(e) => {
                                    format!("Error: Could not retrieve local IP address. ({})", e)
                                }
                            };
                            bot.send_message(msg.chat.id, response).await?;
                        }
                        Command::Start => {
                            bot.send_message(msg.chat.id, "Welcome to the Binance Bot! Use /id to get your chat ID for configuration.").await?;
                        }
                    };
                    Ok(())
                },
            );

            tokio::select! {
                _ = notification_handle => {
                    info!("Notification task finished.");
                }
                _ = command_task => {
                    info!("Command task finished.");
                }
            }

            Ok(())
        })
    }
}

/// Retrieves the primary local IP address of the host machine.
///
/// This implementation uses the UDP "connect" trick: it binds to a local port
/// and attempts to "connect" to a well-known public IP (8.8.8.8).
/// This doesn't actually send any network packets or establish a handshake,
/// but allows the OS to determine which local interface would be used
/// to route traffic to that destination.
///
/// # Complexity
/// - **Time Complexity**: O(1). The operation involves a system call to bind and connect
///   a socket, which are constant time operations regardless of system state.
/// - **Memory Complexity**: O(1). Only a single socket and a few address structures
///   are allocated on the stack.
///
/// # Justification
/// This method is preferred over parsing `ifconfig` or `ip addr` because it is
/// cross-platform and doesn't depend on external command-line tools or specific
/// OS output formats. It correctly identifies the interface with an active
/// route to the internet.
async fn get_local_ip() -> anyhow::Result<String> {
    use tokio::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.connect("8.8.8.8:80").await?;
    let local_addr = socket.local_addr()?;
    Ok(local_addr.ip().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_parsing() {
        let help = Command::parse("/help", "").unwrap();
        if let Command::Help = help {
        } else {
            panic!("Expected Help");
        }

        let id = Command::parse("/id", "").unwrap();
        if let Command::Id = id {
        } else {
            panic!("Expected Id");
        }

        let ip = Command::parse("/ip", "").unwrap();
        if let Command::Ip = ip {
        } else {
            panic!("Expected Ip");
        }

        let start = Command::parse("/start", "").unwrap();
        if let Command::Start = start {
        } else {
            panic!("Expected Start");
        }
    }

    #[tokio::test]
    async fn test_get_local_ip() {
        let ip = get_local_ip().await.unwrap();
        println!("Test Local IP: {}", ip);
        assert!(!ip.is_empty());
        // Basic check for IP format (at least one dot)
        assert!(ip.contains('.'));
    }
}
