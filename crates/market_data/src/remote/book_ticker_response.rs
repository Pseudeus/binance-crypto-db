use common::models::{Price, Quantity, Symbol, book_ticker::BookTickerInsert};
use serde::Deserialize;

use crate::traits::RemoteResponse;

#[derive(Deserialize, Debug)]
pub struct BookTickerEvent {
    #[serde(rename(deserialize = "s"))]
    pub symbol: String,
    #[serde(rename(deserialize = "b"))]
    pub best_bid_price: String,
    #[serde(rename(deserialize = "B"))]
    pub best_bid_qty: String,
    #[serde(rename(deserialize = "a"))]
    pub best_ask_price: String,
    #[serde(rename(deserialize = "A"))]
    pub best_ask_qty: String,
}

impl RemoteResponse<BookTickerInsert> for BookTickerEvent {
    fn to_insertable(&self) -> Result<BookTickerInsert, serde_json::Error> {
        Ok(BookTickerInsert {
            receive_time: self.get_time_i64(),
            symbol: Symbol::from(self.symbol.as_str()),
            best_bid_price: Price::from(self.best_bid_price.as_str()),
            best_bid_qty: Quantity::from(self.best_bid_qty.as_str()),
            best_ask_price: Price::from(self.best_ask_price.as_str()),
            best_ask_qty: Quantity::from(self.best_ask_qty.as_str()),
        })
    }
}
