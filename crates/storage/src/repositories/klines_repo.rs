use crate::{
    batch_size, db::RotatingPool, repositories::Repository,
    storage_write_buffer::StorageWriteBuffer,
};
use common::models::KlineInsert;
use sqlx::QueryBuilder;
use std::sync::Arc;

pub type KlineWriterBuffer = StorageWriteBuffer<KlineInsert, KlineRepository>;

pub struct KlineRepository {
    pool: Arc<RotatingPool>,
}

impl KlineRepository {
    pub fn new(pool: Arc<RotatingPool>) -> Self {
        Self { pool: pool.clone() }
    }
}

impl Repository for KlineRepository {
    type Input = KlineInsert;

    async fn insert(&self, kline: &Self::Input) -> Result<(), sqlx::Error> {
        let (pool, _) = self.pool.get_pool().await?;
        sqlx::query(
            r#"
                INSERT INTO spot_klines_1m (
                    symbol_id, start_time, close_time, open_price, close_price,
                    high_price, low_price, volume, no_of_trades, taker_buy_vol
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&kline.symbol.0)
        .bind(kline.start_time)
        .bind(kline.close_time)
        .bind(kline.open_price.0)
        .bind(kline.close_price.0)
        .bind(kline.high_price.0)
        .bind(kline.low_price.0)
        .bind(kline.volume)
        .bind(kline.no_of_trades)
        .bind(kline.taker_buy_vol)
        .execute(&pool)
        .await?;
        Ok(())
    }

    async fn insert_batch(&self, klines: &[Self::Input]) -> Result<(), sqlx::Error> {
        if klines.is_empty() {
            return Ok(());
        }

        let (pool, _) = self.pool.get_pool().await?;
        let mut tx = pool.begin().await?;

        batch_size!(11);
        for chunk in klines.chunks(BATCH_SIZE) {
            let mut query_builder = QueryBuilder::new(
                r#"
                    INSERT INTO spot_klines_1m (
                        symbol_id, start_time, close_time, open_price, close_price,
                        high_price, low_price, volume, no_of_trades, taker_buy_vol
                    )
                "#,
            );
            query_builder.push_values(chunk, |mut b, kline| {
                b.push_bind(&kline.symbol.0)
                    .push_bind(kline.start_time)
                    .push_bind(kline.close_time)
                    .push_bind(kline.open_price.0)
                    .push_bind(kline.close_price.0)
                    .push_bind(kline.high_price.0)
                    .push_bind(kline.low_price.0)
                    .push_bind(kline.volume)
                    .push_bind(kline.no_of_trades)
                    .push_bind(kline.taker_buy_vol);
            });
            let query = query_builder.build();
            query.execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
