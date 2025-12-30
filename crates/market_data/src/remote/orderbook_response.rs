use serde::Deserialize;
use std::time::{SystemTime, UNIX_EPOCH};

use common::models::OrderBookInsert;

#[derive(Deserialize, Debug)]
pub struct OrderBookCombinedEvent {
    pub stream: String,
    pub data: DepthPayload,
}

#[derive(Deserialize, Debug)]
pub struct DepthPayload {
    pub bids: Vec<[String; 2]>,
    pub asks: Vec<[String; 2]>,
}

impl OrderBookCombinedEvent {
    pub fn to_insertable(&self) -> Result<OrderBookInsert, serde_json::Error> {
        let symbol_upper = &self
            .stream
            .split('@')
            .next()
            .unwrap_or("UNK")
            .to_uppercase();

        let now = SystemTime::now();
        let timestamp_float = now
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs_f64();

        Ok(OrderBookInsert {
            time: timestamp_float,
            symbol: symbol_upper.to_string(),
            bids: Self::pack_level(&self.data.bids),
            asks: Self::pack_level(&self.data.asks),
        })
    }

    fn pack_level(items: &Vec<[String; 2]>) -> Vec<u8> {
        let capacity = items.len() * 8;
        let mut writer = Vec::with_capacity(capacity);

        for item in items {
            let price = item[0].parse::<f32>().unwrap_or(0_f32);
            let quantity = item[1].parse::<f32>().unwrap_or(0_f32);

            writer.extend_from_slice(&price.to_le_bytes());
            writer.extend_from_slice(&quantity.to_le_bytes());
        }
        writer
    }
}
