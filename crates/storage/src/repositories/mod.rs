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
    fn insert(conn: &rusqlite::Connection, entry: &Self::Input) -> rusqlite::Result<()>;
    fn insert_batch(tx: &rusqlite::Transaction, entries: &[Self::Input]) -> rusqlite::Result<()>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::models::{
        AggTradeInsert, ForceOrderInsert, KlineInsert, MarkPriceInsert, OpenInterestInsert,
        OrderBookInsert, Price, Quantity, Symbol,
    };
    use rusqlite::Connection;

    fn create_test_db() -> Connection {
        let conn = Connection::open_in_memory().expect("Failed to open in-memory db");
        let schema = include_str!("../../migrations/schema.sql");
        conn.execute_batch(schema).expect("Failed to execute schema");
        conn
    }

    #[test]
    fn test_agg_trade_repo() {
        let mut conn = create_test_db();
        let trade = AggTradeInsert {
            time: 1_700_000_000.0,
            symbol: Symbol("BTCUSDT".to_string()),
            price: Price(50000.0),
            quantity: Quantity(1.5),
            is_buyer_maker: true,
        };

        AggTradeRepository::insert(&conn, &trade).expect("Failed to insert single trade");

        let trades = vec![
            trade.clone(),
            AggTradeInsert {
                time: 1_700_000_001.0,
                symbol: Symbol("ETHUSDT".to_string()),
                price: Price(3000.0),
                quantity: Quantity(10.0),
                is_buyer_maker: false,
            },
        ];

        let tx = conn.transaction().expect("Failed to create tx");
        AggTradeRepository::insert_batch(&tx, &trades).expect("Failed to insert batch trades");
        tx.commit().expect("Failed to commit tx");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM agg_trades", [], |row| row.get(0))
            .expect("Query failed");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_order_book_repo() {
        let mut conn = create_test_db();
        let book = OrderBookInsert {
            time: 1_700_000_000.0,
            symbol: "BTCUSDT".to_string(),
            bids: vec![1, 2, 3],
            asks: vec![4, 5, 6],
        };

        OrderBookRepository::insert(&conn, &book).expect("Failed to insert single book");

        let books = vec![book.clone(), book];
        let tx = conn.transaction().expect("Failed to create tx");
        OrderBookRepository::insert_batch(&tx, &books).expect("Failed to insert batch books");
        tx.commit().expect("Failed to commit tx");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM order_books", [], |row| row.get(0))
            .expect("Query failed");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_klines_repo() {
        let mut conn = create_test_db();
        let kline = KlineInsert {
            symbol: "BTCUSDT".to_string(),
            start_time: 1_700_000_000,
            close_time: 1_700_000_059,
            open_price: 50000.0,
            close_price: 50100.0,
            high_price: 50200.0,
            low_price: 49900.0,
            volume: 100.0,
            no_of_trades: 500,
            taker_buy_vol: 60.0,
        };

        KlineRepository::insert(&conn, &kline).expect("Failed to insert single kline");

        let klines = vec![kline.clone(), kline];
        let tx = conn.transaction().expect("Failed to create tx");
        KlineRepository::insert_batch(&tx, &klines).expect("Failed to insert batch klines");
        tx.commit().expect("Failed to commit tx");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM klines_1s", [], |row| row.get(0))
            .expect("Query failed");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_mark_price_repo() {
        let mut conn = create_test_db();
        let m_price = MarkPriceInsert {
            time: 1_700_000_000.0,
            symbol: "BTCUSDT".to_string(),
            mark_price: Price(50000.0),
            index_price: Price(50005.0),
            funding_rate: 0.0001,
        };

        MarkPriceRepository::insert(&conn, &m_price).expect("Failed to insert single mark price");

        let prices = vec![m_price.clone(), m_price];
        let tx = conn.transaction().expect("Failed to create tx");
        MarkPriceRepository::insert_batch(&tx, &prices).expect("Failed to insert batch mark prices");
        tx.commit().expect("Failed to commit tx");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM funding_rates", [], |row| row.get(0))
            .expect("Query failed");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_open_interest_repo() {
        let mut conn = create_test_db();
        let oi = OpenInterestInsert {
            time: 1_700_000_000.0,
            symbol: "BTCUSDT".to_string(),
            oi_value: 123456.78,
        };

        OpenInterestRepository::insert(&conn, &oi).expect("Failed to insert single oi");

        let ois = vec![oi.clone(), oi];
        let tx = conn.transaction().expect("Failed to create tx");
        OpenInterestRepository::insert_batch(&tx, &ois).expect("Failed to insert batch ois");
        tx.commit().expect("Failed to commit tx");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM open_interest", [], |row| row.get(0))
            .expect("Query failed");
        assert_eq!(count, 3);
    }

    #[test]
    fn test_force_order_repo() {
        let mut conn = create_test_db();
        let order = ForceOrderInsert {
            time: 1_700_000_000.0,
            symbol: "BTCUSDT".to_string(),
            side: "BUY".to_string(),
            price: Price(49500.0),
            quantity: Quantity(0.5),
        };

        ForceOrderRepository::insert(&conn, &order).expect("Failed to insert single force order");

        let orders = vec![order.clone(), order];
        let tx = conn.transaction().expect("Failed to create tx");
        ForceOrderRepository::insert_batch(&tx, &orders).expect("Failed to insert batch force orders");
        tx.commit().expect("Failed to commit tx");

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM liquidations", [], |row| row.get(0))
            .expect("Query failed");
        assert_eq!(count, 3);
    }
}
