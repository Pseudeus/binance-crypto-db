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
            sqlx::query!(
                r#"
                    INSERT INTO agg_trades (
                        time, symbol, agg_trade_id, price, quantity,
                        first_trade_id, last_trade_id, is_buyer_maker
                    ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                trade.time,
                trade.symbol,
                trade.agg_trade_id,
                trade.price,
                trade.quantity,
                trade.first_trade_id,
                trade.last_trade_id,
                trade.is_buyer_maker
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}
