use common::models::OpenInterestInsert;
use sqlx::QueryBuilder;
use std::sync::Arc;

use crate::{db::RotatingPool, repositories::Repository, storage_write_buffer::StorageWriteBuffer};

const BATCH_SIZE: usize = (i16::MAX / 3) as usize;

pub type OpenInterestWriterBuffer = StorageWriteBuffer<OpenInterestInsert, OpenInterestRepository>;

pub struct OpenInterestRepository {
    pool: Arc<RotatingPool>,
}

impl OpenInterestRepository {
    pub fn new(pool: Arc<RotatingPool>) -> Self {
        Self { pool: pool.clone() }
    }
}

impl Repository for OpenInterestRepository {
    type Input = OpenInterestInsert;

    async fn insert(&self, interest: &Self::Input) -> Result<(), sqlx::Error> {
        let (pool, _) = self.pool.get_pool().await?;
        sqlx::query(
            r#"
                INSERT INTO open_interest (
                    time, symbol_id, oi_value
                ) VALUES (?, ?, ?)
            "#,
        )
        .bind(interest.time)
        .bind(&interest.symbol)
        .bind(interest.oi_value)
        .execute(&pool)
        .await?;
        Ok(())
    }

    async fn insert_batch(&self, interests: &[Self::Input]) -> Result<(), sqlx::Error> {
        if interests.is_empty() {
            return Ok(());
        }

        let (pool, _) = self.pool.get_pool().await?;
        let mut tx = pool.begin().await?;

        for chunk in interests.chunks(BATCH_SIZE) {
            let mut query_builder = QueryBuilder::new(
                r#"
                    INSERT INTO open_interest (
                        time, symbol_id, oi_value
                    )
                "#,
            );
            query_builder.push_values(chunk, |mut b, interest| {
                b.push_bind(interest.time)
                    .push_bind(&interest.symbol)
                    .push_bind(interest.oi_value);
            });
            let query = query_builder.build();
            query.execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
