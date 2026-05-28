use serde::Deserialize;

use common::models::{OrderBookInsert, Symbol};

use crate::traits::RemoteResponse;

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

impl RemoteResponse<OrderBookInsert> for OrderBookCombinedEvent {
    fn to_insertable(&self) -> Result<OrderBookInsert, serde_json::Error> {
        let symbol_upper = &self
            .stream
            .split('@')
            .next()
            .unwrap_or("UNK")
            .to_uppercase();

        Ok(OrderBookInsert {
            receive_time: self.get_time_i64(),
            symbol: Symbol::from(symbol_upper.as_str()),
            bids: Self::pack_level(&self.data.bids),
            asks: Self::pack_level(&self.data.asks),
        })
    }
}

impl OrderBookCombinedEvent {
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
