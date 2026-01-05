#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct MarkPrice {
    pub id: i32,
    pub time: f64,
    pub symbol: i32,
    pub mark_price: f64,
    pub index_price: f64,
    pub funding_rage: f64,
}

#[derive(Debug, Clone)]
pub struct MarkPriceInsert {
    pub time: f64,
    pub symbol: String,
    pub mark_price: f64,
    pub index_price: f64,
    pub funding_rate: f64,
}
