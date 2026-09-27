use crate::repositories::Repository;
use common::models::MarkPriceInsert;
use rusqlite::{params, Connection, Result, Transaction};

pub struct MarkPriceRepository;

impl Repository for MarkPriceRepository {
    type Input = MarkPriceInsert;

    fn insert(conn: &Connection, m_price: &Self::Input) -> Result<()> {
        conn.execute(
            r#"
                INSERT INTO funding_rates (
                    time, symbol_id, mark_price, index_price, rate
                ) VALUES (?, ?, ?, ?, ?)
            "#,
            params![
                m_price.time,
                m_price.symbol,
                m_price.mark_price.0,
                m_price.index_price.0,
                m_price.funding_rate
            ],
        )?;
        Ok(())
    }

    fn insert_batch(tx: &Transaction, m_prices: &[Self::Input]) -> Result<()> {
        if m_prices.is_empty() {
            return Ok(());
        }
        let mut stmt = tx.prepare_cached(
            r#"
                INSERT INTO funding_rates (
                    time, symbol_id, mark_price, index_price, rate
                ) VALUES (?, ?, ?, ?, ?)
            "#,
        )?;
        for m_price in m_prices {
            stmt.execute(params![
                m_price.time,
                m_price.symbol,
                m_price.mark_price.0,
                m_price.index_price.0,
                m_price.funding_rate
            ])?;
        }
        Ok(())
    }
}
