pub mod aggtrade_repo;
pub mod bookticker_repo;
pub mod forceorder_repo;
pub mod klines_repo;
pub mod orderbook_repo;

pub use aggtrade_repo::AggTradeRepository;
pub use forceorder_repo::ForceOrderRepository;
pub use klines_repo::KlineRepository;
pub use orderbook_repo::OrderBookRepository;

#[macro_export]
macro_rules! batch_size {
    ($params:literal) => {
        const BATCH_SIZE: usize = ((i16::MAX - 1) / $params) as usize;
    };
}

pub trait Repository: Send + Sync {
    type Input;
    fn insert(&self, entry: &Self::Input) -> impl Future<Output = Result<(), sqlx::Error>> + Send;
    fn insert_batch(
        &self,
        entries: &[Self::Input],
    ) -> impl Future<Output = Result<(), sqlx::Error>> + Send;
}
