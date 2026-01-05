use common::models::KlineInsert;

use crate::data_manager::DataManager;

pub struct KlinesRepository;

impl KlinesRepository {
    pub async fn insert_batch(
        data_manager: &DataManager,
        klines: &[KlineInsert],
    ) -> Result<(), sqlx::Error> {
        if klines.is_empty() {
            return Ok(());
        }

        let (pool, _) = data_manager.pool_rotator.get_pool().await?;
        let mut tx = pool.begin().await?;

        for kline in klines {
            let symbol_id = data_manager.get_symbol_id(&kline.symbol).await?;
            sqlx::query(
                r#"
                    INSERT INTO klines (
                        symbol_id, start_time, close_time, interval, open_price, close_price,
                        high_price, low_price, volume, no_of_trades, taker_buy_vol
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(symbol_id)
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
