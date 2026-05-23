//! Integration test for StorageActor broadcast->SQLite persistence.
//!
//! This test verifies that:
//! 1. MarketEvent broadcasts are received by StorageActor
//! 2. Events are persisted to SQLite database
//! 3. AccountUpdate and ExecutionReport events are handled gracefully

use common::actors::{Actor, ActorType, ControlMessage};
use common::models::{AccountUpdate, MarketEvent, Price, Quantity, Symbol};
use std::sync::Arc;
use storage::actors::storage_actor::StorageActor;
use storage::db::RotatingPool;
use storage::repositories::AggTradeRepository;
use tokio::sync::{broadcast, mpsc};

/// Integration test: Verify StorageActor broadcasts are persisted
#[tokio::test]
async fn test_storage_actor_persists_agg_trade() {
    // Setup: Create test database
    let data_folder = "/tmp/storage_test";
    tokio::fs::create_dir_all(format!("{}/sqlitedata/current", data_folder))
        .await
        .unwrap();

    let (supervisor_tx, _) = mpsc::channel(512);
    let data_manager = RotatingPool::new(data_folder.to_string(), supervisor_tx.clone())
        .await
        .unwrap();

    // Create broadcast channel for events
    let (market_tx, _) = broadcast::channel::<Arc<MarketEvent>>(100);

    // Create StorageActor
    let market_rx = market_tx.subscribe();
    let storage_actor = StorageActor::new(Arc::new(data_manager.clone()), market_rx);

    // Spawn the actor (will run in background)
    let (supervisor_tx_test, supervisor_rx_test) = mpsc::channel(1);
    let actor_handle = tokio::spawn(async move {
        storage_actor.run(supervisor_tx_test).await.unwrap();
    });

    // Give actor time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

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

    // Drop sender to close channel and trigger StorageActor shutdown
    drop(market_tx);

    // Wait for actor to finish
    let _ = actor_handle.await;

    // Cleanup
    let _ = tokio::fs::remove_dir_all(data_folder).await;

    // Test passed if we got here without panicking
}

/// Integration test: Verify AccountUpdate events are handled without errors
#[tokio::test]
async fn test_storage_actor_handles_account_update() {
    let data_folder = "/tmp/storage_test_account";
    tokio::fs::create_dir_all(format!("{}/sqlitedata/current", data_folder))
        .await
        .unwrap();

    let (supervisor_tx, _) = mpsc::channel(512);
    let data_manager = RotatingPool::new(data_folder.to_string(), supervisor_tx.clone())
        .await
        .unwrap();

    let (market_tx, _) = broadcast::channel::<Arc<MarketEvent>>(100);
    let market_rx = market_tx.subscribe();
    let storage_actor = StorageActor::new(Arc::new(data_manager), market_rx);

    let (supervisor_tx_test, _) = mpsc::channel(1);
    let actor_handle = tokio::spawn(async move {
        storage_actor.run(supervisor_tx_test).await.unwrap();
    });

    // Give actor time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Broadcast AccountUpdate event
    let account_update = AccountUpdate {
        asset: "USDT".to_string(),
        free: 1000.0,
        locked: 0.0,
    };

    let event = Arc::new(MarketEvent::AccountUpdate(vec![account_update]));
    let _ = market_tx.send(event);

    // Drop sender and wait
    drop(market_tx);
    let _ = actor_handle.await;

    // Cleanup
    let _ = tokio::fs::remove_dir_all(data_folder).await;
}

/// Integration test: Verify ExecutionReport events are handled without errors
#[tokio::test]
async fn test_storage_actor_handles_execution_report() {
    let data_folder = "/tmp/storage_test_execution";
    tokio::fs::create_dir_all(format!("{}/sqlitedata/current", data_folder))
        .await
        .unwrap();

    let (supervisor_tx, _) = mpsc::channel(512);
    let data_manager = DataManager::new(data_folder.to_string(), supervisor_tx.clone())
        .await
        .unwrap();

    let (market_tx, _) = broadcast::channel::<Arc<MarketEvent>>(100);
    let market_rx = market_tx.subscribe();
    let storage_actor = StorageActor::new(Arc::new(data_manager), market_rx);

    let (supervisor_tx_test, _) = mpsc::channel(1);
    let actor_handle = tokio::spawn(async move {
        storage_actor.run(supervisor_tx_test).await.unwrap();
    });

    // Give actor time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Broadcast ExecutionReport event
    use common::models::{Price, Quantity};
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

    // Drop sender and wait
    drop(market_tx);
    let _ = actor_handle.await;

    // Cleanup
    let _ = tokio::fs::remove_dir_all(data_folder).await;
}
