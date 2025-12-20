use binance_sdk::config::ConfigurationWebsocketStreams;
use binance_sdk::spot::SpotWsStreams;
use binance_sdk::spot::websocket_streams::{AggTradeParams, AggTradeParamsBuilder, TickerParams};

use std::sync::Arc;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt, TryFutureExt};
use sqlx::sqlite::SqlitePool;
use tokio::sync::broadcast;
use tokio::time;
use tokio_tungstenite::tungstenite::{Bytes, Message};
use tracing::{debug, error, info, warn};

use crate::db::RotatingPool;
use crate::models::AggTradeInsert;
use crate::remote::{AggTradeCombinedEvent, get_ws_base_url};
use crate::repositories::aggtrade_repo::AggTradeRepository;

pub struct AggTradeService {
    symbols: Vec<String>,
}

impl AggTradeService {
    pub fn new(symbols: &[&str]) -> Self {
        Self {
            symbols: symbols.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub async fn start(
        self,
        rotating_pool: Arc<RotatingPool>,
        trade_tx: broadcast::Sender<Arc<AggTradeInsert>>,
    ) {
        info!("Starting AggTrade Ingestion Service");

        let ws_streams_config = ConfigurationWebsocketStreams::builder().build().unwrap();
        let ws_streams_client = SpotWsStreams::production(ws_streams_config);

        let connection = ws_streams_client.connect().await.unwrap();

        let agg_params = AggTradeParams::builder("btcusdt".to_string())
            .symbol("bnbusdt")
            .build()
            .unwrap();

        let stream = connection.agg_trade(agg_params).await.unwrap();

        let ticker_params = TickerParams::builder("btcusdt".to_string())
            .symbol("ethusdt")
            .symbol("bnbusdt")
            .build()
            .unwrap();

        let other_stream = connection.ticker(ticker_params).await.unwrap();

        stream.on_message(|data| {
            info!("{:?}", data);
        });

        other_stream.on_message(|data| {
            info!("{:?}", data);
        });
        connection.disconnect().await.unwrap();

        let streams: Vec<String> = self
            .symbols
            .iter()
            .map(|s| format!("{}@aggTrade", s.to_lowercase()))
            .collect();

        // Use the configured WS URL
        let url = format!("{}{}", get_ws_base_url(), streams.join("/"));

        info!("Connecting to: {}", url);

        loop {
            match tokio_tungstenite::connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    let (mut write, mut read) = ws_stream.split();

                    debug!("Setting up db_writer");
                    let pool = match rotating_pool.get().await {
                        Ok(p) => p,
                        Err(e) => {
                            error!("Failed to get DB pool: {}. Retrying...", e);
                            time::sleep(Duration::from_secs(5)).await;
                            continue;
                        }
                    };

                    tokio::spawn(Self::db_writer(pool, trade_tx.subscribe()));

                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(ref text)) => {
                                match Self::parse_trade(text) {
                                    Ok(trade) => {
                                        // Broadcast the message. We don't care about the number of receivers here.
                                        let _ = trade_tx.send(Arc::new(trade));
                                    }
                                    Err(e) => error!("Failed to deserialize object: {}", e),
                                }
                            }
                            Ok(Message::Ping(pg)) => {
                                let _ = write.send(Message::Pong(pg));
                                continue;
                            }
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

    fn parse_trade(msg: &str) -> Result<AggTradeInsert, Box<dyn std::error::Error>> {
        let v = serde_json::from_str::<AggTradeCombinedEvent>(msg)?;
        Ok(v.to_insertable()?)
    }

    async fn db_writer(
        get_pool: SqlitePool,
        mut trade_rx: broadcast::Receiver<Arc<AggTradeInsert>>,
    ) {
        let mut buffer = Vec::with_capacity(1000);
        let mut last_flush = Instant::now();

        loop {
            tokio::select! {
                result = trade_rx.recv() => {
                    match result {
                        Ok(trade) => {
                            buffer.push((*trade).clone());
                            if buffer.len() >= 1000 || last_flush.elapsed() >= Duration::from_millis(5000) {
                                Self::flush_batch(&get_pool, &buffer).await;
                                buffer.clear();
                                last_flush = Instant::now();
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            warn!("DB Writer lagged! Skipped {} messages.", skipped);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            debug!("Broadcast channel closed.");
                            break;
                        }
                    }
                }

                _ = time::sleep(Duration::from_millis(2000)) => {
                    if !buffer.is_empty() {
                        Self::flush_batch(&get_pool, &buffer).await;
                        buffer.clear();
                        last_flush = Instant::now();
                    }
                }
            }
        }
    }

    async fn flush_batch(pool: &SqlitePool, batch: &[AggTradeInsert]) {
        debug!("Flushing {} elements to disk.", batch.len());
        if let Err(e) = AggTradeRepository::insert_batch(&pool, batch).await {
            error!("DB write failed: {}", e);
        } else {
            debug!("Wrote {} aggTrades", batch.len());
        }
    }
}
