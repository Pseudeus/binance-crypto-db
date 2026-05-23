pub mod aggtrade_repo;
pub mod forceorder_repo;
pub mod klines_repo;
pub mod markprice_repo;
pub mod openinterest_repo;
pub mod orderbook_repo;

pub use aggtrade_repo::AggTradeRepository;
pub use forceorder_repo::ForceOrderRepository;
pub use klines_repo::KlineRepository;
pub use markprice_repo::MarkPriceRepository;
pub use openinterest_repo::OpenInterestRepository;
pub use orderbook_repo::OrderBookRepository;

pub trait Repository {
    type Input;
    fn insert(&self, entry: &Self::Input) -> impl Future<Output = Result<(), sqlx::Error>> + Send;
    fn insert_batch(
        &self,
        entries: &[Self::Input],
    ) -> impl Future<Output = Result<(), sqlx::Error>> + Send;
}
