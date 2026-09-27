use crate::repositories::Repository;
use common::models::KlineInsert;
use rusqlite::{params, Connection, Result, Transaction};

pub struct KlineRepository;

impl Repository for KlineRepository {
    type Input = KlineInsert;

    fn insert(conn: &Connection, kline: &Self::Input) -> Result<()> {
        conn.execute(
            r#"
                INSERT INTO klines_1s (
                    symbol_id, start_time, close_time, open_price, close_price,
                    high_price, low_price, volume, no_of_trades, taker_buy_vol
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                kline.symbol,
                kline.start_time,
                kline.close_time,
                kline.open_price,
                kline.close_price,
                kline.high_price,
                kline.low_price,
                kline.volume,
                kline.no_of_trades,
                kline.taker_buy_vol,
            ],
        )?;
        Ok(())
    }

    fn insert_batch(tx: &Transaction, klines: &[Self::Input]) -> Result<()> {
        if klines.is_empty() {
            return Ok(());
        }
        let mut stmt = tx.prepare_cached(
            r#"
                INSERT INTO klines_1s (
                    symbol_id, start_time, close_time, open_price, close_price,
                    high_price, low_price, volume, no_of_trades, taker_buy_vol
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )?;
        for kline in klines {
            stmt.execute(params![
                kline.symbol,
                kline.start_time,
                kline.close_time,
                kline.open_price,
                kline.close_price,
                kline.high_price,
                kline.low_price,
                kline.volume,
                kline.no_of_trades,
                kline.taker_buy_vol,
            ])?;
        }
        Ok(())
    }
}
