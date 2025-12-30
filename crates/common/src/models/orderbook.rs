#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OrderBook {
    pub id: i32,
    pub time: f64,
    pub symbol: String,
    pub bids: Vec<u8>,
    pub asks: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct OrderBookInsert {
    pub time: f64,
    pub symbol: String,
    pub bids: Vec<u8>,
    pub asks: Vec<u8>,
}
