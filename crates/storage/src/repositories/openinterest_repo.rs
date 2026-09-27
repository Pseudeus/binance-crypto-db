use crate::repositories::Repository;
use common::models::OpenInterestInsert;
use rusqlite::{params, Connection, Result, Transaction};

pub struct OpenInterestRepository;

impl Repository for OpenInterestRepository {
    type Input = OpenInterestInsert;

    fn insert(conn: &Connection, interest: &Self::Input) -> Result<()> {
        conn.execute(
            r#"
                INSERT INTO open_interest (
                    time, symbol_id, oi_value
                ) VALUES (?, ?, ?)
            "#,
            params![interest.time, interest.symbol, interest.oi_value],
        )?;
        Ok(())
    }

    fn insert_batch(tx: &Transaction, interests: &[Self::Input]) -> Result<()> {
        if interests.is_empty() {
            return Ok(());
        }
        let mut stmt = tx.prepare_cached(
            r#"
                INSERT INTO open_interest (
                    time, symbol_id, oi_value
                ) VALUES (?, ?, ?)
            "#,
        )?;
        for interest in interests {
            stmt.execute(params![interest.time, interest.symbol, interest.oi_value])?;
        }
        Ok(())
    }
}
