use crate::{db::RotatingPool, repositories::Repository, storage_write_buffer::StorageWriteBuffer};
use common::models::MarkPriceInsert;
use sqlx::QueryBuilder;
use std::sync::Arc;

const BATCH_SIZE: usize = (i16::MAX / 5) as usize;

pub type MarkPriceWriterBuffer = StorageWriteBuffer<MarkPriceInsert, MarkPriceRepository>;

pub struct MarkPriceRepository {
    pool: Arc<RotatingPool>,
}

impl MarkPriceRepository {
    pub fn new(pool: Arc<RotatingPool>) -> Self {
        Self { pool: pool.clone() }
    }
}

impl Repository for MarkPriceRepository {
    type Input = MarkPriceInsert;

    async fn insert(&self, m_price: &Self::Input) -> Result<(), sqlx::Error> {
        let (pool, _) = self.pool.get_pool().await?;
        sqlx::query(
            r#"
                INSERT INTO funding_rates (
                    time, symbol_id, mark_price, index_price, rate
                ) VALUES (?, ?, ?, ?, ?)
            "#,
        )
        .bind(m_price.time)
        .bind(&m_price.symbol)
        .bind(m_price.mark_price.0)
        .bind(m_price.index_price.0)
        .bind(m_price.funding_rate)
        .execute(&pool)
        .await?;
        Ok(())
    }

    async fn insert_batch(&self, m_prices: &[Self::Input]) -> Result<(), sqlx::Error> {
        if m_prices.is_empty() {
            return Ok(());
        }

        let (pool, _) = self.pool.get_pool().await?;
        let mut tx = pool.begin().await?;

        for chunk in m_prices.chunks(BATCH_SIZE) {
            let mut query_builder = QueryBuilder::new(
                r#"
                    INSERT INTO funding_rates (
                        time, symbol_id, mark_price, index_price, rate
                    )
                "#,
            );
            query_builder.push_values(chunk, |mut b, m_price| {
                b.push_bind(m_price.time)
                    .push_bind(&m_price.symbol)
                    .push_bind(m_price.mark_price.0)
                    .push_bind(m_price.index_price.0)
                    .push_bind(m_price.funding_rate);
            });
            let query = query_builder.build();
            query.execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
