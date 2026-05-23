use serde::Deserialize;

use common::models::{AggTradeInsert, Price, Quantity, Symbol};

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
}

impl RemoteResponse<AggTradeInsert> for AggTradeCombinedEvent {
    fn to_insertable(&self) -> Result<AggTradeInsert, serde_json::Error> {
        Ok(AggTradeInsert {
            time: self.get_time_f64(),
            symbol: Symbol(self.data.symbol.clone()),
            price: Price(self.data.price.parse::<f64>().unwrap_or(0_f64)),
            quantity: Quantity(self.data.quantity.parse::<f64>().unwrap_or(0_f64)),
            is_buyer_maker: self.data.is_buyer_maker,
        })
    }
}
