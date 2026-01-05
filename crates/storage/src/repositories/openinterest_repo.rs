use common::models::OpenInterestInsert;

use crate::data_manager::DataManager;

pub struct OpenInterestRepository;

impl OpenInterestRepository {
    pub async fn insert_batch(
        data_manager: &DataManager,
        interests: &[OpenInterestInsert],
    ) -> Result<(), sqlx::Error> {
        if interests.is_empty() {
            return Ok(());
        }

        let (pool, _) = data_manager.pool_rotator.get_pool().await?;
        let mut tx = pool.begin().await?;

        for interest in interests {
            let symbol_id = data_manager.get_symbol_id(&interest.symbol).await?;
            sqlx::query(
                r#"
                    INSERT INTO open_interest (
                        time, symbol_id, oi_value
                    ) VALUES (?, ?, ?)
                "#,
            )
            .bind(interest.time)
            .bind(symbol_id)
            .bind(interest.oi_value)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
