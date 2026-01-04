use common::models::OrderBookInsert;

use crate::data_manager::DataManager;

pub struct OrderBookRepository;

impl OrderBookRepository {
    pub async fn insert_batch(
        data_manager: &DataManager,
        books: &[OrderBookInsert],
    ) -> Result<(), sqlx::Error> {
        if books.is_empty() {
            return Ok(());
        }
        let (pool, _) = data_manager.pool_rotator.get_pool().await?;
        let mut tx = pool.begin().await?;

        for b in books {
            let symbol_id = data_manager.get_symbol_id(&b.symbol).await?;
            sqlx::query(
                r#"
                    INSERT INTO order_books(time, symbol_id, bids, asks)
                    VALUES (?, ?, ?, ?)
                "#,
            )
            .bind(b.time)
            .bind(symbol_id)
            .bind(&b.bids)
            .bind(&b.asks)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
