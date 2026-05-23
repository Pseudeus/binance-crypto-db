use common::models::OrderBookInsert;
use sqlx::QueryBuilder;
use std::sync::Arc;

use crate::{db::RotatingPool, repositories::Repository, storage_write_buffer::StorageWriteBuffer};

const BATCH_SIZE: usize = (i16::MAX / 4) as usize;

pub type OrderBookWriterBuffer = StorageWriteBuffer<OrderBookInsert, OrderBookRepository>;

pub struct OrderBookRepository {
    pool: Arc<RotatingPool>,
}

impl OrderBookRepository {
    pub fn new(pool: Arc<RotatingPool>) -> Self {
        Self { pool: pool.clone() }
    }
}

impl Repository for OrderBookRepository {
    type Input = OrderBookInsert;

    async fn insert(&self, book: &Self::Input) -> Result<(), sqlx::Error> {
        let (pool, _) = self.pool.get_pool().await?;
        sqlx::query(
            r#"
                INSERT INTO order_books(time, symbol_id, bids, asks)
                VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(book.time)
        .bind(&book.symbol)
        .bind(&book.bids)
        .bind(&book.asks)
        .execute(&pool)
        .await?;
        Ok(())
    }

    async fn insert_batch(&self, books: &[Self::Input]) -> Result<(), sqlx::Error> {
        if books.is_empty() {
            return Ok(());
        }
        let (pool, _) = self.pool.get_pool().await?;
        let mut tx = pool.begin().await?;

        for chunk in books.chunks(BATCH_SIZE) {
            let mut query_builder = QueryBuilder::new(
                r#"
                    INSERT INTO order_books(time, symbol_id, bids, asks)
                "#,
            );
            query_builder.push_values(chunk, |mut b, book| {
                b.push_bind(book.time)
                    .push_bind(&book.symbol)
                    .push_bind(&book.bids)
                    .push_bind(&book.asks);
            });
            let query = query_builder.build();
            query.execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
