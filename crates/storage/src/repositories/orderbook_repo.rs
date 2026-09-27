use crate::repositories::Repository;
use common::models::OrderBookInsert;
use rusqlite::{params, Connection, Result, Transaction};

pub struct OrderBookRepository;

impl Repository for OrderBookRepository {
    type Input = OrderBookInsert;

    fn insert(conn: &Connection, book: &Self::Input) -> Result<()> {
        conn.execute(
            r#"
                INSERT INTO order_books (time, symbol_id, bids, asks)
                VALUES (?, ?, ?, ?)
            "#,
            params![book.time, book.symbol, book.bids, book.asks],
        )?;
        Ok(())
    }

    fn insert_batch(tx: &Transaction, books: &[Self::Input]) -> Result<()> {
        if books.is_empty() {
            return Ok(());
        }
        let mut stmt = tx.prepare_cached(
            r#"
                INSERT INTO order_books (time, symbol_id, bids, asks)
                VALUES (?, ?, ?, ?)
            "#,
        )?;
        for book in books {
            stmt.execute(params![book.time, book.symbol, book.bids, book.asks])?;
        }
        Ok(())
    }
}
