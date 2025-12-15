use sqlx::SqlitePool;

use crate::models::OrderBookInsert;

pub struct OrderBookRepository;

impl OrderBookRepository {
    pub async fn insert_batch(
        pool: &SqlitePool,
        books: &[OrderBookInsert],
    ) -> Result<(), sqlx::Error> {
        if books.is_empty() {
            return Ok(());
        }

        let mut tx = pool.begin().await?;

        for b in books {
            sqlx::query!(
                r#"
                    INSERT INTO order_books(time, symbol, bids, asks)
                    VALUES (?, ?, ?, ?)
                "#,
                b.time,
                b.symbol,
                b.bids,
                b.asks
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
