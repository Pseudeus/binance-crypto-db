use anyhow::Context;
use dotenvy::dotenv;
use std::{env, sync::Arc};
use storage::db::RotatingPool;
use tokio::signal;
use tokio::sync::broadcast;
use tracing::debug;

use tokio::signal::unix::{Signal, SignalKind, signal};

use common::actors::ActorType;
use common::logger;
use market_data::services::futures_stream_service::FuturesStreamActor;
use market_data::services::public_stream_service::PublicStreamActor;
use market_data::services::user_stream_service::UserStreamActor;
use storage::actors::storage_actor::StorageActor;

use crate::actors::supervisor::Supervisor;

mod actors;
mod services;

const SYMBOLS: &[&str; 15] = &[
    // Core (7)
    "btcusdt", "ethusdt", "bnbusdt", "solusdt", "avaxusdt", "nearusdt", "polusdt",
    // Alpha (5)
    "dogeusdt", "shibusdt", "pepeusdt", "wifusdt", "bonkusdt", // Macro (3)
    "xrpusdt", "adausdt", "dotusdt",
];

macro_rules! regist_actor {
    ($supervisor:ident, $market_tx:ident, $actor:ident) => {
        let tx_for_public = $market_tx.clone();
        $supervisor.register_actor(
            ActorType::$actor,
            Box::new(move || Box::new($actor::new(SYMBOLS, tx_for_public.clone()))),
        );
    };
    ($supervisor:ident, $market_tx:ident, $pool_mgr:ident, $actor:ident) => {
        let dm_for_storage = $pool_mgr.clone();
        let rx_for_storage = $market_tx.subscribe();
        $supervisor.register_actor(
            ActorType::$actor,
            Box::new(move || {
                Box::new($actor::new(
                    dm_for_storage.clone(),
                    rx_for_storage.resubscribe(),
                ))
            }),
        );
    };
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // let ctrl_c = async {
    //     signal::ctrl_c()
    //         .await
    //         .context("Failed to install SIGINT handler")
    // };
    // let terminate = async {
    //     signal(SignalKind::terminate())
    //         .expect("Failed to install SIGTERM handler")
    //         .recv()
    //         .await
    // };

    // tokio::select! {
    //     _ = ctrl_c => {
    //         println!("\n[Signal] Received SIGINT (Ctrl+C)");
    //     },
    //     _ = terminate => {
    //         println!("\n[Signal] Received SIGTERM");
    //     }
    // }

    logger::setup_logger();
    dotenv().ok();
    debug!("System starting up...");

    let mut supervisor = Supervisor::new();
    let supervisor_tx = supervisor.sender();

    let data_folder = env::var("WORKDIR")?;
    let data_manager = Arc::new(RotatingPool::new(data_folder, supervisor_tx).await?);

    let (market_tx, _) = broadcast::channel::<Arc<common::models::MarketEvent>>(10_000);

    // --- Specialized Ingestion Actors ---
    regist_actor!(supervisor, market_tx, PublicStreamActor);

    // let tx_for_user = market_tx.clone();
    // supervisor.register_actor(
    //     ActorType::UserStreamActor,
    //     Box::new(move || Box::new(UserStreamActor::new(tx_for_user.clone()))),
    // );

    regist_actor!(supervisor, market_tx, FuturesStreamActor);

    // --- Centralized Storage Actor ---
    regist_actor!(supervisor, market_tx, data_manager, StorageActor);

    // AI/Strategy & Execution Setup
    // let (exec_tx, _) = broadcast::channel::<common::models::TradeSignal>(100);
    let (notify_tx, _) = broadcast::channel::<String>(100);

    // let model_path = env::var("MODEL_PATH").unwrap_or_else(|_| "models/strategy.onnx".to_string());

    // let exec_rx_for_svc = exec_tx.clone();
    // supervisor.register_actor(
    //     ActorType::ExecutionActor,
    //     Box::new(move || {
    //         Box::new(services::execution_service::ExecutionService::new(
    //             exec_rx_for_svc.subscribe(),
    //         ))
    //     }),
    // );

    // let market_rx_for_strat = market_tx.clone();
    // let exec_tx_for_strat = exec_tx.clone();
    // let notify_tx_for_strat = notify_tx.clone();
    // let model_path_strat = model_path.clone();

    // supervisor.register_actor(
    //     ActorType::StrategyActor,
    //     Box::new(move || {
    //         Box::new(
    //             strategy::services::strategy_service::StrategyService::new(
    //                 SYMBOLS,
    //                 100,
    //                 &model_path_strat,
    //                 market_rx_for_strat.subscribe(),
    //             )
    //             .with_notifier(notify_tx_for_strat.clone())
    //             .with_executor(exec_tx_for_strat.clone()),
    //         )
    //     }),
    // );

    let telegram_rx = notify_tx.subscribe();
    supervisor.register_actor(
        ActorType::Dynamic,
        Box::new(move || {
            Box::new(services::telegram_service::TelegramService::new(
                telegram_rx.resubscribe(),
            ))
        }),
    );

    supervisor.start().await;
    Ok(())
}
