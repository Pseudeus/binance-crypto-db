use crate::models::{Price, Quantity, Symbol};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ForceOrder {
    pub id: i32,
    pub receive_time: i64,
    pub exchange_time: i64,
    pub symbol: Symbol,
    pub side: String,
    pub avg_price: Price,
    pub quantity: Quantity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForceOrderInsert {
    pub receive_time: i64,
    pub exchange_time: i64,
    pub symbol: Symbol,
    pub side: String,
    pub avg_price: Price,
    pub quantity: Quantity,
}
