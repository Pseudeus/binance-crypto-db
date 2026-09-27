use crate::repositories::Repository;
use common::models::AggTradeInsert;
use rusqlite::{params, Connection, Result, Transaction};

pub struct AggTradeRepository;

impl Repository for AggTradeRepository {
    type Input = AggTradeInsert;

    fn insert(conn: &Connection, trade: &Self::Input) -> Result<()> {
        conn.execute(
            r#"
                INSERT INTO agg_trades (
                    time, symbol_id, price, quantity, is_buyer_maker
                ) VALUES (?, ?, ?, ?, ?)
            "#,
            params![
                trade.time,
                trade.symbol.0,
                trade.price.0,
                trade.quantity.0,
                trade.is_buyer_maker
            ],
        )?;
        Ok(())
    }

    fn insert_batch(tx: &Transaction, trades: &[Self::Input]) -> Result<()> {
        if trades.is_empty() {
            return Ok(());
        }
        let mut stmt = tx.prepare_cached(
            r#"
                INSERT INTO agg_trades (
                    time, symbol_id, price, quantity, is_buyer_maker
                ) VALUES (?, ?, ?, ?, ?)
            "#,
        )?;
        for trade in trades {
            stmt.execute(params![
                trade.time,
                trade.symbol.0,
                trade.price.0,
                trade.quantity.0,
                trade.is_buyer_maker
            ])?;
        }
        Ok(())
    }
}
