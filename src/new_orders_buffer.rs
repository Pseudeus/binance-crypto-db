// use crate::models::NewOrderBook;
// use crate::schema::order_books;
// use diesel::prelude::*;
// use diesel::sqlite::SqliteConnection;
// use tracing::{error, info};

// pub struct OrdersBookBuffer {
//     data: Vec<NewOrderBook>,
// }

// impl OrdersBookBuffer {
//     pub fn new(capacity: usize) -> Self {
//         Self {
//             data: Vec::with_capacity(capacity),
//         }
//     }
//     pub fn push(&mut self, new_order: NewOrderBook) {
//         self.data.push(new_order);
//     }

//     pub fn len(&self) -> usize {
//         self.data.len()
//     }

//     pub fn flush(&mut self, conn: &mut SqliteConnection) {
//         if self.data.is_empty() {
//             return;
//         }

//         let result = conn.transaction::<_, diesel::result::Error, _>(|c| {
//             diesel::insert_into(order_books::table)
//                 .values(&*self.data)
//                 .execute(c)
//         });

//         match result {
//             Ok(_) => {
//                 info!("Flushed {} records to disk.", self.data.len());
//                 self.data.clear();
//             }
//             Err(e) => error!("Error flushing buffer: {}", e),
//         }
//     }
// }
