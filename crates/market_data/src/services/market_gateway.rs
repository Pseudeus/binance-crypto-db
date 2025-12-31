use std::sync::Arc;
use std::time::Duration;

use anyhow::bail;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::{
    sync::{broadcast, mpsc},
    time,
};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info};

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::remote::{
    AggTradeCombinedEvent, AggTradeEvent, DepthPayload, KlineDataCombinedEvent,
    OrderBookCombinedEvent, get_ws_base_url,
};

use common::{
    actors::{Actor, ActorType, ControlMessage},
    models::{AggTradeInsert, KlineInsert, OrderBookInsert},
};

pub enum MarketEvent {
    AggTrade(AggTradeInsert),
    OrderBook(OrderBookInsert),
    Kline((KlineInsert, bool)),
}

#[derive(Deserialize)]
struct RawStreamEvent {
    stream: String,
    data: Value, // Delay parsing this until we know what it is!
}

pub struct MarketGateway {
    id: Uuid,
    symbols: Vec<String>,
    market_tx: broadcast::Sender<Arc<MarketEvent>>,
}

#[async_trait]
impl Actor for MarketGateway {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> ActorType {
        ActorType::GatewayActor
    }

    async fn run(&mut self, supervisor_tx: mpsc::Sender<ControlMessage>) -> anyhow::Result<()> {
        let heartbeat_handle = self.spawn_heartbeat(supervisor_tx.clone());

        let streams: Vec<String> = self
            .symbols
            .iter()
            .map(|s| {
                format!(
                    "{sl}@aggTrade/{sl}@depth20@100ms/{sl}@kline_1h/{sl}@kline_1m/{sl}@kline_1s",
                    sl = s.to_lowercase()
                )
            })
            .collect();

        let url = format!("{}{}", get_ws_base_url(), streams.join("/"));

        info!("Connecting to: {}", url);

        loop {
            match tokio_tungstenite::connect_async(&url).await {
                Ok((ws_stream, _)) => {
                    let (mut write, mut read) = ws_stream.split();

                    while let Some(msg) = read.next().await {
                        match msg {
                            Ok(Message::Text(ref text)) => {
                                match Self::parse_websocket_message(&text) {
                                    Ok(stream) => {
                                        let _ = self.market_tx.send(Arc::new(stream));
                                    }
                                    Err(e) => {
                                        supervisor_tx
                                            .send(ControlMessage::Error(
                                                self.id,
                                                format!("Unknown socket response: {}", e),
                                            ))
                                            .await?;
                                        continue;
                                    }
                                }
                            }
                            Ok(Message::Ping(pg)) => {
                                let _ = write.send(Message::Pong(pg));
                                info!("Ping - Pong message sent to websocket.");
                                continue;
                            }
                            Ok(Message::Close(_)) => {
                                debug!("Close message received");
                                heartbeat_handle.abort();
                                break;
                            }
                            Err(e) => {
                                error!("WebSocket error: {}", e);
                                heartbeat_handle.abort();
                                break;
                            }
                            _ => {
                                supervisor_tx
                                    .send(ControlMessage::Error(
                                        self.id,
                                        "Unexpected message received, continuing...".to_string(),
                                    ))
                                    .await?;
                                continue;
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Connection failed: {}. Retrying in 2s...", e);

                    supervisor_tx
                        .send(ControlMessage::Error(
                            self.id,
                            format!("Connection failed: {}. Retrying in 2s...", e),
                        ))
                        .await?;
                    time::sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
}

impl MarketGateway {
    pub fn new(symbols: &[&str], market_tx: broadcast::Sender<Arc<MarketEvent>>) -> Self {
        Self {
            id: Uuid::new_v4(),
            symbols: symbols.iter().map(|s| s.to_string()).collect(),
            market_tx,
        }
    }

    fn parse_websocket_message(json_input: &str) -> Result<MarketEvent, anyhow::Error> {
        let raw_event: RawStreamEvent = serde_json::from_str(json_input)?;

        if raw_event.stream.ends_with("@aggTrade") {
            let specific_data = serde_json::from_value::<AggTradeEvent>(raw_event.data)?;

            return Ok(MarketEvent::AggTrade(
                AggTradeCombinedEvent {
                    data: specific_data,
                }
                .to_insertable()?,
            ));
        } else if raw_event.stream.ends_with("@depth20@100ms") {
            let specific_data = serde_json::from_value::<DepthPayload>(raw_event.data)?;

            return Ok(MarketEvent::OrderBook(
                OrderBookCombinedEvent {
                    stream: raw_event.stream,
                    data: specific_data,
                }
                .to_insertable()?,
            ));
        } else if raw_event.stream.contains("@kline") {
            let specific_data = serde_json::from_value::<KlineDataCombinedEvent>(raw_event.data)?;

            return Ok(MarketEvent::Kline(specific_data.to_insertable()?));
        } else {
            bail!("Unknown received data.");
        }
    }
}
