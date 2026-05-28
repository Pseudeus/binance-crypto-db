use common::models::ForceOrderInsert;
use sqlx::QueryBuilder;
use std::sync::Arc;

use crate::{
    batch_size, db::RotatingPool, repositories::Repository,
    storage_write_buffer::StorageWriteBuffer,
};

pub type ForceOrderWriterBuffer = StorageWriteBuffer<ForceOrderInsert, ForceOrderRepository>;

pub struct ForceOrderRepository {
    pool: Arc<RotatingPool>,
}

impl ForceOrderRepository {
    pub fn new(pool: Arc<RotatingPool>) -> Self {
        Self { pool: pool.clone() }
    }
}

impl Repository for ForceOrderRepository {
    type Input = ForceOrderInsert;

    async fn insert(&self, liquidation: &Self::Input) -> Result<(), sqlx::Error> {
        let (pool, _) = self.pool.get_pool().await?;
        sqlx::query(
            r#"
                INSERT INTO fut_liquidations (
                    receive_time, exchange_time, symbol_id,
                    side, price, quantity
                ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(liquidation.receive_time)
        .bind(liquidation.exchange_time)
        .bind(&liquidation.symbol.0)
        .bind(&liquidation.side)
        .bind(liquidation.avg_price.0)
        .bind(liquidation.quantity.0)
        .execute(&pool)
        .await?;
        Ok(())
    }

    async fn insert_batch(&self, liquidations: &[Self::Input]) -> Result<(), sqlx::Error> {
        if liquidations.is_empty() {
            return Ok(());
        }
        let (pool, _) = self.pool.get_pool().await?;
        let mut tx = pool.begin().await?;

        batch_size!(6);
        for chunk in liquidations.chunks(BATCH_SIZE) {
            let mut query_builder = QueryBuilder::new(
                r#"
                    INSERT INTO fut_liquidations (
                        receive_time, exchange_time, symbol_id,
                        side, price, quantity
                    )
                "#,
            );
            query_builder.push_values(chunk, |mut b, liquidation| {
                b.push_bind(liquidation.receive_time)
                    .push_bind(liquidation.exchange_time)
                    .push_bind(&liquidation.symbol.0)
                    .push_bind(&liquidation.side)
                    .push_bind(liquidation.avg_price.0)
                    .push_bind(liquidation.quantity.0);
            });
            let query = query_builder.build();
            query.execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
