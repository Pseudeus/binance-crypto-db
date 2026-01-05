use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::bail;
use async_trait::async_trait;
use common::{
    actors::{Actor, ActorType, ControlMessage},
    models::MarkPriceInsert,
};
use storage::{data_manager::DataManager, repositories::markprice_repo::MarkPriceRepository};
use tokio::sync::{broadcast, mpsc};
use tokio::time;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::services::market_gateway::MarketEvent;

pub struct MarkPriceService {
    id: Uuid,
    rotating_pool: Arc<DataManager>,
    mark_rx: broadcast::Receiver<Arc<MarketEvent>>,
}

#[async_trait]
impl Actor for MarkPriceService {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> ActorType {
        ActorType::MarkPriceActor
    }

    async fn run(&mut self, supervisor_tx: mpsc::Sender<ControlMessage>) -> anyhow::Result<()> {
        let heartbeat_handle = self.spawn_heartbeat(supervisor_tx.clone());

        info!("Starting MarkPrice Ingestion Service");

        let (db_tx, db_rx) = mpsc::channel(1200);

        tokio::spawn(Self::db_writer(self.rotating_pool.clone(), db_rx));

        loop {
            match self.mark_rx.recv().await {
                Ok(event_mark) => {
                    let event = &*event_mark;

                    if let MarketEvent::MarkPrice(mark) = event {
                        if let Err(e) = db_tx.send(mark.to_owned()).await {
                            heartbeat_handle.abort();
                            supervisor_tx.try_send(ControlMessage::Error(
                                self.id,
                                format!("{:?} Failed to send to DB writer: {}", self.name(), e),
                            ))?;
                            bail!("Failed to send to DB writer: {}", e);
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("MarkPrice service lagged: missed {} signals", n);
                }
                Err(_) => {
                    heartbeat_handle.abort();
                    supervisor_tx.try_send(ControlMessage::Error(
                        self.id,
                        format!("{:?}: MarkPrice channel closed unexpedtedly.", self.name()),
                    ))?;
                    bail!("MarkPrice channel closed unexpectedly.")
                }
            }
        }
    }
}

impl MarkPriceService {
    pub fn new(
        rotating_pool: Arc<DataManager>,
        mark_rx: broadcast::Receiver<Arc<MarketEvent>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            rotating_pool,
            mark_rx,
        }
    }

    async fn db_writer(r_pool: Arc<DataManager>, mut mark_rx: mpsc::Receiver<MarkPriceInsert>) {
        let mut buffer = Vec::with_capacity(256);
        let mut last_flush = Instant::now();

        loop {
            tokio::select! {
                result = mark_rx.recv() => {
                    match result {
                        Some(mark) => {
                            buffer.push(mark);
                            if buffer.len() >= 300 || last_flush.elapsed() >= Duration::from_secs(10) {
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

                _ = time::sleep(Duration::from_secs(20)) => {
                    if !buffer.is_empty() {
                        Self::flush_batch(&*r_pool, &buffer).await;
                        buffer.clear();
                        last_flush = Instant::now();
                    }

                }
            }
        }
    }

    async fn flush_batch(r_pool: &DataManager, batch: &[MarkPriceInsert]) {
        if let Err(e) = MarkPriceRepository::insert_batch(r_pool, batch).await {
            error!("DB write failed: {}", e);
        } else {
            debug!("Wrote {} MarkPrices to DB", batch.len());
        }
    }
}
