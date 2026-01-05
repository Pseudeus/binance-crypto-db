use std::{sync::Arc, time::Duration};

use anyhow::bail;
use async_trait::async_trait;
use common::{
    actors::{Actor, ActorType, ControlMessage},
    models::OpenInterestInsert,
};
use storage::{data_manager::DataManager, repositories::openinterest_repo::OpenInterestRepository};
use tokio::{
    sync::{broadcast, mpsc},
    time::{self, Instant},
};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::services::market_gateway::MarketEvent;

pub struct OpenInterestService {
    id: Uuid,
    rotating_pool: Arc<DataManager>,
    interest_rx: broadcast::Receiver<Arc<MarketEvent>>,
}

#[async_trait]
impl Actor for OpenInterestService {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> ActorType {
        ActorType::OpenInterestActor
    }

    async fn run(&mut self, supervisor_tx: mpsc::Sender<ControlMessage>) -> anyhow::Result<()> {
        let heartbeat_handle = self.spawn_heartbeat(supervisor_tx.clone());

        info!("Starting ForceOrder Ingestion Service");
        let (db_tx, db_rx) = mpsc::channel(512);
        tokio::spawn(Self::db_writer(self.rotating_pool.clone(), db_rx));

        loop {
            match self.interest_rx.recv().await {
                Ok(interest_arc) => {
                    let event = &*interest_arc;

                    if let MarketEvent::OpenInterest(interest) = event {
                        if let Err(e) = db_tx.send(interest.to_owned()).await {
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
                    warn!("OpenInterest service lagged: missed {} signals", n);
                }
                Err(_) => {
                    heartbeat_handle.abort();
                    supervisor_tx
                        .send(ControlMessage::Error(
                            self.id,
                            format!(
                                "{:?}: OpenInterest channel closed unexpectedly.",
                                self.name()
                            ),
                        ))
                        .await?;
                    bail!("OpenInterest channel closed unexpectedly.")
                }
            }
        }
    }
}

impl OpenInterestService {
    pub fn new(
        rotating_pool: Arc<DataManager>,
        interest_rx: broadcast::Receiver<Arc<MarketEvent>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            rotating_pool,
            interest_rx,
        }
    }

    async fn db_writer(
        r_pool: Arc<DataManager>,
        mut interest_rx: mpsc::Receiver<OpenInterestInsert>,
    ) {
        let mut buffer = Vec::with_capacity(1024);
        let mut last_flush = Instant::now();

        loop {
            tokio::select! {
                result = interest_rx.recv() => {
                    match result {
                        Some(interest) => {
                            buffer.push(interest);
                            if buffer.len() >= 512 || last_flush.elapsed() >= Duration::from_secs(20) {
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

                _ = time::sleep(Duration::from_secs(10)) => {
                    if !buffer.is_empty() {
                        Self::flush_batch(&*r_pool, &buffer).await;
                        buffer.clear();
                        last_flush = Instant::now();
                    }
                }
            }
        }
    }

    async fn flush_batch(r_pool: &DataManager, batch: &[OpenInterestInsert]) {
        if let Err(e) = OpenInterestRepository::insert_batch(r_pool, batch).await {
            error!("DB write failed: {}", e);
        } else {
            debug!("Wrote {} OpenInterest to DB", batch.len());
        }
    }
}
