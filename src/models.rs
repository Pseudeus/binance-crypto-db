use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema;

#[derive(Insertable, Clone)]
#[diesel(table_name = schema::order_books)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewOrderBook {
    pub time: f64,
    pub symbol: String,
    pub bids: Vec<u8>,
    pub asks: Vec<u8>,
}

#[derive(Queryable, Selectable, Debug, Clone, Serialize, Deserialize)]
#[diesel(table_name = schema::agg_trades)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AggTrade {
    pub id: i32,
    pub time: f64,
    pub symbol: String,
    pub agg_trade_id: i64,
    pub price: f64,
    pub quantity: f64,
    pub first_trade_id: i64,
    pub last_trade_id: i64,
    pub is_buyer_maker: bool,
}

#[derive(Insertable, Debug, Clone)]
#[diesel(table_name = schema::agg_trades)]
pub struct NewAggTrade {
    pub time: f64,
    pub symbol: String,
    pub agg_trade_id: i64,
    pub price: f64,
    pub quantity: f64,
    pub first_trade_id: i64,
    pub last_trade_id: i64,
    pub is_buyer_maker: bool,
}
