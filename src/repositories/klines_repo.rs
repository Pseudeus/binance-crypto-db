use sqlx::SqlitePool;

use crate::models::KlineInsert;

pub struct KlinesRepository;

impl KlinesRepository {
    pub async fn insert_batch(
        pool: &SqlitePool,
        klines: &[KlineInsert],
    ) -> Result<(), sqlx::Error> {
        if klines.is_empty() {
            return Ok(());
        }

        let mut tx = pool.begin().await?;

        for kline in klines {
            sqlx::query(
                r#"
                    INSERT INTO klines (
                        symbol, start_time, close_time, interval, open_price, close_price,
                        high_price, low_price, volume, no_of_trades, taker_buy_vol
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&kline.symbol)
            .bind(kline.start_time)
            .bind(kline.close_time)
            .bind(&kline.interval)
            .bind(kline.open_price)
            .bind(kline.close_price)
            .bind(kline.high_price)
            .bind(kline.low_price)
            .bind(kline.volume)
            .bind(kline.no_of_trades)
            .bind(kline.taker_buy_vol)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
