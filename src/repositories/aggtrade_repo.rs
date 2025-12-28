use sqlx::SqlitePool;

use crate::models::AggTradeInsert;

pub struct AggTradeRepository;

impl AggTradeRepository {
    pub async fn insert_batch(
        pool: &SqlitePool,
        trades: &[AggTradeInsert],
    ) -> Result<(), sqlx::Error> {
        if trades.is_empty() {
            return Ok(());
        }

        let mut tx = pool.begin().await?;

        for trade in trades {
            sqlx::query(
                r#"
                    INSERT INTO agg_trades (
                        time, symbol, price, quantity, is_buyer_maker
                    ) VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(trade.time)
            .bind(&trade.symbol)
            .bind(trade.price)
            .bind(trade.quantity)
            .bind(trade.is_buyer_maker)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
