use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
#[allow(dead_code)]
pub struct AggTrade {
    pub id: i32,
    pub time: f64,
    pub symbol: String,
    pub price: f64,
    pub quantity: f64,
    pub is_buyer_maker: bool,
}

#[derive(Debug, Clone)]
pub struct AggTradeInsert {
    pub time: f64,
    pub symbol: String,
    pub price: f64,
    pub quantity: f64,
    pub is_buyer_maker: bool,
}
