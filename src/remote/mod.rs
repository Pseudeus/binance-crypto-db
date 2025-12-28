use std::env;

pub mod aggtrade_response;
pub mod binance_client;
pub mod kline_response;
pub mod orderbook_response;

pub use aggtrade_response::{AggTradeCombinedEvent, AggTradeEvent};
pub use binance_client::BinanceClient;
pub use kline_response::KlineDataCombinedEvent;
pub use orderbook_response::{DepthPayload, OrderBookCombinedEvent};

pub fn get_ws_base_url() -> String {
    env::var("BINANCE_WS_URL")
        .unwrap_or_else(|_| "wss://stream.binance.com:9443/stream?streams=".to_string())
}
