#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Kline {
    pub id: i32,
    pub symbol: String,
    pub start_time: i32,
    pub close_time: i32,
    pub interval: String,
    pub open_price: f32,
    pub close_price: f32,
    pub high_price: f32,
    pub low_price: f32,
    pub volume: f64,
    pub no_of_trades: i32,
    pub taker_buy_vol: f32,
}

#[derive(Debug, Clone)]
pub struct KlineInsert {
    pub symbol: String,
    pub start_time: i32,
    pub close_time: i32,
    pub interval: String,
    pub open_price: f32,
    pub close_price: f32,
    pub high_price: f32,
    pub low_price: f32,
    pub volume: f64,
    pub no_of_trades: i32,
    pub taker_buy_vol: f32,
}
