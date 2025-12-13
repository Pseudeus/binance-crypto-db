use diesel::prelude::*;

use crate::schema::order_books;

#[derive(Insertable, Clone)]
#[diesel(table_name = order_books)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct NewOrderBook {
    pub time: f64,
    pub symbol: String,
    pub bids: Vec<u8>,
    pub asks: Vec<u8>,
}
