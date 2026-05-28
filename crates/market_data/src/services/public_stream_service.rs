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

use crate::remote::book_ticker_response::BookTickerEvent;
use crate::remote::{
    AggTradeCombinedEvent, AggTradeEvent, DepthPayload, KlineDataCombinedEvent,
    OrderBookCombinedEvent, get_ws_base_url,
};
use crate::traits::RemoteResponse;

/// Actor responsible for ingesting public market data from Binance WebSockets.
///
/// Ingests:
/// - aggTrade
/// - depth20
/// - kline_1m
///
/// # Complexity
/// - **Time Complexity**: O(N) where N is the number of streams subscribed to. Parsing is O(1) per message.
/// - **Memory Complexity**: O(M) where M is the size of the symbol list.
pub struct PublicStreamActor {
    id: Uuid,
    symbols: Vec<String>,
    market_tx: broadcast::Sender<Arc<MarketEvent>>,
}

impl PublicStreamActor {
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

        if raw_event.stream.ends_with("@aggTrade") {
            let specific_data = serde_json::from_value::<AggTradeEvent>(raw_event.data)?;
            return Ok(MarketEvent::AggTrade(
                AggTradeCombinedEvent {
                    data: specific_data,
                }
                .to_insertable()?,
            ));
        } else if raw_event.stream.ends_with("@bookTicker") {
            let specific_data = serde_json::from_value::<BookTickerEvent>(raw_event.data)?;
            return Ok(MarketEvent::BookTicker(specific_data.to_insertable()?));
        } else if raw_event.stream.ends_with("@depth20") {
            let specific_data = serde_json::from_value::<DepthPayload>(raw_event.data)?;
            return Ok(MarketEvent::OrderBook(
                OrderBookCombinedEvent {
                    stream: raw_event.stream,
                    data: specific_data,
                }
                .to_insertable()?,
            ));
        } else if raw_event.stream.ends_with("@kline_1m") {
            let specific_data = serde_json::from_value::<KlineDataCombinedEvent>(raw_event.data)?;
            return Ok(MarketEvent::Kline(specific_data.to_insertable()?));
        } else {
            anyhow::bail!("Unknown public stream data: {}", raw_event.stream);
        }
    }
}

impl Actor for PublicStreamActor {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> ActorType {
        ActorType::PublicStreamActor
    }

    fn run(
        &mut self,
        supervisor_tx: mpsc::Sender<ControlMessage>,
    ) -> BoxFuture<'_, anyhow::Result<()>> {
        let _heartbeat_handle = self.spawn_heartbeat(supervisor_tx.clone());

        let streams: Vec<String> = self
            .symbols
            .iter()
            .map(|s| {
                format!(
                    "{sl}@aggTrade/{sl}@depth20/{sl}@kline_1m/{sl}@bookTicker",
                    sl = s.to_lowercase()
                )
            })
            .collect();

        let url = format!("{}{}", get_ws_base_url(), streams.join("/"));

        info!("PublicStreamActor connecting to: {}", url);

        Box::pin(async move {
            loop {
                match tokio_tungstenite::connect_async(&url).await {
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
                                            warn!("Parse error in PublicStreamActor: {}", e);
                                            continue;
                                        }
                                    }
                                }
                                Ok(Message::Ping(_)) => {
                                    // Tungstenite handles pong automatically if we use the right API,
                                    // but here we are just reading.
                                }
                                Ok(Message::Close(_)) => break,
                                Err(e) => {
                                    error!("WebSocket error in PublicStreamActor: {}", e);
                                    break;
                                }
                                _ => continue,
                            }
                        }
                    }
                    Err(e) => {
                        error!(
                            "PublicStreamActor connection failed: {}. Retrying in 5s...",
                            e
                        );
                        //TODO: send a message to notify via telegram
                        time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_tungstenite::tungstenite::Message;

    /// Mock stream messages for testing PublicStreamActor parsing
    const MOCK_AGG_TRADE: &str = r#"{
        "e": "aggTrade",
        "E": 1234567890000,
        "s": "BTCUSDT",
        "t": 123456789012,
        "p": "100.00",
        "q": "0.5",
        "m": true,
        "M": true
    }"#;

    const MOCK_ORDER_BOOK: &str = r#"{
        "e": "depthUpdate",
        "E": 1234567890000,
        "s": "BTCUSDT",
        "u": 123456,
        "p": [
            ["100.00", "10.5"],
            ["100.01", "5.2"]
        ],
        "b": [
            ["100.02", "20.0"],
            ["100.03", "15.5"]
        ],
        "a": [
            ["100.00", "10.5"],
            ["100.01", "5.2"]
        ]
    }"#;

    const MOCK_KLINE: &str = r#"{
        "e": "kline",
        "E": 1234567890000,
        "k": {
            "t": 1234567800000,
            "T": 1234567859999,
            "s": "BTCUSDT",
            "i": "1m",
            "o": "100.10",
            "c": "100.50",
            "h": "100.60",
            "l": "100.00",
            "v": "1000.0",
            "n": 100,
            "x": true,
            "q": "100500.0",
            "V": 1000,
            "Q": "100500.0",
            "FO": "0",
            "r": "0.10"
        }
    }"#;

    #[test]
    fn test_parse_agg_trade_websocket_message() {
        // Test parsing of aggTrade message
        let _event = PublicStreamActor::parse_websocket_message(MOCK_AGG_TRADE).unwrap();
        // If we reach here, parsing succeeded
    }

    #[test]
    fn test_parse_order_book_websocket_message() {
        // Test parsing of order book depth20@100ms message
        let _event = PublicStreamActor::parse_websocket_message(MOCK_ORDER_BOOK).unwrap();
        // If we reach here, parsing succeeded
    }

    #[test]
    fn test_parse_kline_websocket_message() {
        // Test parsing of kline message
        let _event = PublicStreamActor::parse_websocket_message(MOCK_KLINE).unwrap();
        // If we reach here, parsing succeeded
    }

    #[test]
    fn test_parse_unknown_stream_fails() {
        // Test that unknown stream types fail gracefully
        let unknown = r#"{
            "e": "unknownEvent",
            "data": "some data"
        }"#;

        let result = PublicStreamActor::parse_websocket_message(unknown);
        assert!(result.is_err());
    }
}
