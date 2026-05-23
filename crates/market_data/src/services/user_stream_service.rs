use std::sync::Arc;
use std::time::Duration;

use common::actors::{Actor, ActorType, ControlMessage};
use common::models::MarketEvent;
use futures_util::future::BoxFuture;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::{broadcast, mpsc};
use tokio::time;
use tokio_tungstenite::tungstenite::Message;
use tracing::{error, info};
use uuid::Uuid;

use crate::remote::binance_client::BinanceClient;

/// Actor responsible for ingesting account-specific data (User Data Stream).
///
/// Ingests:
/// - outboundAccountPosition (AccountUpdate)
/// - executionReport
///
/// # Complexity
/// - **Time Complexity**: O(1) for parsing and broadcasting messages.
/// - **Memory Complexity**: O(1) - maintains state for ListenKey only.
pub struct UserStreamActor {
    id: Uuid,
    market_tx: broadcast::Sender<Arc<MarketEvent>>,
    client: BinanceClient,
}

impl UserStreamActor {
    pub fn new(market_tx: broadcast::Sender<Arc<MarketEvent>>) -> Self {
        Self {
            id: Uuid::new_v4(),
            market_tx,
            client: BinanceClient::new(),
        }
    }

    fn parse_user_stream_message(json_input: &str) -> Result<MarketEvent, anyhow::Error> {
        use serde_json::Value;
        let v: Value = serde_json::from_str(json_input)?;
        let event_type = v["e"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Missing event type"))?;

        match event_type {
            "outboundAccountPosition" => {
                use crate::traits::RemoteResponse;
                let event = serde_json::from_str::<
                    crate::remote::user_stream_response::AccountPositionEvent,
                >(json_input)?;
                let insertable = event.to_insertable()?;
                info!("USER STREAM: Account Position Update: {:?}", insertable);
                Ok(MarketEvent::AccountUpdate(insertable))
            }
            "executionReport" => {
                use crate::traits::RemoteResponse;
                let event = serde_json::from_str::<
                    crate::remote::user_stream_response::ExecutionReportEvent,
                >(json_input)?;
                let insertable = event.to_insertable()?;
                info!("USER STREAM: Execution Report: {:?}", insertable);
                Ok(MarketEvent::ExecutionReport(insertable))
            }
            _ => anyhow::bail!("Unknown user stream event: {}", event_type),
        }
    }
}

impl Actor for UserStreamActor {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> ActorType {
        ActorType::UserStreamActor
    }

    fn run(
        &mut self,
        supervisor_tx: mpsc::Sender<ControlMessage>,
    ) -> BoxFuture<'_, anyhow::Result<()>> {
        let _heartbeat_handle = self.spawn_heartbeat(supervisor_tx.clone());

        info!("Starting UserStreamActor Loop");
        Box::pin(async move {
            loop {
                match self.client.get_listen_key().await {
                    Ok(listen_key) => {
                        let url = format!("wss://stream.binance.com:9443/ws/{}", listen_key);
                        let mut refresh_interval = time::interval(Duration::from_secs(30 * 60)); // 30 mins

                        match tokio_tungstenite::connect_async(&url).await {
                            Ok((ws_stream, _)) => {
                                info!("UserStreamActor connected");
                                let (mut write, mut read) = ws_stream.split();

                                loop {
                                    tokio::select! {
                                        msg = read.next() => {
                                            match msg {
                                                Some(Ok(Message::Text(text))) => {
                                                    if let Ok(event) = Self::parse_user_stream_message(&text) {
                                                        let _ = self.market_tx.send(Arc::new(event));
                                                    }
                                                }
                                                Some(Ok(Message::Ping(pg))) => {
                                                    let _ = write.send(Message::Pong(pg)).await;
                                                }
                                                Some(Ok(Message::Close(_))) => break,
                                                Some(Err(e)) => {
                                                    error!("UserStreamActor Error: {}", e);
                                                    break;
                                                }
                                                None => break,
                                                _ => {}
                                            }
                                        }
                                        _ = refresh_interval.tick() => {
                                            if let Err(e) = self.client.refresh_listen_key(&listen_key).await {
                                                error!("UserStreamActor failed to refresh ListenKey: {}", e);
                                                break;
                                            }
                                            info!("UserStreamActor ListenKey refreshed");
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!(
                                    "UserStreamActor connection failed: {}. Retrying in 5s...",
                                    e
                                );
                                time::sleep(Duration::from_secs(5)).await;
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            "UserStreamActor failed to get ListenKey: {}. Retrying in 5s...",
                            e
                        );
                        time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        })
    }
}
