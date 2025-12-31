use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::bail;
use async_trait::async_trait;
use tokio::sync::{broadcast, mpsc};
use tokio::time;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::services::market_gateway::MarketEvent;
use common::actors::{Actor, ActorType, ControlMessage};
use common::models::KlineInsert;
use storage::db::RotatingPool;
use storage::repositories::KlinesRepository;

pub struct KlinesService {
    id: Uuid,
    rotating_pool: Arc<RotatingPool>,
    kline_rx: broadcast::Receiver<Arc<MarketEvent>>,
}

#[async_trait]
impl Actor for KlinesService {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> ActorType {
        ActorType::KlinesActor
    }

    async fn run(&mut self, supervisor_tx: mpsc::Sender<ControlMessage>) -> anyhow::Result<()> {
        let heartbeat_handle = self.spawn_heartbeat(supervisor_tx.clone());

        info!("Starting Klines Ingestion Service");

        let (db_tx, db_rx) = mpsc::channel(600);

        tokio::spawn(Self::db_writer(self.rotating_pool.clone(), db_rx));

        loop {
            match self.kline_rx.recv().await {
                Ok(event_arc) => {
                    let event = &*event_arc;

                    if let MarketEvent::Kline((kline, closed)) = event {
                        if let Err(e) = db_tx.send((kline.to_owned(), *closed)).await {
                            let err_msg = format!("Failed to send to DB writer: {}", e);
                            heartbeat_handle.abort();
                            supervisor_tx
                                .send(ControlMessage::Error(self.id, err_msg.clone()))
                                .await?;
                            bail!(err_msg);
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("Klines service lagged: missed {} signals", n);
                }
                Err(_) => {
                    let err_msg = format!("Kline channel closed. Stopping service.");
                    heartbeat_handle.abort();
                    supervisor_tx
                        .send(ControlMessage::Error(self.id, err_msg.clone()))
                        .await?;
                    bail!(err_msg);
                }
            }
        }
    }
}

impl KlinesService {
    pub fn new(
        rotating_pool: Arc<RotatingPool>,
        kline_rx: broadcast::Receiver<Arc<MarketEvent>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            rotating_pool,
            kline_rx,
        }
    }

    async fn db_writer(
        r_pool: Arc<RotatingPool>,
        mut kline_rx: mpsc::Receiver<(KlineInsert, bool)>,
    ) {
        let mut buffer = Vec::with_capacity(300);
        let mut last_flush = Instant::now();

        loop {
            tokio::select! {
                result = kline_rx.recv() => {
                    match result {
                        Some((kline, closed)) => {
                            if closed {
                                buffer.push(kline);
                            }

                            if buffer.len() >= 300 || last_flush.elapsed() >= Duration::from_secs(20) {
                                Self::flush_batch(&r_pool, &buffer).await;
                                buffer.clear();
                                last_flush = Instant::now();
                            }
                        }
                        None => {
                            info!("DB Channel closed. Flushing remaining buffer.");
                            if !buffer.is_empty() {
                                Self::flush_batch(&r_pool, &buffer).await;
                            }
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn flush_batch(r_pool: &RotatingPool, batch: &[KlineInsert]) {
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

        if let Err(e) = KlinesRepository::insert_batch(&pool, batch).await {
            error!("DB write failed: {}", e);
        } else {
            debug!("Wrote {} klines to DB", batch.len());
        }
    }
}
