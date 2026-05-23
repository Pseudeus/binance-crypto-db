use crate::models::{Price, Quantity, Symbol};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AggTrade {
    pub id: i32,
    pub time: f64,
    pub symbol_id: Symbol,
    pub price: Price,
    pub quantity: Quantity,
    pub is_buyer_maker: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggTradeInsert {
    pub time: f64,
    pub symbol: Symbol,
    pub price: Price,
    pub quantity: Quantity,
    pub is_buyer_maker: bool,
}
