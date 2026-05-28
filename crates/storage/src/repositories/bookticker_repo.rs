use std::sync::Arc;

use common::models::book_ticker::BookTickerInsert;
use sqlx::QueryBuilder;

use crate::{
    batch_size, db::RotatingPool, repositories::Repository,
    storage_write_buffer::StorageWriteBuffer,
};

pub type BookTickerWriterBuffer = StorageWriteBuffer<BookTickerInsert, BookTickerRepository>;

pub struct BookTickerRepository {
    pool: Arc<RotatingPool>,
}

impl BookTickerRepository {
    pub fn new(pool: Arc<RotatingPool>) -> Self {
        Self { pool: pool.clone() }
    }
}

impl Repository for BookTickerRepository {
    type Input = BookTickerInsert;

    async fn insert(&self, ticker: &Self::Input) -> Result<(), sqlx::Error> {
        let (pool, _) = self.pool.get_pool().await?;
        sqlx::query(
            r#"
                INSERT INTO spot_book_ticker (
                    receive_time, symbol_id, best_bid_price,
                    best_bid_qty, best_ask_price, best_ask_qty
                ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(ticker.receive_time)
        .bind(&ticker.symbol.0)
        .bind(ticker.best_bid_price.0)
        .bind(ticker.best_bid_qty.0)
        .bind(ticker.best_ask_price.0)
        .bind(ticker.best_ask_qty.0)
        .execute(&pool)
        .await?;
        Ok(())
    }

    async fn insert_batch(&self, tickers: &[Self::Input]) -> Result<(), sqlx::Error> {
        if tickers.is_empty() {
            return Ok(());
        }
        let (pool, _) = self.pool.get_pool().await?;
        let mut tx = pool.begin().await?;

        batch_size!(6);
        for chunk in tickers.chunks(BATCH_SIZE) {
            let mut query_builder = QueryBuilder::new(
                r#"
                    INSERT INTO spot_book_ticker (
                        receive_time, symbol_id, best_bid_price,
                        best_bid_qty, best_ask_price, best_ask_qty
                    )
                "#,
            );
            query_builder.push_values(chunk, |mut b, ticker| {
                b.push_bind(ticker.receive_time)
                    .push_bind(&ticker.symbol.0)
                    .push_bind(ticker.best_bid_price.0)
                    .push_bind(ticker.best_bid_qty.0)
                    .push_bind(ticker.best_ask_price.0)
                    .push_bind(ticker.best_ask_qty.0);
            });
            let query = query_builder.build();
            query.execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
