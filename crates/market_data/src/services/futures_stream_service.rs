use std::sync::Arc;
use std::time::Duration;

use common::actors::{Actor, ActorType, ControlMessage};
use common::models::MarketEvent;
use futures_util::StreamExt;
use futures_util::future::BoxFuture;
use tokio::sync::{broadcast, mpsc};
use tokio::time;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::remote::{
    forceorder_response::ForceOrderCombinedEvent, get_futures_ws_base_url,
    markprice_response::MarkPriceEvent,
};
use crate::traits::RemoteResponse;

/// Actor responsible for ingesting futures-specific market data from Binance.
///
/// Ingests:
/// - markPrice@1s
/// - forceOrder
///
/// # Complexity
/// - **Time Complexity**: O(N) where N is the number of futures streams.
/// - **Memory Complexity**: O(M) where M is the size of the symbol list.
pub struct FuturesStreamActor {
    id: Uuid,
    symbols: Vec<String>,
    market_tx: broadcast::Sender<Arc<MarketEvent>>,
}

impl FuturesStreamActor {
    pub fn new(symbols: &[&str], market_tx: broadcast::Sender<Arc<MarketEvent>>) -> Self {
        Self {
            id: Uuid::new_v4(),
            symbols: symbols.iter().map(|s| s.to_string()).collect(),
            market_tx,
        }
    }

    fn parse_websocket_message(json_input: &str) -> Result<MarketEvent, anyhow::Error> {
        use serde::Deserialize;
        use serde_json::Value;

        #[derive(Deserialize)]
        struct RawStreamEvent {
            stream: String,
            data: Value,
        }

        let raw_event: RawStreamEvent = serde_json::from_str(json_input)?;

        if raw_event.stream.ends_with("@markPrice@1s") {
            let specific_data = serde_json::from_value::<MarkPriceEvent>(raw_event.data)?;
            return Ok(MarketEvent::MarkPrice(specific_data.to_insertable()?));
        } else if raw_event.stream.ends_with("@forceOrder") {
            let specific_data = serde_json::from_value::<ForceOrderCombinedEvent>(raw_event.data)?;
            return Ok(MarketEvent::ForceOrder(specific_data.to_insertable()?));
        } else {
            anyhow::bail!("Unknown futures stream data: {}", raw_event.stream);
        }
    }
}

impl Actor for FuturesStreamActor {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> ActorType {
        ActorType::FuturesStreamActor
    }

    fn run(
        &mut self,
        supervisor_tx: mpsc::Sender<ControlMessage>,
    ) -> BoxFuture<'_, anyhow::Result<()>> {
        let _heartbeat_handle = self.spawn_heartbeat(supervisor_tx.clone());

        let fstreams: Vec<String> = self
            .symbols
            .iter()
            .map(|s| format!("{sl}@forceOrder/{sl}@markPrice@1s", sl = s.to_lowercase()))
            .collect();

        let furl = format!("{}{}", get_futures_ws_base_url(), fstreams.join("/"));

        info!("FuturesStreamActor connecting to: {}", furl);

        Box::pin(async move {
            loop {
                match tokio_tungstenite::connect_async(&furl).await {
                    Ok((ws_stream, _)) => {
                        let (mut _write, mut read) = ws_stream.split();

                        while let Some(msg) = read.next().await {
                            match msg {
                                Ok(Message::Text(ref text)) => {
                                    match Self::parse_websocket_message(text) {
                                        Ok(event) => {
                                            let _ = self.market_tx.send(Arc::new(event));
                                        }
                                        Err(e) => {
                                            warn!("Parse error in FuturesStreamActor: {}", e);
                                            continue;
                                        }
                                    }
                                }
                                Ok(Message::Close(_)) => break,
                                Err(e) => {
                                    error!("WebSocket error in FuturesStreamActor: {}", e);
                                    break;
                                }
                                _ => continue,
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            "FuturesStreamActor connection failed: {}. Retrying in 5s...",
                            e
                        );
                        time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        })
    }
}
