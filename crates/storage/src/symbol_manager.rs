use sqlx::SqlitePool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct SymbolManager {
    cache: Arc<Mutex<HashMap<String, i64>>>,
}

impl SymbolManager {
    pub fn new() -> Self {
        Self {
            cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn get_or_create_id(
        &self,
        pool: SqlitePool,
        symbol: &str,
    ) -> Result<i64, sqlx::Error> {
        {
            let cache = self.cache.lock().await;
            if let Some(&id) = cache.get(symbol) {
                return Ok(id);
            }
        }

        let mut tx = pool.begin().await?;

        let id_opt = sqlx::query_scalar::<_, i64>("SELECT id FROM symbols WHERE ticker = ?")
            .bind(symbol)
            .fetch_optional(&mut *tx)
            .await?;

        let id = if let Some(existing_id) = id_opt {
            existing_id
        } else {
            let new_id =
                sqlx::query_scalar::<_, i64>("INSERT INTO symbols(ticker) VALUES (?) RETURNING id")
                    .bind(symbol)
                    .fetch_one(&mut *tx)
                    .await?;
            new_id
        };
        tx.commit().await?;

        let mut cache = self.cache.lock().await;
        cache.insert(symbol.to_string(), id);

        Ok(id)
    }

    pub async fn clear_cache(&mut self) {
        let mut cache = self.cache.lock().await;
        cache.clear();
    }

    pub async fn get_cache(&self, ticker: &str) -> Option<i64> {
        let cache = self.cache.lock().await;
        cache.get(ticker).cloned()
    }
}
