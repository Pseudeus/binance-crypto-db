use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::bail;
use async_trait::async_trait;
use tokio::sync::{broadcast, mpsc};
use tokio::time;
use tracing::{debug, error, info, warn};

use crate::actors::{Actor, ActorType, ControlMessage};
use crate::db::RotatingPool;
use crate::models::AggTradeInsert;
use crate::repositories::AggTradeRepository;
use crate::services::market_gateway::MarketEvent;

pub struct AggTradeService {
    rotating_pool: Arc<RotatingPool>,
    trade_rx: broadcast::Receiver<Arc<MarketEvent>>,
}

#[async_trait]
impl Actor for AggTradeService {
    fn name(&self) -> ActorType {
        ActorType::AggTradeActor
    }

    async fn run(&mut self, supervisor_tx: mpsc::Sender<ControlMessage>) -> anyhow::Result<()> {
        let heartbeat_handle = {
            let tx = supervisor_tx.clone();
            let name = self.name();
            tokio::spawn(async move {
                loop {
                    if tx.send(ControlMessage::Heartbeat(name)).await.is_err() {
                        break;
                    }
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
            })
        };

        info!("Starting AggTrade Ingestion Service");

        let (db_tx, db_rx) = mpsc::channel(2000);

        tokio::spawn(Self::db_writer(self.rotating_pool.clone(), db_rx));

        loop {
            match self.trade_rx.recv().await {
                Ok(event_arc) => {
                    let event = &*event_arc;

                    if let MarketEvent::AggTrade(trade) = event {
                        if let Err(e) = db_tx.send(trade.to_owned()).await {
                            heartbeat_handle.abort();
                            supervisor_tx
                                .send(ControlMessage::Error(
                                    self.name(),
                                    format!("Failed to send to DB writer: {}", e),
                                ))
                                .await?;
                            bail!("Failed to send to DB writer: {}", e);
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("AggTrade service lagged: missed {} signals", n);
                }
                Err(_) => {
                    heartbeat_handle.abort();
                    supervisor_tx
                        .send(ControlMessage::Error(
                            self.name(),
                            format!("AggTrade channel closed unexpectedly."),
                        ))
                        .await?;
                    bail!("AggTrade channel closed unexpectedly.");
                }
            }
        }
    }
}

impl AggTradeService {
    pub fn new(
        rotating_pool: Arc<RotatingPool>,
        trade_rx: broadcast::Receiver<Arc<MarketEvent>>,
    ) -> Self {
        Self {
            rotating_pool,
            trade_rx,
        }
    }

    async fn db_writer(r_pool: Arc<RotatingPool>, mut trade_rx: mpsc::Receiver<AggTradeInsert>) {
        let mut buffer = Vec::with_capacity(1200);
        let mut last_flush = Instant::now();

        loop {
            tokio::select! {
                result = trade_rx.recv() => {
                    match result {
                        Some(trade) => {
                            buffer.push(trade);
                            if buffer.len() >= 1000 || last_flush.elapsed() >= Duration::from_secs(10) {
                                Self::flush_batch(&*r_pool, &buffer).await;
                                buffer.clear();
                                last_flush = Instant::now();
                            }
                        }
                        None => {
                            info!("DB Channel closed. Flushing remaining buffer.");
                            if !buffer.is_empty() {
                                Self::flush_batch(&*r_pool, &buffer).await;
                            }
                            break;
                        }
                    }
                }

                _ = time::sleep(Duration::from_millis(2000)) => {
                    if !buffer.is_empty() {
                        Self::flush_batch(&*r_pool, &buffer).await;
                        buffer.clear();
                        last_flush = Instant::now();
                    }
                }
            }
        }
    }

    async fn flush_batch(r_pool: &RotatingPool, batch: &[AggTradeInsert]) {
        let pool = loop {
            match r_pool.get().await {
                Ok(p) => {
                    break p;
                }
                Err(e) => {
                    error!("Failed to get DB pool: {}. Retrying...", e);
                    time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            };
        };

        if let Err(e) = AggTradeRepository::insert_batch(&pool, batch).await {
            error!("DB write failed: {}", e);
        } else {
            debug!("Wrote {} aggTrades to DB", batch.len());
        }
    }
}
