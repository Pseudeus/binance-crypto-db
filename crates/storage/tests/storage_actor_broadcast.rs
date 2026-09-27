//! Integration test for StorageActor broadcast -> DbWriter -> SQLite persistence.
//!
//! This test verifies that:
//! 1. MarketEvent broadcasts are received by StorageActor
//! 2. Events are flushed via channel to dedicated DbWriter and persisted to SQLite
//! 3. AccountUpdate and ExecutionReport events are handled gracefully

use common::actors::Actor;
use common::models::{AccountUpdate, MarketEvent, Price, Quantity, Symbol};
use std::sync::Arc;
use storage::actors::storage_actor::StorageActor;
use storage::db::{get_date_components, DbWriterHandle};
use tokio::sync::{broadcast, mpsc};

/// Integration test: Verify StorageActor broadcasts are persisted to SQLite
#[tokio::test]
async fn test_storage_actor_persists_agg_trade() {
    let data_folder = format!("/tmp/storage_test_{}", std::process::id());
    tokio::fs::create_dir_all(format!("{}/sqlitedata/current", data_folder))
        .await
        .unwrap();

    let (supervisor_tx, _) = mpsc::channel(512);
    let db_writer = DbWriterHandle::spawn(data_folder.clone(), supervisor_tx.clone()).unwrap();

    // Create broadcast channel for events
    let (market_tx, _) = broadcast::channel::<Arc<MarketEvent>>(100);

    // Create StorageActor
    let market_rx = market_tx.subscribe();
    let mut storage_actor = StorageActor::new(db_writer.sender(), market_rx);

    // Spawn the actor (will run in background)
    let (supervisor_tx_test, _) = mpsc::channel(1);
    let actor_handle = tokio::spawn(async move {
        storage_actor.run(supervisor_tx_test).await.unwrap();
    });

    // Give actor time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    // Broadcast a test AggTrade event
    let test_trade = common::models::AggTradeInsert {
        symbol: Symbol("BTCUSDT".to_string()),
        time: 1_234_567_890_000.0,
        price: Price(100.5),
        quantity: Quantity(0.5),
        is_buyer_maker: true,
    };

    let event = Arc::new(MarketEvent::AggTrade(test_trade));
    let _ = market_tx.send(event);

    // Drop sender to close broadcast channel and trigger StorageActor shutdown & flush
    drop(market_tx);

    // Wait for actor to finish and flush buffers
    let _ = actor_handle.await;

    // Drop writer sender and wait briefly for dedicated worker thread to commit
    drop(db_writer);
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Verify row was written to weekly database file
    let (year, week) = get_date_components(chrono::Utc::now());
    let db_file = format!(
        "{}/sqlitedata/current/crypto_{}_{:02}.db",
        data_folder, year, week
    );
    let conn = rusqlite::Connection::open(&db_file).expect("Failed to open db file");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM agg_trades", [], |row| row.get(0))
        .expect("Failed to query row count");
    assert_eq!(count, 1);

    // Cleanup
    let _ = tokio::fs::remove_dir_all(data_folder).await;
}

/// Integration test: Verify AccountUpdate events are handled without errors
#[tokio::test]
async fn test_storage_actor_handles_account_update() {
    let data_folder = format!("/tmp/storage_test_account_{}", std::process::id());
    tokio::fs::create_dir_all(format!("{}/sqlitedata/current", data_folder))
        .await
        .unwrap();

    let (supervisor_tx, _) = mpsc::channel(512);
    let db_writer = DbWriterHandle::spawn(data_folder.clone(), supervisor_tx).unwrap();

    let (market_tx, _) = broadcast::channel::<Arc<MarketEvent>>(100);
    let market_rx = market_tx.subscribe();
    let mut storage_actor = StorageActor::new(db_writer.sender(), market_rx);

    let (supervisor_tx_test, _) = mpsc::channel(1);
    let actor_handle = tokio::spawn(async move {
        storage_actor.run(supervisor_tx_test).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let account_update = AccountUpdate {
        asset: "USDT".to_string(),
        free: 1000.0,
        locked: 0.0,
    };

    let event = Arc::new(MarketEvent::AccountUpdate(vec![account_update]));
    let _ = market_tx.send(event);

    drop(market_tx);
    let _ = actor_handle.await;

    let _ = tokio::fs::remove_dir_all(data_folder).await;
}

/// Integration test: Verify ExecutionReport events are handled without errors
#[tokio::test]
async fn test_storage_actor_handles_execution_report() {
    let data_folder = format!("/tmp/storage_test_execution_{}", std::process::id());
    tokio::fs::create_dir_all(format!("{}/sqlitedata/current", data_folder))
        .await
        .unwrap();

    let (supervisor_tx, _) = mpsc::channel(512);
    let db_writer = DbWriterHandle::spawn(data_folder.clone(), supervisor_tx).unwrap();

    let (market_tx, _) = broadcast::channel::<Arc<MarketEvent>>(100);
    let market_rx = market_tx.subscribe();
    let mut storage_actor = StorageActor::new(db_writer.sender(), market_rx);

    let (supervisor_tx_test, _) = mpsc::channel(1);
    let actor_handle = tokio::spawn(async move {
        storage_actor.run(supervisor_tx_test).await.unwrap();
    });

    tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

    let execution_report = common::models::ExecutionReport {
        symbol: "BTCUSDT".to_string(),
        side: "BUY".to_string(),
        price: Price(100.5),
        quantity: Quantity(0.5),
        status: "FILLED".to_string(),
        commission: 0.001,
        commission_asset: "BNB".to_string(),
    };

    let event = Arc::new(MarketEvent::ExecutionReport(execution_report));
    let _ = market_tx.send(event);

    drop(market_tx);
    let _ = actor_handle.await;

    let _ = tokio::fs::remove_dir_all(data_folder).await;
}
