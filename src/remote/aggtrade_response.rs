use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::models::AggTradeInsert;

#[derive(Deserialize, Debug)]
pub struct AggTradeCombinedEvent {
    data: AggTradeEvent,
}

#[derive(Deserialize, Debug)]
pub struct AggTradeEvent {
    #[serde(rename(deserialize = "s"))]
    pub symbol: String,
    #[serde(rename(deserialize = "a"))]
    pub agg_trade_id: i64,
    #[serde(rename(deserialize = "p"))]
    pub price: String,
    #[serde(rename(deserialize = "q"))]
    pub quantity: String,
    #[serde(rename(deserialize = "f"))]
    pub first_trade_id: i64,
    #[serde(rename(deserialize = "l"))]
    pub last_trade_id: i64,
    #[serde(rename(deserialize = "m"))]
    pub is_buyer_maker: bool,
}

impl AggTradeCombinedEvent {
    pub fn to_insertable(&self) -> Result<AggTradeInsert, serde_json::Error> {
        let now = SystemTime::now();
        let timestamp_float = now
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs_f64();

        Ok(AggTradeInsert {
            time: timestamp_float,
            symbol: self.data.symbol.clone(),
            agg_trade_id: self.data.agg_trade_id,
            price: self.data.price.parse::<f64>().unwrap_or(0_f64),
            quantity: self.data.quantity.parse::<f64>().unwrap_or(0_f64),
            first_trade_id: self.data.first_trade_id,
            last_trade_id: self.data.last_trade_id,
            is_buyer_maker: self.data.is_buyer_maker,
        })
    }
}
