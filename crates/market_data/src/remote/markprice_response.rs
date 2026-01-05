use common::models::markprice::MarkPriceInsert;
use serde::Deserialize;

use crate::traits::RemoteResponse;

#[derive(Deserialize, Debug)]
pub struct MarkPriceEvent {
    #[serde(rename(deserialize = "s"))]
    pub symbol: String,
    #[serde(rename(deserialize = "p"))]
    pub mark_price: String,
    #[serde(rename(deserialize = "i"))]
    pub index_price: String,
    #[serde(rename(deserialize = "r"))]
    pub funding_rate: String,
}

impl RemoteResponse<MarkPriceInsert> for MarkPriceEvent {
    fn to_insertable(&self) -> Result<MarkPriceInsert, serde_json::Error> {
        Ok(MarkPriceInsert {
            time: self.get_time_f64(),
            symbol: self.symbol.clone(),
            mark_price: self.mark_price.parse::<f64>().unwrap_or(0_f64),
            index_price: self.index_price.parse::<f64>().unwrap_or(0_f64),
            funding_rate: self.funding_rate.parse::<f64>().unwrap_or(0_f64),
        })
    }
}
