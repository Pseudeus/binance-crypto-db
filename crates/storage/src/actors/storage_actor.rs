use common::actors::{Actor, ActorType, ControlMessage};
use common::models::{
    AggTradeInsert, ForceOrderInsert, KlineInsert, MarketEvent, MarkPriceInsert, OpenInterestInsert,
    OrderBookInsert,
};
use futures_util::future::BoxFuture;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use tracing::{info, warn};
use uuid::Uuid;

use crate::db::DbWriteBatch;
use crate::storage_write_buffer::StorageWriteBuffer;

const BUFFER_CAPACITY: usize = 5_000;

/// Actor responsible for persisting all market events to SQLite via a dedicated writer thread.
///
/// Subscribes to the central broadcast bus, aggregates events into typed in-memory buffers,
/// and flushes batches to the `DbWriter` worker thread on dual triggers:
/// 1. When buffer capacity reaches `BUFFER_CAPACITY` (e.g. 5,000 items).
/// 2. When a 1.0-second interval ticker elapses for any non-empty buffer.
pub struct StorageActor {
    id: Uuid,
    db_sender: mpsc::Sender<DbWriteBatch>,
    market_rx: broadcast::Receiver<Arc<MarketEvent>>,

    aggtrade_buf: StorageWriteBuffer<AggTradeInsert>,
    orderbook_buf: StorageWriteBuffer<OrderBookInsert>,
    kline_buf: StorageWriteBuffer<KlineInsert>,
    markprice_buf: StorageWriteBuffer<MarkPriceInsert>,
    forceorder_buf: StorageWriteBuffer<ForceOrderInsert>,
    openinterest_buf: StorageWriteBuffer<OpenInterestInsert>,
}

impl StorageActor {
    pub fn new(
        db_sender: mpsc::Sender<DbWriteBatch>,
        market_rx: broadcast::Receiver<Arc<MarketEvent>>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            db_sender,
            market_rx,
            aggtrade_buf: StorageWriteBuffer::new(BUFFER_CAPACITY),
            orderbook_buf: StorageWriteBuffer::new(BUFFER_CAPACITY),
            kline_buf: StorageWriteBuffer::new(BUFFER_CAPACITY),
            markprice_buf: StorageWriteBuffer::new(BUFFER_CAPACITY),
            forceorder_buf: StorageWriteBuffer::new(BUFFER_CAPACITY),
            openinterest_buf: StorageWriteBuffer::new(BUFFER_CAPACITY),
        }
    }

    async fn handle_event(&mut self, event: Arc<MarketEvent>) {
        use common::models::MarketEvent::*;

        match &*event {
            AggTrade(data) => {
                if let Some(batch) = self.aggtrade_buf.push(data.clone()) {
                    let _ = self.db_sender.send(DbWriteBatch::AggTrade(batch)).await;
                }
            }
            OrderBook(data) => {
                if let Some(batch) = self.orderbook_buf.push(data.clone()) {
                    let _ = self.db_sender.send(DbWriteBatch::OrderBook(batch)).await;
                }
            }
            Kline((data, _is_final)) => {
                if let Some(batch) = self.kline_buf.push(data.clone()) {
                    let _ = self.db_sender.send(DbWriteBatch::Kline(batch)).await;
                }
            }
            MarkPrice(data) => {
                if let Some(batch) = self.markprice_buf.push(data.clone()) {
                    let _ = self.db_sender.send(DbWriteBatch::MarkPrice(batch)).await;
                }
            }
            ForceOrder(data) => {
                if let Some(batch) = self.forceorder_buf.push(data.clone()) {
                    let _ = self.db_sender.send(DbWriteBatch::ForceOrder(batch)).await;
                }
            }
            OpenInterest(data) => {
                if let Some(batch) = self.openinterest_buf.push(data.clone()) {
                    let _ = self.db_sender.send(DbWriteBatch::OpenInterest(batch)).await;
                }
            }
            AccountUpdate(_) => {}
            ExecutionReport(_) => {}
        }
    }

    async fn flush_all(&mut self) {
        if let Some(batch) = self.aggtrade_buf.flush() {
            let _ = self.db_sender.send(DbWriteBatch::AggTrade(batch)).await;
        }
        if let Some(batch) = self.orderbook_buf.flush() {
            let _ = self.db_sender.send(DbWriteBatch::OrderBook(batch)).await;
        }
        if let Some(batch) = self.kline_buf.flush() {
            let _ = self.db_sender.send(DbWriteBatch::Kline(batch)).await;
        }
        if let Some(batch) = self.markprice_buf.flush() {
            let _ = self.db_sender.send(DbWriteBatch::MarkPrice(batch)).await;
        }
        if let Some(batch) = self.forceorder_buf.flush() {
            let _ = self.db_sender.send(DbWriteBatch::ForceOrder(batch)).await;
        }
        if let Some(batch) = self.openinterest_buf.flush() {
            let _ = self.db_sender.send(DbWriteBatch::OpenInterest(batch)).await;
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

        info!("StorageActor started and listening for events with dedicated DB writer.");
        Box::pin(async move {
            let mut flush_timer = tokio::time::interval(std::time::Duration::from_secs(1));
            flush_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    msg = self.market_rx.recv() => {
                        match msg {
                            Ok(event) => {
                                self.handle_event(event).await;
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
                    _ = flush_timer.tick() => {
                        self.flush_all().await;
                    }
                }
            }

            self.flush_all().await;
            info!("StorageActor stopped, buffers flushed.");
            Ok(())
        })
    }
}
