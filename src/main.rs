use dotenvy::dotenv;
use std::{env, sync::Arc};
use tokio::sync::broadcast;
use tracing::{debug, error};

use crate::db::RotatingPool;
use crate::models::AggTradeInsert;

mod db;
mod inference;
mod logger;
mod models;
mod remote;
mod repositories;
mod services;

const SYMBOLS: &[&str; 15] = &[
    // Core (7)
    "btcusdt",
    "ethusdt",
    "bnbusdt",
    "solusdt",
    "avaxusdt",
    "nearusdt",
    "maticusdt",
    // Alpha (5)
    "dogeusdt",
    "shibusdt",
    "pepeusdt",
    "wifiusdt",
    "bonkusdt",
    // Macro (3)
    "xrpusdt",
    "adausdt",
    "dotusdt",
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    logger::setup_logger();
    dotenv().ok();
    debug!("System starting up...");

    let data_folder = env::var("WORKDIR")?;
    let rotating_pool = Arc::new(RotatingPool::new(data_folder).await?);

    // Create the broadcast channel for AggTrades
    let (trade_tx, _trade_rx) = broadcast::channel::<Arc<AggTradeInsert>>(10000);
    // Create the broadcast channel for OrderBooks
    let (order_tx, _order_rx) = broadcast::channel::<Arc<crate::models::OrderBookInsert>>(1000);

        let agg_svc = services::AggTradeService::new(SYMBOLS);
        let order_svc = services::OrderBookService::new(SYMBOLS);
        
        // Configurable Model Path
        let model_path = env::var("MODEL_PATH").unwrap_or_else(|_| "models/strategy.onnx".to_string());
        debug!("Using AI Model: {}", model_path);
    
        // Initialize Strategy Service (Process Phase)
        // Tracks all 15 symbols with a window size of 100
        let strategy_svc = services::StrategyService::new(SYMBOLS, 100, &model_path);
    
        let agg_handle = tokio::spawn(agg_svc.start(rotating_pool.clone(), trade_tx.clone()));    let order_handle = tokio::spawn(order_svc.start(rotating_pool, order_tx.clone()));
    let strategy_handle =
        tokio::spawn(strategy_svc.start(trade_tx.subscribe(), order_tx.subscribe()));

    tokio::select! {
        _ = agg_handle => error!("AggTrade service stopped"),
        _ = order_handle => error!("OrderBook service stopped"),
        _ = strategy_handle => error!("Strategy service stopped"),
    }
    Ok(())
}
