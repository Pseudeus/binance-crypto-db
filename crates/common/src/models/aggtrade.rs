use crate::models::{Price, Quantity, Symbol};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AggTrade {
    pub id: i32,
    pub receive_time: i64,
    pub exchange_time: i64,
    pub symbol_id: Symbol,
    pub price: Price,
    pub quantity: Quantity,
    pub is_buyer_maker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggTradeInsert {
    pub receive_time: i64,
    pub exchange_time: i64,
    pub symbol: Symbol,
    pub price: Price,
    pub quantity: Quantity,
    pub is_buyer_maker: bool,
}
