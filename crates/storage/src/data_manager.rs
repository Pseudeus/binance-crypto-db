use common::actors::ControlMessage;
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::{db::RotatingPool, symbol_manager::SymbolManager};

pub struct DataManager {
    pub pool_rotator: RotatingPool,
    symbol_manager: SymbolManager,
}

impl DataManager {
    pub async fn new(
        data_folder: String,
        supervisor_tx: mpsc::Sender<ControlMessage>,
    ) -> Result<Arc<Self>, sqlx::Error> {
        let pool_rotator = RotatingPool::new(data_folder, supervisor_tx).await?;
        Ok(Arc::new(Self {
            pool_rotator,
            symbol_manager: SymbolManager::new(),
        }))
    }

    pub async fn get_symbol_id(&self, ticker: &str) -> Result<i64, sqlx::Error> {
        let (pool, _) = self.pool_rotator.get_pool().await?;

        let id = self
            .symbol_manager
            .get_or_create_id(pool.clone(), ticker)
            .await?;

        return Ok(id);
    }
}
