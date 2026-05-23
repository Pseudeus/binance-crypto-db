use common::models::ForceOrderInsert;
use sqlx::QueryBuilder;
use std::sync::Arc;

use crate::{db::RotatingPool, repositories::Repository, storage_write_buffer::StorageWriteBuffer};

const BATCH_SIZE: usize = (i16::MAX / 5) as usize;

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

    async fn insert(&self, order: &Self::Input) -> Result<(), sqlx::Error> {
        let (pool, _) = self.pool.get_pool().await?;
        sqlx::query(
            r#"
                INSERT INTO liquidations (
                    time, symbol_id, side, price, quantity
                ) VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(order.time)
        .bind(&order.symbol)
        .bind(order.side.clone())
        .bind(order.price.0)
        .bind(order.quantity.0)
        .execute(&pool)
        .await?;
        Ok(())
    }

    async fn insert_batch(&self, orders: &[Self::Input]) -> Result<(), sqlx::Error> {
        if orders.is_empty() {
            return Ok(());
        }
        let (pool, _) = self.pool.get_pool().await?;
        let mut tx = pool.begin().await?;

        for chunk in orders.chunks(BATCH_SIZE) {
            let mut query_builder = QueryBuilder::new(
                r#"
                    INSERT INTO liquidations (
                        time, symbol_id, side, price, quantity
                    )
                "#,
            );
            query_builder.push_values(chunk, |mut b, order| {
                b.push_bind(order.time)
                    .push_bind(&order.symbol)
                    .push_bind(&order.side)
                    .push_bind(order.price.0)
                    .push_bind(order.quantity.0);
            });
            let query = query_builder.build();
            query.execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
