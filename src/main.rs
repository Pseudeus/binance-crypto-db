use chrono::{Datelike, Utc};
use diesel::sqlite::SqliteConnection;
use diesel::{connection::SimpleConnection, prelude::*};
use dotenvy::dotenv;
use futures_util::StreamExt;
use std::{env, fs, path::Path};
use tokio_tungstenite::{
    connect_async,
    tungstenite::{Message, client::IntoClientRequest},
};

use crate::new_orders_buffer::OrdersBookBuffer;
use crate::remote::{BASE_URL, CombinedEvent};

mod models;
mod new_orders_buffer;
mod remote;
mod schema;

const CURRENT_DB_FOLDER: &str = "current";
const ARCHIVED_DB_FOLDER: &str = "archived";

const API_AWAIT_TIME_MILIS: usize = 1_000;
const SYMBOLS: [&str; 7] = [
    "btcusdt", "ethusdt", "bnbusdt", "solusdt", "xrpusdt", "adausdt", "avaxusdt",
];

#[tokio::main]
async fn main() {
    dotenv().ok();
    let data_folder = env::var("WORKDIR").expect("WORKDIR must be set");

    let stream_params: Vec<String> = SYMBOLS
        .iter()
        .map(|s| format!("{}@depth20@{}ms", s, API_AWAIT_TIME_MILIS))
        .collect();

    let full_url = format!("{}{}", BASE_URL, stream_params.join("/"));

    println!("Connecting to Binance (Monthly Rotation Mode)...");
    println!("Targeting: {:?}", SYMBOLS);

    let request = full_url.into_client_request().unwrap();
    let (ws_stream, _) = connect_async(request).await.expect("Failed to connect");

    println!(
        "Connected! Aggregating data for {} symbols.",
        SYMBOLS.iter().count()
    );
    let (_, mut read) = ws_stream.split();

    let mut current_month_str = String::new();
    let mut conn: Option<SqliteConnection> = None;

    let max_buffer_size = SYMBOLS.len() * (API_AWAIT_TIME_MILIS / 100_usize);
    let mut buffer = OrdersBookBuffer::new(max_buffer_size);

    while let Some(message) = read.next().await {
        if let Ok(Message::Text(text)) = message {
            if let Ok(event) = serde_json::from_str::<CombinedEvent>(&text) {
                let now = Utc::now();
                let this_month_str = format!("{}_{}", now.year(), now.month());

                if current_month_str != this_month_str {
                    if let Some(ref mut c) = conn {
                        buffer.flush(c);
                    }

                    println!("Switching database to month: {}", this_month_str);
                    conn = Some(connect_to_monthly_db(&data_folder));
                    current_month_str = this_month_str;
                }

                let new_order = match event.to_insertable() {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("Serialization error: {}", e);
                        continue;
                    }
                };

                buffer.push(new_order);

                if buffer.len() >= max_buffer_size {
                    if let Some(ref mut c) = conn {
                        buffer.flush(c);
                    }
                }
            }
        }
    }
}

fn connect_to_monthly_db(base_paht: &str) -> SqliteConnection {
    let current_db_path = format!("{}/sqlitedata/{}", base_paht, CURRENT_DB_FOLDER);
    fs::create_dir_all(&current_db_path).expect("Failed to create database directory");

    let now = Utc::now();
    let db_filename = format!(
        "{}/crypto_{}_{:02}.db",
        current_db_path,
        now.year(),
        now.month()
    );
    let path = Path::new(&db_filename);

    let needs_initialization = !path.exists();

    let database_url = format!("sqlite://{}", db_filename);
    let mut conn = SqliteConnection::establish(&database_url)
        .expect(&format!("Error connecting to {}", db_filename));

    conn.batch_execute(
        r"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            PRAGMA cache_size = -64000;
            PRAGMA busy_timeout = 5000;
        ",
    )
    .expect("Failed to apply PRAGMA optimizations");

    if needs_initialization {
        println!("New month detected!, Creating database: {}", db_filename);
        let create_table_sql = r"
                CREATE TABLE IF NOT EXISTS order_books (
                    id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                    time REAL NOT NULL,
                    symbol TEXT NOT NULL,
                    bids BLOB NOT NULL,
                    asks BLOB NOT NULL
                );
                CREATE INDEX IF NOT EXISTS idx_time ON order_books(time);
                CREATE INDEX IF NOT EXISTS idx_symbol ON order_books(symbol);
            ";

        conn.batch_execute(create_table_sql)
            .expect("Failed to initializate new database schema");
    }

    conn
}
