use teloxide::prelude::*;
use tokio::sync::broadcast;
use std::env;
use tracing::{info, error};

pub struct TelegramService {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramService {
    pub fn new() -> Self {
        // We expect these to be present. If not, the service will panic at startup, which is fine for critical config.
        let token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set in .env");
        let chat_id_str = env::var("TELEGRAM_CHAT_ID").expect("TELEGRAM_CHAT_ID not set in .env");
        let chat_id = chat_id_str.parse::<i64>().expect("TELEGRAM_CHAT_ID must be a number");
        
        let bot = Bot::new(token);
        
        Self {
            bot,
            chat_id: ChatId(chat_id),
        }
    }

    pub async fn start(self, mut rx: broadcast::Receiver<String>) {
        info!("Starting Telegram Notification Service");
        
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    // Send message and log error if it fails, but don't crash
                    if let Err(e) = self.bot.send_message(self.chat_id, msg).await {
                        error!("Failed to send Telegram message: {}", e);
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    error!("Telegram service lagged behind. Missed {} messages.", n);
                }
                Err(_) => {
                    info!("Telegram notification channel closed. Stopping service.");
                    break;
                }
            }
        }
    }
}
