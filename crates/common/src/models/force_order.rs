#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ForceOrder {
    pub id: i32,
    pub time: f64,
    pub symbol: i32,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
}

#[derive(Debug, Clone)]
pub struct ForceOrderInsert {
    pub time: f64,
    pub symbol: String,
    pub side: String,
    pub price: f64,
    pub quantity: f64,
}
