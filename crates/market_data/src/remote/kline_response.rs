use serde::Deserialize;

use common::models::{KlineInsert, Price, Symbol};

use crate::traits::RemoteResponse;

#[derive(Deserialize, Debug)]
pub struct KlineDataCombinedEvent {
    #[serde(rename(deserialize = "k"))]
    pub data: KlineEvent,
}

#[derive(Deserialize, Debug)]
pub struct KlineEvent {
    #[serde(rename(deserialize = "s"))]
    pub symbol: String,
    #[serde(rename(deserialize = "t"))]
    pub start_time: i64,
    #[serde(rename(deserialize = "T"))]
    pub close_time: i64,
    #[serde(rename(deserialize = "o"))]
    pub open_price: String,
    #[serde(rename(deserialize = "c"))]
    pub close_price: String,
    #[serde(rename(deserialize = "h"))]
    pub high_price: String,
    #[serde(rename(deserialize = "l"))]
    pub low_price: String,
    #[serde(rename(deserialize = "v"))]
    pub volume: String,
    #[serde(rename(deserialize = "n"))]
    pub no_of_trades: u64,
    #[serde(rename(deserialize = "x"))]
    pub is_closed: bool,
    #[serde(rename(deserialize = "V"))]
    pub taker_buy_vol: String,
}

impl RemoteResponse<(KlineInsert, bool)> for KlineDataCombinedEvent {
    fn to_insertable(&self) -> Result<(KlineInsert, bool), serde_json::Error> {
        Ok((
            KlineInsert {
                symbol: Symbol::from(self.data.symbol.as_str()),
                start_time: self.data.start_time,
                close_time: self.data.close_time,
                open_price: Price::from(self.data.open_price.as_str()),
                close_price: Price::from(self.data.close_price.as_str()),
                high_price: Price::from(self.data.high_price.as_str()),
                low_price: Price::from(self.data.low_price.as_str()),
                volume: self.data.volume.parse::<f64>().unwrap_or(0_f64),
                no_of_trades: self.data.no_of_trades as i32,
                taker_buy_vol: self.data.taker_buy_vol.parse::<f64>().unwrap_or(0_f64),
            },
            self.data.is_closed,
        ))
    }
}
