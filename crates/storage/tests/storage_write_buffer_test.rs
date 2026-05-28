use storage::storage_write_buffer::StorageWriteBuffer;
use storage::repositories::Repository;
use std::future::Future;

struct MockRepository;

impl Repository for MockRepository {
    type Input = i32;
    fn insert(&self, _entry: &Self::Input) -> impl Future<Output = Result<(), sqlx::Error>> + Send {
        async { Ok(()) }
    }
    fn insert_batch(
        &self,
        _entries: &[Self::Input],
    ) -> impl Future<Output = Result<(), sqlx::Error>> + Send {
        async { Ok(()) }
    }
}

#[tokio::test]
async fn test_storage_write_buffer_deadlock() {
    let repo = MockRepository;
    let buffer = StorageWriteBuffer::new(repo, 2);

    println!("Pushing 1");
    buffer.push(1).await.unwrap();
    println!("Pushing 2");
    buffer.push(2).await.unwrap();
    
    // This should trigger flush and deadlock
    println!("Pushing 3 (should flush)");
    let push_future = buffer.push(3);
    
    match tokio::time::timeout(std::time::Duration::from_secs(1), push_future).await {
        Ok(result) => {
            result.expect("Push failed");
            println!("Pushing 3 succeeded");
        }
        Err(_) => {
            panic!("Deadlock detected in StorageWriteBuffer::push!");
        }
    }
}
