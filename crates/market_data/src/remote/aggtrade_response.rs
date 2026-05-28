use common::models::{AggTradeInsert, Price, Quantity, Symbol};
use serde::Deserialize;

use crate::traits::RemoteResponse;

#[derive(Deserialize, Debug)]
pub struct AggTradeCombinedEvent {
    pub data: AggTradeEvent,
}

#[derive(Deserialize, Debug)]
pub struct AggTradeEvent {
    #[serde(rename(deserialize = "s"))]
    pub symbol: String,
    #[serde(rename(deserialize = "p"))]
    pub price: String,
    #[serde(rename(deserialize = "q"))]
    pub quantity: String,
    #[serde(rename(deserialize = "m"))]
    pub is_buyer_maker: bool,
    #[serde(rename(deserialize = "T"))]
    pub exchange_time: i64,
}

impl RemoteResponse<AggTradeInsert> for AggTradeCombinedEvent {
    fn to_insertable(&self) -> Result<AggTradeInsert, serde_json::Error> {
        Ok(AggTradeInsert {
            receive_time: self.get_time_i64(),
            exchange_time: self.data.exchange_time,
            symbol: Symbol::from(self.data.symbol.as_str()),
            price: Price::from(self.data.price.as_str()),
            quantity: Quantity::from(self.data.quantity.as_str()),
            is_buyer_maker: self.data.is_buyer_maker,
        })
    }
}
