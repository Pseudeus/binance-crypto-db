use common::models::ForceOrderInsert;

use crate::data_manager::DataManager;

pub struct ForceOrderRepository;

impl ForceOrderRepository {
    pub async fn insert_batch(
        data_manager: &DataManager,
        orders: &[ForceOrderInsert],
    ) -> Result<(), sqlx::Error> {
        if orders.is_empty() {
            return Ok(());
        }
        let (pool, _) = data_manager.pool_rotator.get_pool().await?;
        let mut tx = pool.begin().await?;

        for order in orders {
            let symbol_id = data_manager.get_symbol_id(&order.symbol).await?;
            sqlx::query(
                r#"
                    INSERT INTO liquidations (
                        time, symbol_id, side, price, quantity
                    ) VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(order.time)
            .bind(symbol_id)
            .bind(order.side.clone())
            .bind(order.price)
            .bind(order.quantity)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
