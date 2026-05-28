use serde::{Deserialize, Serialize};

use crate::models::Symbol;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OrderBook {
    pub id: i32,
    pub receive_time: i64,
    pub symbol: String,
    pub bids: Vec<u8>,
    pub asks: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderBookInsert {
    pub receive_time: i64,
    pub symbol: Symbol,
    pub bids: Vec<u8>,
    pub asks: Vec<u8>,
}
