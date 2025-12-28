use serde::Deserialize;

use crate::models::KlineInsert;

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
    pub start_time: u64,
    #[serde(rename(deserialize = "T"))]
    pub close_time: u64,
    #[serde(rename(deserialize = "i"))]
    pub interval: String,
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

impl KlineDataCombinedEvent {
    pub fn to_insertable(&self) -> Result<(KlineInsert, bool), serde_json::Error> {
        Ok((
            KlineInsert {
                symbol: self.data.symbol.clone(),
                start_time: self.data.start_time as i32,
                close_time: self.data.close_time as i32,
                interval: self.data.interval.clone(),
                open_price: self.data.open_price.parse::<f32>().unwrap_or(0_f32),
                close_price: self.data.close_price.parse::<f32>().unwrap_or(0_f32),
                high_price: self.data.high_price.parse::<f32>().unwrap_or(0_f32),
                low_price: self.data.low_price.parse::<f32>().unwrap_or(0_f32),
                volume: self.data.volume.parse::<f64>().unwrap_or(0_f64),
                no_of_trades: self.data.no_of_trades as i32,
                taker_buy_vol: self.data.taker_buy_vol.parse::<f32>().unwrap_or(0_f32),
            },
            self.data.is_closed,
        ))
    }
}
