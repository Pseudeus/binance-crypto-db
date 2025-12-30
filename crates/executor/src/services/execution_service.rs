use common::models::TradeSignal;
use market_data::remote::BinanceClient;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

pub struct ExecutionService {
    client: BinanceClient,
}

impl ExecutionService {
    pub fn new() -> Self {
        Self {
            client: BinanceClient::new(),
        }
    }

    pub async fn start(self, mut rx: broadcast::Receiver<TradeSignal>) {
        info!("Starting Execution Service (Binance Connected)");

        // Log Initial Balance
        match self.client.get_account().await {
            Ok(info) => {
                info!("Binance Account Connected. Can Trade: {}", info.can_trade);
                for b in info
                    .balances
                    .iter()
                    .filter(|b| b.free.parse::<f64>().unwrap_or(0.0) > 0.0)
                {
                    info!("Balance: {} Free={} Locked={}", b.asset, b.free, b.locked);
                }
            }
            Err(e) => error!("Failed to fetch account info: {}", e),
        }

        loop {
            match rx.recv().await {
                Ok(signal) => {
                    info!("RECEIVED SIGNAL: {:?} - Executing...", signal);

                    // EXECUTE ORDER
                    // For safety in this phase, we might want to hardcode a small quantity or use the one from signal.
                    // Let's assume the signal provides a safe quantity.

                    match self
                        .client
                        .post_order(&signal.symbol, &signal.side, signal.quantity)
                        .await
                    {
                        Ok(order) => {
                            info!(
                                "ORDER EXECUTED: ID={}, Status={}",
                                order.order_id, order.status
                            );
                        }
                        Err(e) => {
                            error!("ORDER FAILED: {}", e);
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Execution service lagged: missed {} signals", n);
                }
                Err(_) => {
                    info!("Execution channel closed. Stopping service.");
                    break;
                }
            }
        }
    }
}
