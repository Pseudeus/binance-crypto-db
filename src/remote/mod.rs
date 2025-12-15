pub mod aggtrade_response;
pub mod orderbook_response;

pub use aggtrade_response::{AggTradeCombinedEvent, AggTradeEvent};
pub use orderbook_response::{DepthPayload, OrderBookCombinedEvent};

pub const BASE_URL: &str = "wss://stream.binance.com:9443/stream?streams=";
