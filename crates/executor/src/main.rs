use dotenvy::dotenv;
use std::{env, sync::Arc};
use tokio::sync::broadcast;
use tracing::debug;

use common::logger;
use common::actors::ActorType;
use storage::db::RotatingPool;
use market_data::services::market_gateway::{MarketEvent, MarketGateway};
use market_data::services::aggtrade_service::AggTradeService;
use market_data::services::orderbook_service::OrderBookService;
use market_data::services::klines_service::KlinesService;

use crate::actors::supervisor::Supervisor;

mod actors;
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

    let mut supervisor = Supervisor::new();
    let supervisor_tx = supervisor.sender();

    let data_folder = env::var("WORKDIR")?;
    let rotating_pool = Arc::new(RotatingPool::new(data_folder, supervisor_tx).await?);

    let (market_tx, _) = broadcast::channel::<Arc<MarketEvent>>(10_000);

    let tx_for_gateway = market_tx.clone();
    supervisor.register_actor(
        ActorType::GatewayActor,
        Box::new(move || {
            Box::new(MarketGateway::new(
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
            Box::new(AggTradeService::new(
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
            Box::new(OrderBookService::new(
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
            Box::new(KlinesService::new(
                pool_for_klines.clone(),
                tx_for_klines.resubscribe(),
            ))
        }),
    );

    // let telegram_svc = services::telegram_service::TelegramService::new();
    // let execution_svc = services::execution_service::ExecutionService::new();

    // Configurable Model Path
    let model_path = env::var("MODEL_PATH").unwrap_or_else(|_| "models/strategy.onnx".to_string());
    debug!("Using AI Model: {}", model_path);

    // Initialize Strategy Service (Process Phase)
    // Tracks all 15 symbols with a window size of 100
    // let strategy_svc = strategy::services::strategy_service::StrategyService::new(SYMBOLS, 100, &model_path)
    //     .with_notifier(notify_tx.clone())
    //     .with_executor(exec_tx.clone());

    supervisor.start().await;
    Ok(())
}
