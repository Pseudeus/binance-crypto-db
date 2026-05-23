use common::actors::{Actor, ActorType, ControlMessage};
use common::models::MarketEvent;
use futures_util::future::BoxFuture;
use std::sync::Arc;
use tokio::sync::{Semaphore, broadcast, mpsc};
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::RotatingPool;
use crate::repositories::aggtrade_repo::AggTradeWriterBuffer;
use crate::repositories::forceorder_repo::ForceOrderWriterBuffer;
use crate::repositories::klines_repo::KlineWriterBuffer;
use crate::repositories::markprice_repo::MarkPriceWriterBuffer;
use crate::repositories::openinterest_repo::OpenInterestWriterBuffer;
use crate::repositories::orderbook_repo::OrderBookWriterBuffer;
use crate::repositories::{
    AggTradeRepository, ForceOrderRepository, KlineRepository, MarkPriceRepository,
    OpenInterestRepository, OrderBookRepository,
};

const BUFFER_CAPACITY: usize = 1_000;

/// Internal state shared between the actor loop and worker tasks.
struct StorageState {
    write_semaphore: Arc<Semaphore>,

    // Repositories (Pre-instantiated "Singletons")
    aggtrade_repo: AggTradeWriterBuffer,
    orderbook_repo: OrderBookWriterBuffer,
    kline_repo: KlineWriterBuffer,
    markprice_repo: MarkPriceWriterBuffer,
    forceorder_repo: ForceOrderWriterBuffer,
    openinterest_repo: OpenInterestWriterBuffer,
}

impl StorageState {
    pub fn new(pool: Arc<RotatingPool>) -> Self {
        Self {
            write_semaphore: Arc::new(Semaphore::new(50)),

            aggtrade_repo: AggTradeWriterBuffer::new(
                AggTradeRepository::new(pool.clone()),
                BUFFER_CAPACITY,
            ),
            orderbook_repo: OrderBookWriterBuffer::new(
                OrderBookRepository::new(pool.clone()),
                BUFFER_CAPACITY,
            ),
            kline_repo: KlineWriterBuffer::new(KlineRepository::new(pool.clone()), BUFFER_CAPACITY),
            markprice_repo: MarkPriceWriterBuffer::new(
                MarkPriceRepository::new(pool.clone()),
                BUFFER_CAPACITY,
            ),
            forceorder_repo: ForceOrderWriterBuffer::new(
                ForceOrderRepository::new(pool.clone()),
                BUFFER_CAPACITY,
            ),
            openinterest_repo: OpenInterestWriterBuffer::new(
                OpenInterestRepository::new(pool),
                BUFFER_CAPACITY,
            ),
        }
    }

    async fn persist_event(&self, event: Arc<MarketEvent>) -> anyhow::Result<()> {
        use common::models::MarketEvent::*;

        match &*event {
            AggTrade(data) => {
                self.aggtrade_repo.push(data.clone()).await?;
            }
            OrderBook(data) => {
                self.orderbook_repo.push(data.clone()).await?;
            }
            Kline((data, _is_final)) => {
                self.kline_repo.push(data.clone()).await?;
            }
            MarkPrice(data) => {
                self.markprice_repo.push(data.clone()).await?;
            }
            ForceOrder(data) => {
                self.forceorder_repo.push(data.clone()).await?;
            }
            OpenInterest(data) => {
                self.openinterest_repo.push(data.clone()).await?;
            }
            AccountUpdate(_updates) => {}
            ExecutionReport(_report) => {}
        }
        Ok(())
    }
}

/// Actor responsible for persisting all market and account events to SQLite.
///
/// Subscribes to the central broadcast bus and uses `DataManager` for async I/O.
pub struct StorageActor {
    id: Uuid,
    state: Arc<StorageState>,
    market_rx: broadcast::Receiver<Arc<MarketEvent>>,
}

impl StorageActor {
    pub fn new(pool: Arc<RotatingPool>, market_rx: broadcast::Receiver<Arc<MarketEvent>>) -> Self {
        Self {
            id: Uuid::new_v4(),
            state: Arc::new(StorageState::new(pool)),
            market_rx,
        }
    }
}

impl Actor for StorageActor {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> ActorType {
        ActorType::StorageActor
    }

    fn run(
        &mut self,
        supervisor_tx: mpsc::Sender<ControlMessage>,
    ) -> BoxFuture<'_, anyhow::Result<()>> {
        let _heartbeat_handle = self.spawn_heartbeat(supervisor_tx.clone());

        info!("StorageActor started and listening for events.");
        Box::pin(async move {
            loop {
                match self.market_rx.recv().await {
                    Ok(event) => {
                        let state = self.state.clone();

                        match *event {
                            MarketEvent::AccountUpdate(_) | MarketEvent::ExecutionReport(_) => {
                                tokio::spawn(async move {
                                    if let Err(e) = state.persist_event(event).await {
                                        error!(
                                            "CRITICAL: Failed to persist account/execution event: {}",
                                            e
                                        );
                                    }
                                });
                            }
                            _ => {
                                //TODO: this is kinda a bottleneck to tokio runtime.
                                tokio::spawn(async move {
                                    let _permit = state.write_semaphore.acquire().await.ok();
                                    if let Err(e) = state.persist_event(event).await {
                                        error!("Failed to persist market event: {}", e);
                                    }
                                });
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("StorageActor lagged behind broadcast by {} messages", n);
                    }
                    Err(_) => {
                        info!("StorageActor stopping: broadcast channel closed.");
                        break;
                    }
                }
            }
            Ok(())
        })
    }
}
