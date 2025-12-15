use chrono::{Datelike, Utc};
use sqlx::sqlite::{self, SqliteConnectOptions, SqlitePool};
use std::str::FromStr;
use std::time::Duration;
use tokio::sync::RwLock;

pub struct RotatingPool {
    data_folder: String,
    inner: RwLock<(u32, SqlitePool)>,
}

impl RotatingPool {
    pub async fn new(data_folder: String) -> Result<Self, sqlx::Error> {
        let pool = get_monthly_pool(&data_folder).await?;
        let packed = Self::current_packed();
        Ok(Self {
            data_folder,
            inner: RwLock::new((packed, pool)),
        })
    }

    fn current_packed() -> u32 {
        let now = Utc::now();
        (now.year() as u32) << 4 | (now.month() & 0x0f)
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
            let new_pool = get_monthly_pool(&self.data_folder).await?;
            *write = (Self::current_packed(), new_pool);
        }
        Ok(write.1.clone())
    }
}

async fn get_monthly_pool(data_folder: &str) -> Result<SqlitePool, sqlx::Error> {
    let current_db_path = format!("{}/sqlitedata/current", data_folder);
    std::fs::create_dir_all(&current_db_path)?;

    let now = Utc::now();
    let db_filename = format!(
        "{}/crypto_{}_{:02}.db",
        current_db_path,
        now.year(),
        now.month()
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
            CREATE INDEX IF NOT EXISTS idx_symbol ON order_books(symbol);

            CREATE TABLE IF NOT EXISTS agg_trades(
                id INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
                time REAL NOT NULL,
                symbol TEXT NOT NULL,
                agg_trade_id INTEGER NOT NULL,
                price REAL NOT NULL,
                quantity REAL NOT NULL,
                first_trade_id INTEGER NOT NULL,
                last_trade_id INTEGER NOT NULL,
                is_buyer_maker BOOLEAN NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_agg_time ON agg_trades(time);
            CREATE INDEX IF NOT EXISTS idx_agg_symbol_time ON agg_trades(symbol, time);
        "#,
    )
    .execute(&pool)
    .await?;
    Ok(pool)
}
