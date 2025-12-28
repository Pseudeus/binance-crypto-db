use chrono::{Datelike, Utc};
use sqlx::sqlite::{self, SqliteConnectOptions, SqlitePool};
use std::env;
use std::str::FromStr;
use std::time::Duration;
use tokio::process::Command;
use tokio::sync::RwLock;
use tracing::{error, info};

pub struct RotatingPool {
    data_folder: String,
    inner: RwLock<(u32, SqlitePool)>,
}

impl RotatingPool {
    pub async fn new(data_folder: String) -> Result<Self, sqlx::Error> {
        let pool = get_weekly_pool(&data_folder).await?;
        let packed = Self::current_packed();
        Ok(Self {
            data_folder,
            inner: RwLock::new((packed, pool)),
        })
    }

    fn current_packed() -> u32 {
        let now = Utc::now().iso_week();
        (now.year() as u32) << 6 | (now.week() & 0x3f)
    }

    pub async fn get(&self) -> Result<SqlitePool, sqlx::Error> {
        let read = self.inner.read().await;
        let (current_packed, ref pool) = *read;

        if current_packed == Self::current_packed() {
            return Ok(pool.clone());
        }
        drop(read);

        let mut write = self.inner.write().await;
        let (current_packed, _) = *write;

        if current_packed != Self::current_packed() {
            let new_pool = get_weekly_pool(&self.data_folder).await?;
            *write = (Self::current_packed(), new_pool);
            tokio::spawn(async { run_backup_script().await });
        }
        Ok(write.1.clone())
    }
}

async fn get_weekly_pool(data_folder: &str) -> Result<SqlitePool, sqlx::Error> {
    let current_db_path = format!("{}/sqlitedata/current", data_folder);
    std::fs::create_dir_all(&current_db_path)?;

    let now = Utc::now().iso_week();
    let db_filename = format!(
        "{}/crypto_{}_{:02}.db",
        current_db_path,
        now.year(),
        now.week()
    );

    let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_filename))?
        .create_if_missing(true)
        .journal_mode(sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlite::SqliteSynchronous::Normal)
        .busy_timeout(Duration::from_secs(30))
        .statement_cache_capacity(100)
        .auto_vacuum(sqlite::SqliteAutoVacuum::Incremental)
        .analysis_limit(Some(400))
        .command_buffer_size(5000);

    let pool = SqlitePool::connect_with(options).await?;

    sqlx::query(
        r#"
            CREATE TABLE IF NOT EXISTS order_books(
                id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                time REAL NOT NULL,
                symbol TEXT NOT NULL,
                bids BLOB NOT NULL,
                asks BLOB NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_time ON order_books(time);
            CREATE INDEX IF NOT EXISTS idx_symbol_time ON order_books(symbol, time);

            CREATE TABLE IF NOT EXISTS agg_trades(
                id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                time REAL NOT NULL,
                symbol TEXT NOT NULL,
                price REAL NOT NULL,
                quantity REAL NOT NULL,
                is_buyer_maker BOOLEAN NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_agg_symbol_time ON agg_trades(symbol, time);

            CREATE TABLE IF NOT EXISTS klines(
                id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                symbol TEXT NOT NULL,
                interval TEXT NOT NULL,
                start_time INTEGER NOT NULL,
                close_time INTEGER NOT NULL,
                open_price REAL NOT NULL,
                close_price REAL NOT NULL,
                high_price REAL NOT NULL,
                low_price REAL NOT NULL,
                volume REAL NOT NULL,
                no_of_trades INTEGER NOT NULL,
                taker_buy_vol REAL NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_klines_symbol_interval_starttime ON klines(symbol, interval, start_time);
        "#,
    )
    .execute(&pool)
    .await?;
    Ok(pool)
}

async fn run_backup_script() {
    let data_folder_env = env::var("WORKDIR").expect("WORKDIR must be set");
    let data_folder = format!("{}/sqlitedata", data_folder_env);

    let now = Utc::now().iso_week();

    let (prev_year, prev_week) = if now.week() == 1 {
        (now.year() - 1, 53)
    } else {
        (now.year(), now.week() - 1)
    };

    let utils_path = env::var("UTILS").expect("UTILS must be set");

    let result = Command::new(format!("{}/dump_db.sh", utils_path))
        .arg(data_folder)
        .arg(format!("crypto_{}_{:02}.db", prev_year, prev_week))
        .output()
        .await;

    match result {
        Ok(output) => {
            if output.status.success() {
                info!("Script finished successfully!");
                let stdout = String::from_utf8_lossy(&output.stdout);
                info!("{}", stdout);
            } else {
                error!("Script failed with error code: {:?}", output.status.code());
                let stderr = String::from_utf8_lossy(&output.stderr);
                error!("Error Log: {}", stderr);
                return;
            }
        }
        Err(err) => {
            error!("Failed to execute command: {}", err);
            return;
        }
    }
}
