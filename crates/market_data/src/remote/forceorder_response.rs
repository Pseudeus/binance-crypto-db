use common::models::{ForceOrderInsert, Price, Quantity, Symbol};
use serde::Deserialize;

use crate::traits::RemoteResponse;

#[derive(Deserialize, Debug)]
pub struct ForceOrderCombinedEvent {
    #[serde(rename(deserialize = "o"))]
    pub data: ForceOrderEvent,
}

#[derive(Deserialize, Debug)]
pub struct ForceOrderEvent {
    #[serde(rename(deserialize = "s"))]
    pub symbol: String,
    #[serde(rename(deserialize = "S"))]
    pub side: String,
    #[serde(rename(deserialize = "ap"))]
    pub avg_price: String,
    #[serde(rename(deserialize = "q"))]
    pub quantity: String,
    #[serde(rename(deserialize = "T"))]
    pub exchange_time: i64,
}

impl RemoteResponse<ForceOrderInsert> for ForceOrderCombinedEvent {
    fn to_insertable(&self) -> Result<ForceOrderInsert, serde_json::Error> {
        Ok(ForceOrderInsert {
            exchange_time: self.data.exchange_time,
            receive_time: self.get_time_i64(),
            symbol: Symbol::from(self.data.symbol.as_str()),
            side: self.data.side.clone(),
            avg_price: Price::from(self.data.avg_price.as_str()),
            quantity: Quantity::from(self.data.quantity.as_str()),
        })
    }
}
