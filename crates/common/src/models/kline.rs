use serde::{Deserialize, Serialize};

use crate::models::{Price, Symbol};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Kline {
    pub id: i32,
    pub symbol: Symbol,
    pub start_time: i64,
    pub close_time: i64,
    pub open_price: Price,
    pub close_price: Price,
    pub high_price: Price,
    pub low_price: Price,
    pub volume: f64,
    pub no_of_trades: i32,
    pub taker_buy_vol: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KlineInsert {
    pub symbol: Symbol,
    pub start_time: i64,
    pub close_time: i64,
    pub open_price: Price,
    pub close_price: Price,
    pub high_price: Price,
    pub low_price: Price,
    pub volume: f64,
    pub no_of_trades: i32,
    pub taker_buy_vol: f64,
}
