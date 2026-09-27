use crate::repositories::Repository;
use common::models::ForceOrderInsert;
use rusqlite::{params, Connection, Result, Transaction};

pub struct ForceOrderRepository;

impl Repository for ForceOrderRepository {
    type Input = ForceOrderInsert;

    fn insert(conn: &Connection, order: &Self::Input) -> Result<()> {
        conn.execute(
            r#"
                INSERT INTO liquidations (
                    time, symbol_id, side, price, quantity
                ) VALUES (?, ?, ?, ?, ?)
            "#,
            params![
                order.time,
                order.symbol,
                order.side,
                order.price.0,
                order.quantity.0
            ],
        )?;
        Ok(())
    }

    fn insert_batch(tx: &Transaction, orders: &[Self::Input]) -> Result<()> {
        if orders.is_empty() {
            return Ok(());
        }
        let mut stmt = tx.prepare_cached(
            r#"
                INSERT INTO liquidations (
                    time, symbol_id, side, price, quantity
                ) VALUES (?, ?, ?, ?, ?)
            "#,
        )?;
        for order in orders {
            stmt.execute(params![
                order.time,
                order.symbol,
                order.side,
                order.price.0,
                order.quantity.0
            ])?;
        }
        Ok(())
    }
}
