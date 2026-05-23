use crate::{db::RotatingPool, repositories::Repository, storage_write_buffer::StorageWriteBuffer};
use common::models::AggTradeInsert;
use sqlx::QueryBuilder;
use std::sync::Arc;

const BATCH_SIZE: usize = (i16::MAX / 5) as usize;

pub type AggTradeWriterBuffer = StorageWriteBuffer<AggTradeInsert, AggTradeRepository>;

pub struct AggTradeRepository {
    pool: Arc<RotatingPool>,
}

impl AggTradeRepository {
    pub fn new(pool: Arc<RotatingPool>) -> Self {
        Self { pool: pool.clone() }
    }
}

impl Repository for AggTradeRepository {
    type Input = AggTradeInsert;

    async fn insert(&self, trade: &Self::Input) -> Result<(), sqlx::Error> {
        let (pool, _) = self.pool.get_pool().await?;
        sqlx::query(
            r#"
                INSERT INTO agg_trades (
                    time, symbol_id, price, quantity, is_buyer_maker
                ) VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(trade.time)
        .bind(&trade.symbol.0)
        .bind(trade.price.0)
        .bind(trade.quantity.0)
        .bind(trade.is_buyer_maker)
        .execute(&pool)
        .await?;
        Ok(())
    }

    async fn insert_batch(&self, trades: &[Self::Input]) -> Result<(), sqlx::Error> {
        if trades.is_empty() {
            return Ok(());
        }
        let (pool, _) = self.pool.get_pool().await?;
        let mut tx = pool.begin().await?;

        for chunks in trades.chunks(BATCH_SIZE) {
            let mut query_builder = QueryBuilder::new(
                r#"
                    INSERT INTO agg_trades (
                        time, symbol_id, price, quantity, is_buyer_maker
                    )
                "#,
            );
            query_builder.push_values(chunks, |mut b, entry| {
                b.push_bind(entry.time)
                    .push_bind(&entry.symbol.0)
                    .push_bind(entry.price.0)
                    .push_bind(entry.quantity.0)
                    .push_bind(entry.is_buyer_maker);
            });
            let query = query_builder.build();
            query.execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
