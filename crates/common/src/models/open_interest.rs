use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct OpenInterest {
    pub id: i32,
    pub time: f64,
    pub symbol_id: String,
    pub oi_value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenInterestInsert {
    pub time: f64,
    pub symbol: String,
    pub oi_value: f64,
}
