use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use sqlx::sqlite::SqlitePool;
use tokio::sync::broadcast;
use tokio::time;
use tokio_tungstenite::tungstenite::{Bytes, Message};
use tracing::{debug, error, info, warn};

use crate::models::OrderBookInsert;
use crate::remote::OrderBookCombinedEvent;
use crate::repositories::orderbook_repo::OrderBookRepository;
use crate::{db::RotatingPool, remote::BASE_URL};

pub struct OrderBookService {
    symbols: Vec<String>,
}

impl OrderBookService {
    pub fn new(symbols: &[&str]) -> Self {
        Self {
            symbols: symbols.iter().map(|s| s.to_lowercase()).collect(),
        }
    }

    pub async fn start(
        self,
        rotating_pool: Arc<RotatingPool>,
        order_tx: broadcast::Sender<Arc<OrderBookInsert>>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let stream_params: Vec<String> = self
            .symbols
            .iter()
            .map(|s| format!("{}@depth20@100ms", s))
            .collect();

        let url = format!("{}{}", BASE_URL, stream_params.join("/"));

        debug!("Connecting to web socket: {}", url);
        info!("Starting orderbook service for: {:?}", self.symbols);

        loop {
            match tokio_tungstenite::connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    let (mut write, mut read) = ws_stream.split();

                    debug!("Setting up db_writer");
                    let pool = rotating_pool.get().await?;
                    tokio::spawn(Self::db_writter(pool, order_tx.subscribe()));

                    debug!("Setting up heartbit");
                    tokio::spawn(async move {
                        let mut interval = time::interval(Duration::from_secs(20));
                        loop {
                            interval.tick().await;
                            let _ = write.send(Message::Ping(Bytes::new())).await;
                        }
                    });

                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(ref text)) => match Self::parse_trade(text) {
                                Ok(order) => {
                                    let _ = order_tx.send(Arc::new(order));
                                }
                                Err(e) => error!("Failed to deserialize object: {}", e),
                            },
                            Ok(Message::Close(_)) => {
                                debug!("Close message received");
                                break;
                            }
                            Err(e) => {
                                error!("WebSocket error: {}", e);
                                break;
                            }
                            _ => {
                                debug!("Unexpected message received, continuing...");
                                continue;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Connection failed: {}. Retrying in 2s...", e);
                    time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }

    fn parse_trade(msg: &str) -> Result<OrderBookInsert, Box<dyn std::error::Error>> {
        let v = serde_json::from_str::<OrderBookCombinedEvent>(msg)?;
        Ok(v.to_insertable()?)
    }

    async fn db_writter(pool: SqlitePool, mut order_rx: broadcast::Receiver<Arc<OrderBookInsert>>) {
        let mut buffer = Vec::with_capacity(200);
        let mut last_flush = Instant::now();

        loop {
            tokio::select! {
                result = order_rx.recv() => {
                    match result {
                        Ok(order) => {
                            buffer.push((*order).clone());
                            if buffer.len() >= 200 || last_flush.elapsed() >= Duration::from_millis(2000) {
                                Self::flush_batch(&pool, &buffer).await;
                                buffer.clear();
                                last_flush = Instant::now();
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            warn!("DB Writer lagged! Skipped {} orderbook messages.", skipped);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            debug!("Broadcast channel closed.");
                            break;
                        }
                    }
                }

                _ = time::sleep(Duration::from_millis(2000)) => {
                    if !buffer.is_empty() {
                        Self::flush_batch(&pool, &buffer).await;
                        buffer.clear();
                        last_flush = Instant::now();
                    }
                }
            }
        }
    }

    async fn flush_batch(pool: &SqlitePool, batch: &[OrderBookInsert]) {
        debug!("Flushing {} order_book entries to disk.", batch.len());
        if let Err(e) = OrderBookRepository::insert_batch(&pool, batch).await {
            error!("DB write failed: {}", e);
        } else {
            debug!("Wrote {} order_books", batch.len());
        }
    }
}

