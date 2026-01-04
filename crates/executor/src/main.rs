use dotenvy::dotenv;
use market_data::services::forceorder_service::ForceOrderService;
use market_data::services::markprice_service::MarkPriceService;
use std::{env, sync::Arc};
use storage::data_manager::DataManager;
use tokio::sync::broadcast;
use tracing::debug;

use common::actors::ActorType;
use common::logger;
use market_data::services::aggtrade_service::AggTradeService;
use market_data::services::klines_service::KlinesService;
use market_data::services::market_gateway::{MarketEvent, MarketGateway};
use market_data::services::orderbook_service::OrderBookService;

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
    let data_manager = DataManager::new(data_folder, supervisor_tx).await?;

    let (market_tx, _) = broadcast::channel::<Arc<MarketEvent>>(10_000);

    let tx_for_gateway = market_tx.clone();
    supervisor.register_actor(
        ActorType::GatewayActor,
        Box::new(move || Box::new(MarketGateway::new(SYMBOLS, tx_for_gateway.clone()))),
    );

    let pool_for_agg = data_manager.clone();
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

    let pool_for_order = data_manager.clone();
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

    let pool_for_klines = data_manager.clone();
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

    let pool_for_mark_prices = data_manager.clone();
    let tx_for_mark_prices = market_tx.subscribe();
    supervisor.register_actor(
        ActorType::MarkPriceActor,
        Box::new(move || {
            Box::new(MarkPriceService::new(
                pool_for_mark_prices.clone(),
                tx_for_mark_prices.resubscribe(),
            ))
        }),
    );

    let pool_for_force_order = data_manager.clone();
    let tx_for_force_order = market_tx.subscribe();
    supervisor.register_actor(
        ActorType::ForceOrderActor,
        Box::new(move || {
            Box::new(ForceOrderService::new(
                pool_for_force_order.clone(),
                tx_for_force_order.resubscribe(),
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
