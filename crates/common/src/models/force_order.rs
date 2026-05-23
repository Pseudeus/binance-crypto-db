use crate::models::{Price, Quantity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ForceOrder {
    pub id: i32,
    pub time: f64,
    pub symbol: String,
    pub side: String,
    pub price: Price,
    pub quantity: Quantity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForceOrderInsert {
    pub time: f64,
    pub symbol: String,
    pub side: String,
    pub price: Price,
    pub quantity: Quantity,
}
