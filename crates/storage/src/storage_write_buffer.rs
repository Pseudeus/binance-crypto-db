use std::sync::Arc;

use anyhow::Ok;
use tokio::sync::Mutex;

use crate::repositories::Repository;

pub struct StorageWriteBuffer<T, R>
where
    R: Repository<Input = T>,
{
    repo: R,
    buffer: Arc<Mutex<Vec<T>>>,
    buffer_capacity: usize,
}

impl<T, R: Repository<Input = T>> StorageWriteBuffer<T, R> {
    pub fn new(repo: R, capacity: usize) -> Self {
        let buffer = Vec::with_capacity(capacity);
        Self {
            repo,
            buffer: Arc::new(Mutex::new(buffer)),
            buffer_capacity: capacity,
        }
    }

    async fn flush(&self) -> anyhow::Result<()> {
        let mut guard = self.buffer.lock().await;
        self.repo.insert_batch(&guard).await?;
        guard.clear();
        drop(guard);
        Ok(())
    }

    pub async fn push(&self, item: T) -> anyhow::Result<()> {
        let mut guard = self.buffer.lock().await;

        if guard.len() < self.buffer_capacity {
            guard.push(item);
        } else {
            drop(guard);
            self.flush().await?;
            guard = self.buffer.lock().await;
            guard.push(item);
        }
        drop(guard);
        Ok(())
    }

    pub async fn close(&self) -> anyhow::Result<()> {
        let guard = self.buffer.lock().await;

        if !guard.is_empty() {
            drop(guard);
            self.flush().await?;
        }
        Ok(())
    }
}
