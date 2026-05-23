use common::actors::{Actor, ActorType, ControlMessage};
use common::models::TradeSignal;
use futures_util::future::BoxFuture;
use market_data::remote::BinanceClient;
use tokio::sync::{broadcast, mpsc};
use tracing::{error, info, warn};
use uuid::Uuid;

pub struct ExecutionService {
    id: Uuid,
    client: BinanceClient,
    rx: broadcast::Receiver<TradeSignal>,
}

impl ExecutionService {
    pub fn new(rx: broadcast::Receiver<TradeSignal>) -> Self {
        Self {
            id: Uuid::new_v4(),
            client: BinanceClient::new(),
            rx,
        }
    }
}

impl Actor for ExecutionService {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> ActorType {
        ActorType::ExecutionActor
    }

    fn run(
        &mut self,
        supervisor_tx: mpsc::Sender<ControlMessage>,
    ) -> BoxFuture<'_, anyhow::Result<()>> {
        let _heartbeat_handle = self.spawn_heartbeat(supervisor_tx.clone());
        info!("Starting Execution Service (Binance Connected)");

        Box::pin(async move {
            // Log Initial Balance
            match self.client.get_account().await {
                Ok(info) => {
                    info!(
                        "Binance Account Connected. Can Trade: {} (Maker: {}bps, Taker: {}bps)",
                        info.can_trade, info.maker_commission, info.taker_commission
                    );
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
                match self.rx.recv().await {
                    Ok(signal) => {
                        info!("RECEIVED SIGNAL: {:?} - Executing...", signal);

                        match self
                            .client
                            .post_order(&signal.symbol, &signal.side, *signal.quantity)
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
            Ok(())
        })
    }
}
