use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::bail;
use async_trait::async_trait;
use tokio::sync::{broadcast, mpsc};
use tokio::time;
use tracing::{debug, error, info, warn};

use common::actors::{Actor, ActorType, ControlMessage};
use storage::db::RotatingPool;
use common::models::OrderBookInsert;
use storage::repositories::OrderBookRepository;
use crate::services::market_gateway::MarketEvent;

pub struct OrderBookService {
    rotating_pool: Arc<RotatingPool>,
    order_tx: broadcast::Receiver<Arc<MarketEvent>>,
}

#[async_trait]
impl Actor for OrderBookService {
    fn name(&self) -> ActorType {
        ActorType::OrderBookActor
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

        info!("Starting OrderBook Ingestion Service");

        let (db_tx, db_rx) = mpsc::channel(2000);

        tokio::spawn(Self::db_writer(self.rotating_pool.clone(), db_rx));

        loop {
            match self.order_tx.recv().await {
                Ok(order_arc) => {
                    let event = &*order_arc;

                    if let MarketEvent::OrderBook(order) = event {
                        if let Err(e) = db_tx.send(order.to_owned()).await {
                            let err_msg = format!("Failed to send to DB writer: {}", e);
                            heartbeat_handle.abort();
                            supervisor_tx
                                .send(ControlMessage::Error(self.name(), err_msg.clone()))
                                .await?;
                            bail!(err_msg);
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("OrderBook service lagged: missed {} signals", n);
                }
                Err(_) => {
                    let err_msg = format!("OrderBook channel closed unexpectedly.");
                    heartbeat_handle.abort();
                    supervisor_tx
                        .send(ControlMessage::Error(self.name(), err_msg.clone()))
                        .await?;
                    bail!(err_msg);
                }
            }
        }
    }
}

impl OrderBookService {
    pub fn new(
        rotating_pool: Arc<RotatingPool>,
        order_tx: broadcast::Receiver<Arc<MarketEvent>>,
    ) -> Self {
        Self {
            rotating_pool,
            order_tx,
        }
    }

    async fn db_writer(
        rotating_pool: Arc<RotatingPool>,
        mut order_rx: mpsc::Receiver<OrderBookInsert>,
    ) {
        let mut buffer = Vec::with_capacity(750);
        let mut last_flush = Instant::now();

        loop {
            tokio::select! {
                result = order_rx.recv() => {
                    match result {
                        Some(order) => {
                            buffer.push(order);
                            if buffer.len() >= 600 || last_flush.elapsed() >= Duration::from_secs(5) {
                                Self::flush_batch(&*rotating_pool, &buffer).await;
                                buffer.clear();
                                last_flush = Instant::now();
                            }
                        }
                        None => {
                            info!("DB Channel closed. Flusing remaining buffer.");
                            if !buffer.is_empty() {
                                Self::flush_batch(&*rotating_pool, &buffer).await;
                            }
                            break;
                        }
                    }
                }

                _ = time::sleep(Duration::from_secs(5)) => {
                    if !buffer.is_empty() {
                        Self::flush_batch(&*rotating_pool, &buffer).await;
                        buffer.clear();
                        last_flush = Instant::now();
                    }
                }
            }
        }
    }

    async fn flush_batch(rotating_pool: &RotatingPool, batch: &[OrderBookInsert]) {
        let pool = loop {
            match rotating_pool.get().await {
                Ok(p) => break p,
                Err(e) => {
                    error!("Failed to get DB pool: {}. Retrying...", e);
                    time::sleep(Duration::from_secs(5)).await;
                    continue;
                }
            }
        };

        if let Err(e) = OrderBookRepository::insert_batch(&pool, batch).await {
            error!("DB write failed: {}", e);
        } else {
            debug!("Wrote {} order_books to DB.", batch.len());
        }
    }
}
