use crate::models::Price;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MarkPrice {
    pub id: i32,
    pub time: f64,
    pub symbol: String,
    pub mark_price: Price,
    pub index_price: Price,
    pub funding_rage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarkPriceInsert {
    pub time: f64,
    pub symbol: String,
    pub mark_price: Price,
    pub index_price: Price,
    pub funding_rate: f64,
}
