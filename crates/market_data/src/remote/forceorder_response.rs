use common::models::force_order::ForceOrderInsert;
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
    #[serde(rename(deserialize = "p"))]
    pub price: String,
    #[serde(rename(deserialize = "q"))]
    pub quantity: String,
}

impl RemoteResponse<ForceOrderInsert> for ForceOrderCombinedEvent {
    fn to_insertable(&self) -> Result<ForceOrderInsert, serde_json::Error> {
        Ok(ForceOrderInsert {
            time: self.get_time_f64(),
            symbol: self.data.symbol.clone(),
            side: self.data.side.clone(),
            price: self.data.price.parse::<f64>().unwrap_or(0_f64),
            quantity: self.data.quantity.parse::<f64>().unwrap_or(0_f64),
        })
    }
}
