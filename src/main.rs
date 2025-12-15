use dotenvy::dotenv;
use std::{env, sync::Arc};
use tracing::{debug, error, info};

use crate::db::RotatingPool;

mod aggtrade_recorder;
mod db;
mod legacy_remote;
mod logger;
mod models;
mod new_orders_buffer;
mod remote;
mod repositories;
mod services;

// "btcusdt", "ethusdt", "bnbusdt", "solusdt", "xrpusdt", "adausdt", "avaxusdt",

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

    let agg_svc = services::AggTradeService::new(SYMBOLS);
    let order_svc = services::OrderBookService::new(SYMBOLS);

    let agg_handle = tokio::spawn(agg_svc.start(rotating_pool.clone()));
    let order_handle = tokio::spawn(order_svc.start(rotating_pool));

    tokio::select! {
        _ = agg_handle => error!("AggTrade service stopped"),
        _ = order_handle => error!("OrderBook service stopped"),
    }
    Ok(())

    // let stream_params: Vec<String> = SYMBOLS
    //     .iter()
    //     .map(|s| format!("{}@depth20@1000ms", s))
    //     .collect();

    // let full_url = format!("{}{}", BASE_URL, stream_params.join("/"));

    // debug!("Connecting to Binance (Monthly Rotation Mode)...");
    // debug!("Targeting: {:?}", SYMBOLS);

    // let request = full_url.into_client_request().unwrap();
    // let (ws_stream, _) = connect_async(request).await.expect("Failed to connect");

    // info!(
    //     "Connected! Aggregating data for {} symbols.",
    //     SYMBOLS.iter().count()
    // );
    // let (_, mut read) = ws_stream.split();

    // let mut current_packed_month = 0u32;
    // let mut conn: Option<SqliteConnection> = None;

    // let max_buffer_size = SYMBOLS.len() * 10_usize;
    // let mut buffer = OrdersBookBuffer::new(max_buffer_size);

    // while let Some(message) = read.next().await {
    //     if let Ok(Message::Text(text)) = message {
    //         if let Ok(event) = serde_json::from_str::<CombinedEvent>(&text) {
    //             let this_packed_month = current_packed();

    //             if current_packed_month != this_packed_month {
    //                 if let Some(ref mut c) = conn {
    //                     buffer.flush(c);
    //                 }
    //                 let now = Utc::now();
    //                 info!(
    //                     "Switching database to month: {}",
    //                     format!("{}_{}", now.year(), now.month())
    //                 );
    //                 conn = Some(connect_to_monthly_db(&data_folder));
    //                 current_packed_month = this_packed_month;
    //             }

    //             let new_order = match event.to_insertable() {
    //                 Ok(v) => v,
    //                 Err(e) => {
    //                     error!("Serialization error: {}", e);
    //                     continue;
    //                 }
    //             };

    //             buffer.push(new_order);

    //             if buffer.len() >= max_buffer_size {
    //                 if let Some(ref mut c) = conn {
    //                     buffer.flush(c);
    //                 }
    //             }
    //         }
    //     }
    // }
    // Ok(())
}

// fn connect_to_monthly_db(base_paht: &str) -> SqliteConnection {
//     let current_db_path = format!("{}/sqlitedata/{}", &base_paht, CURRENT_DB_FOLDER);
//     fs::create_dir_all(&current_db_path).expect("Failed to create database directory");

//     let now = Utc::now();
//     let db_filename = format!(
//         "{}/crypto_{}_{:02}.db",
//         current_db_path,
//         now.year(),
//         now.month()
//     );
//     let path = Path::new(&db_filename);

//     let needs_initialization = !path.exists();

//     let database_url = format!("sqlite://{}", db_filename);
//     let mut conn = SqliteConnection::establish(&database_url)
//         .expect(&format!("Error connecting to {}", db_filename));

//     conn.batch_execute(
//         r"
//             PRAGMA journal_mode = WAL;
//             PRAGMA synchronous = NORMAL;
//             PRAGMA cache_size = -64000;
//             PRAGMA busy_timeout = 5000;
//         ",
//     )
//     .expect("Failed to apply PRAGMA optimizations");

//     if needs_initialization {
//         info!("New month detected!, Creating database: {}", db_filename);
//         let create_table_sql = r"
//                 CREATE TABLE IF NOT EXISTS order_books (
//                     id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
//                     time REAL NOT NULL,
//                     symbol TEXT NOT NULL,
//                     bids BLOB NOT NULL,
//                     asks BLOB NOT NULL
//                 );
//                 CREATE INDEX IF NOT EXISTS idx_time ON order_books(time);
//                 CREATE INDEX IF NOT EXISTS idx_symbol ON order_books(symbol);
//             ";

//         conn.batch_execute(create_table_sql)
//             .expect("Failed to initializate new database schema");

//         tokio::spawn(async { run_backup_script().await });
//     }

//     conn
// }

// async fn run_backup_script() {
//     let data_folder_env = env::var("WORKDIR").expect("WORKDIR must be set");
//     let data_folder = format!("{}/sqlitedata", data_folder_env);

//     let now = Utc::now();

//     let (prev_year, prev_month) = if now.month() == 1 {
//         (now.year() - 1, 12)
//     } else {
//         (now.year(), now.month() - 1)
//     };

//     let utils_path = env::var("UTILS").expect("UTILS must be set");

//     let result = Command::new(format!("{}/dump_db.sh", utils_path))
//         .arg(data_folder)
//         .arg(format!("crypto_{}_{:02}.db", prev_year, prev_month))
//         .output()
//         .await;

//     match result {
//         Ok(output) => {
//             if output.status.success() {
//                 info!("Script finished successfully!");
//                 let stdout = String::from_utf8_lossy(&output.stdout);
//                 info!("{}", stdout);
//             } else {
//                 error!("Script failed with error code: {:?}", output.status.code());
//                 let stderr = String::from_utf8_lossy(&output.stderr);
//                 error!("Error Log: {}", stderr);
//                 return;
//             }
//         }
//         Err(err) => {
//             error!("Failed to execute command: {}", err);
//             return;
//         }
//     }
// }
