use crate::models::{Price, Quantity, Symbol};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BookTicker {
    pub id: i32,
    pub receive_time: i64,
    pub symbol: Symbol,
    pub best_bid_price: Price,
    pub best_bid_qty: Quantity,
    pub best_ask_price: Price,
    pub best_ask_qty: Quantity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookTickerInsert {
    pub receive_time: i64,
    pub symbol: Symbol,
    pub best_bid_price: Price,
    pub best_bid_qty: Quantity,
    pub best_ask_price: Price,
    pub best_ask_qty: Quantity,
}
