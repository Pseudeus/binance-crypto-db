use anyhow::Ok;
use std::{sync::Arc, time::Duration};
use tokio::{
    sync::{Mutex, MutexGuard},
    time,
};
use tracing::error;

use crate::repositories::Repository;

pub struct StorageWriteBuffer<T, R>
where
    T: Send + Sync,
    R: Repository<Input = T>,
{
    repo: Arc<R>,
    buffer: Arc<Mutex<Vec<T>>>,
    buffer_capacity: usize,
}

impl<T, R> StorageWriteBuffer<T, R>
where
    T: Send + Sync + 'static,
    R: Repository<Input = T> + 'static,
{
    pub fn new(repo: R, capacity: usize) -> Self {
        let buffer = Arc::new(Mutex::new(Vec::with_capacity(capacity)));
        let repo = Arc::new(repo);

        let h_buffer = buffer.clone();
        let h_repo = repo.clone();
        tokio::spawn(async move {
            let mut interval = time::interval(Duration::from_mins(60));
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        let mut guard = h_buffer.lock().await;
                        let tmp_buffer = std::mem::take(&mut *guard);
                        drop(guard);
                        if !tmp_buffer.is_empty() {
                            if let Err(e) = h_repo.insert_batch(&tmp_buffer).await {
                                error!("Hourly background flush failed, returning to ram: {}", e);

                                let mut re_guard = h_buffer.lock().await;

                                let old_data = std::mem::take(&mut *re_guard);
                                *re_guard = tmp_buffer;
                                re_guard.extend(old_data);
                            }
                        }
                    }
                }
            }
        });

        Self {
            repo,
            buffer: buffer,
            buffer_capacity: capacity,
        }
    }

    async fn flush(&self, mut guard: MutexGuard<'_, Vec<T>>) -> anyhow::Result<()> {
        let tmp_buffer = std::mem::take(&mut *guard);
        drop(guard);
        if let Err(e) = self.repo.insert_batch(&tmp_buffer).await {
            error!("Buffer limit flush failed: {}", e);

            let mut re_guard = self.buffer.lock().await;

            let old_data = std::mem::take(&mut *re_guard);
            *re_guard = tmp_buffer;
            re_guard.extend(old_data);

            return Err(anyhow::anyhow!("Database write failed: {}", e));
        }
        Ok(())
    }

    pub async fn push(&self, item: T) -> anyhow::Result<()> {
        let mut guard = self.buffer.lock().await;
        guard.push(item);

        if guard.len() >= self.buffer_capacity {
            self.flush(guard).await?;
        }
        Ok(())
    }

    pub async fn close(&self) -> anyhow::Result<()> {
        let guard = self.buffer.lock().await;

        if !guard.is_empty() {
            self.flush(guard).await?;
        }
        Ok(())
    }
}
