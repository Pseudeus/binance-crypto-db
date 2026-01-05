use common::models::MarkPriceInsert;

use crate::data_manager::DataManager;

pub struct MarkPriceRepository;

impl MarkPriceRepository {
    pub async fn insert_batch(
        data_manager: &DataManager,
        m_prices: &[MarkPriceInsert],
    ) -> Result<(), sqlx::Error> {
        if m_prices.is_empty() {
            return Ok(());
        }

        let (pool, _) = data_manager.pool_rotator.get_pool().await?;
        let mut tx = pool.begin().await?;

        for m_price in m_prices {
            let symbol_id = data_manager.get_symbol_id(&m_price.symbol).await?;
            sqlx::query(
                r#"
                    INSERT INTO funding_rates (
                        time, symbol_id, mark_price, index_price, rate
                    ) VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(m_price.time)
            .bind(symbol_id)
            .bind(m_price.mark_price)
            .bind(m_price.index_price)
            .bind(m_price.funding_rate)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
