use dotenvy::dotenv;
use std::{env, sync::Arc};
use tokio::sync::broadcast;
use tracing::debug;

use crate::actors::ActorType;
use crate::actors::supervisor::Supervisor;
use crate::db::RotatingPool;
use crate::services::market_gateway::MarketEvent;

mod actors;
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
async fn main() -> anyhow::Result<()> {
    logger::setup_logger();
    dotenv().ok();
    debug!("System starting up...");

    let data_folder = env::var("WORKDIR")?;
    let rotating_pool = Arc::new(RotatingPool::new(data_folder).await?);

    let (market_tx, _) = broadcast::channel::<Arc<MarketEvent>>(10_000);
    // Create the broadcast channel for AggTrades
    // let (trade_tx, _trade_rx) = broadcast::channel::<Arc<AggTradeInsert>>(10000);
    // Create the broadcast channel for OrderBooks
    // let (order_tx, _order_rx) = broadcast::channel::<Arc<crate::models::OrderBookInsert>>(1000);
    // Create the broadcast channel for Notifications
    // let (notify_tx, notify_rx) = broadcast::channel::<String>(100);
    // Create the broadcast channel for Execution
    // let (exec_tx, exec_rx) = broadcast::channel::<crate::models::TradeSignal>(100);

    let mut supervisor = Supervisor::new();

    let tx_for_gateway = market_tx.clone();
    supervisor.register_actor(
        ActorType::GatewayActor,
        Box::new(move || {
            Box::new(services::MarketGateway::new(
                SYMBOLS,
                tx_for_gateway.clone(),
            ))
        }),
    );

    let pool_for_agg = rotating_pool.clone();
    let tx_for_agg = market_tx.subscribe();
    supervisor.register_actor(
        ActorType::AggTradeActor,
        Box::new(move || {
            Box::new(services::AggTradeService::new(
                pool_for_agg.clone(),
                tx_for_agg.resubscribe(),
            ))
        }),
    );

    let pool_for_order = rotating_pool.clone();
    let tx_for_order = market_tx.subscribe();
    supervisor.register_actor(
        ActorType::OrderBookActor,
        Box::new(move || {
            Box::new(services::OrderBookService::new(
                pool_for_order.clone(),
                tx_for_order.resubscribe(),
            ))
        }),
    );

    let pool_for_klines = rotating_pool.clone();
    let tx_for_klines = market_tx.subscribe();
    supervisor.register_actor(
        ActorType::KlinesActor,
        Box::new(move || {
            Box::new(services::KlinesService::new(
                pool_for_klines.clone(),
                tx_for_klines.resubscribe(),
            ))
        }),
    );

    // let telegram_svc = services::TelegramService::new();
    // let execution_svc = services::ExecutionService::new();

    // Configurable Model Path
    let model_path = env::var("MODEL_PATH").unwrap_or_else(|_| "models/strategy.onnx".to_string());
    debug!("Using AI Model: {}", model_path);

    // Initialize Strategy Service (Process Phase)
    // Tracks all 15 symbols with a window size of 100
    // let strategy_svc = services::StrategyService::new(SYMBOLS, 100, &model_path)
    //     .with_notifier(notify_tx.clone())
    //     .with_executor(exec_tx.clone());

    // let strategy_handle =
    //     tokio::spawn(strategy_svc.start(trade_tx.subscribe(), order_tx.subscribe()));
    // let telegram_handle = tokio::spawn(telegram_svc.start(notify_rx));
    // let execution_handle = tokio::spawn(execution_svc.start(exec_rx));

    supervisor.start().await;
    Ok(())
}
