use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::bail;
use async_trait::async_trait;
use common::{
    actors::{Actor, ActorType, ControlMessage},
    models::ForceOrderInsert,
};
use storage::{data_manager::DataManager, repositories::forceorder_repo::ForceOrderRepository};
use tokio::{
    sync::{broadcast, mpsc},
    time,
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::services::market_gateway::MarketEvent;

pub struct ForceOrderService {
    id: Uuid,
    rotating_pool: Arc<DataManager>,
    order_rx: broadcast::Receiver<Arc<MarketEvent>>,
}

#[async_trait]
impl Actor for ForceOrderService {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> ActorType {
        ActorType::ForceOrderActor
    }

    async fn run(&mut self, supervisor_tx: mpsc::Sender<ControlMessage>) -> anyhow::Result<()> {
        let heartbeat_handle = self.spawn_heartbeat(supervisor_tx.clone());

        info!("Starting ForceOrder Ingestion Service");

        let (db_tx, db_rx) = mpsc::channel(512);

        tokio::spawn(Self::db_writer(self.rotating_pool.clone(), db_rx));

        loop {
            match self.order_rx.recv().await {
                Ok(order_arc) => {
                    let event = &*order_arc;

                    if let MarketEvent::ForceOrder(order) = event {
                        if let Err(e) = db_tx.send(order.to_owned()).await {
                            heartbeat_handle.abort();
                            supervisor_tx.try_send(ControlMessage::Error(
                                self.id,
                                format!("{:?}: Failed to send to DB writer: {}", self.name(), e),
                            ))?;
                            bail!("Failed to send to DB writer: {}", e);
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("ForceOrder service lagged: missed {} signals", n);
                }
                Err(_) => {
                    heartbeat_handle.abort();
                    supervisor_tx
                        .send(ControlMessage::Error(
                            self.id,
                            format!("{:?}: ForceOrder channel closed unexpectedly.", self.name()),
                        ))
                        .await?;
                    bail!("ForceOrder channel closed unexpectedly.")
                }
            }
        }
    }
}

impl ForceOrderService {
    pub fn new(
        rotating_pool: Arc<DataManager>,
        order_rx: broadcast::Receiver<Arc<MarketEvent>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            rotating_pool,
            order_rx,
        }
    }

    async fn db_writer(r_pool: Arc<DataManager>, mut order_rx: mpsc::Receiver<ForceOrderInsert>) {
        let mut buffer = Vec::with_capacity(1024);
        let mut last_flush = Instant::now();

        loop {
            tokio::select! {
                result = order_rx.recv() => {
                    match result {
                        Some(order) => {
                            buffer.push(order);
                            if buffer.len() >= 512 || last_flush.elapsed() >= Duration::from_secs(10) {
                                Self::flush_batch(&*r_pool, &buffer).await;
                                buffer.clear();
                                last_flush = Instant::now();
                            }
                        }
                        None => {
                            info!("DB Channel closed. Flusing remaining buffer.");
                            if !buffer.is_empty() {
                                Self::flush_batch(&*r_pool, &buffer).await;
                            }
                            break;
                        }
                    }
                }

                _ = time::sleep(Duration::from_secs(5)) => {
                    if !buffer.is_empty() {
                        Self::flush_batch(&*r_pool, &buffer).await;
                        buffer.clear();
                        last_flush = Instant::now();
                    }
                }
            }
        }
    }

    async fn flush_batch(r_pool: &DataManager, batch: &[ForceOrderInsert]) {
        if let Err(e) = ForceOrderRepository::insert_batch(r_pool, batch).await {
            error!("DB write failed: {}", e);
        } else {
            debug!("Wrote {} ForceOrder to DB", batch.len());
        }
    }
}
