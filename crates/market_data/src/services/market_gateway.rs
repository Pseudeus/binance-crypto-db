use std::error::Error;
use std::sync::Arc;

use anyhow::bail;
use async_trait::async_trait;
use common::models::{ForceOrderInsert, MarkPriceInsert, OpenInterestInsert};
use futures_util::{SinkExt, StreamExt};
use tokio::{
    sync::{broadcast, mpsc},
    time::{self, Duration},
};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, info, warn};

use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::remote::{binance_poller::BinancePoller, markprice_response::MarkPriceEvent};
use crate::remote::{forceorder_response::ForceOrderCombinedEvent, get_futures_ws_base_url};
use crate::{
    remote::{
        AggTradeCombinedEvent, AggTradeEvent, DepthPayload, KlineDataCombinedEvent,
        OrderBookCombinedEvent, get_ws_base_url,
    },
    traits::RemoteResponse,
};

use common::{
    actors::{Actor, ActorType, ControlMessage},
    models::{AggTradeInsert, KlineInsert, OrderBookInsert},
};

pub enum MarketEvent {
    AggTrade(AggTradeInsert),
    OrderBook(OrderBookInsert),
    Kline((KlineInsert, bool)),
    MarkPrice(MarkPriceInsert),
    ForceOrder(ForceOrderInsert),
    OpenInterest(OpenInterestInsert),
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

        let fstreams: Vec<String> = self
            .symbols
            .iter()
            .map(|s| format!("{sl}@forceOrder/{sl}@markPrice@1s", sl = s.to_lowercase()))
            .collect();

        let url = format!("{}{}", get_ws_base_url(), streams.join("/"));
        let furl = format!("{}{}", get_futures_ws_base_url(), fstreams.join("/"));

        tokio::select! {
            _ = self.websocket_connection(&url, supervisor_tx.clone()) => {
                heartbeat_handle.abort()
            }
            _ = self.websocket_connection(&furl, supervisor_tx.clone()) => {
                heartbeat_handle.abort()
            }
            _ = self.oi_connection() => {
                heartbeat_handle.abort();
            }
        }
        Ok(())
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

    async fn oi_connection(&self) -> anyhow::Result<()> {
        let poller = BinancePoller::new();

        loop {
            let general_result = poller.fetch_all_open_interest(&self.symbols).await;

            if let Err(e) = general_result {
                bail!("OI connection error: {}", e);
            } else {
                let results = general_result.unwrap();

                results.into_iter().for_each(|res| match res {
                    Ok(data) => {
                        let _ = self
                            .market_tx
                            .send(Arc::new(MarketEvent::OpenInterest(data)));
                    }
                    Err(e) => {
                        warn!("Failed to fetch OI: {}", e);
                    }
                });
            }
        }
    }

    async fn websocket_connection(
        &self,
        url: &str,
        supervisor_tx: mpsc::Sender<ControlMessage>,
    ) -> Result<(), Box<dyn Error>> {
        info!("Connecting to: {}", url);
        loop {
            match tokio_tungstenite::connect_async(url).await {
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
                                break;
                            }
                            Err(e) => {
                                error!("WebSocket error: {}", e);
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
        } else if raw_event.stream.ends_with("@markPrice@1s") {
            let specific_data = serde_json::from_value::<MarkPriceEvent>(raw_event.data)?;

            return Ok(MarketEvent::MarkPrice(specific_data.to_insertable()?));
        } else if raw_event.stream.ends_with("@forceOrder") {
            let specific_data = serde_json::from_value::<ForceOrderCombinedEvent>(raw_event.data)?;

            return Ok(MarketEvent::ForceOrder(specific_data.to_insertable()?));
        } else {
            bail!("Unknown received data.");
        }
    }
}
