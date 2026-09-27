use chrono::{DateTime, Datelike, Duration, Utc};
use common::actors::ControlMessage;
use common::models::{
    AggTradeInsert, ForceOrderInsert, KlineInsert, MarkPriceInsert, OpenInterestInsert,
    OrderBookInsert,
};
use tokio::sync::mpsc;
use tracing::{error, info};

use crate::actors::backup_actor::BackupOneShotActor;
use crate::repositories::{
    AggTradeRepository, ForceOrderRepository, KlineRepository, MarkPriceRepository,
    OpenInterestRepository, OrderBookRepository, Repository,
};

#[derive(Debug)]
pub enum DbWriteBatch {
    AggTrade(Vec<AggTradeInsert>),
    OrderBook(Vec<OrderBookInsert>),
    Kline(Vec<KlineInsert>),
    MarkPrice(Vec<MarkPriceInsert>),
    OpenInterest(Vec<OpenInterestInsert>),
    ForceOrder(Vec<ForceOrderInsert>),
}

#[derive(Clone)]
pub struct DbWriterHandle {
    tx: mpsc::Sender<DbWriteBatch>,
}

impl DbWriterHandle {
    pub fn spawn(
        data_folder: String,
        supervisor_tx: mpsc::Sender<ControlMessage>,
    ) -> std::io::Result<Self> {
        let (tx, rx) = mpsc::channel(256);
        std::thread::Builder::new()
            .name("db-writer".to_string())
            .spawn(move || {
                run_db_writer(data_folder, supervisor_tx, rx);
            })?;
        Ok(Self { tx })
    }

    pub fn sender(&self) -> mpsc::Sender<DbWriteBatch> {
        self.tx.clone()
    }
}

pub fn run_db_writer(
    data_folder: String,
    supervisor_tx: mpsc::Sender<ControlMessage>,
    mut rx: mpsc::Receiver<DbWriteBatch>,
) {
    info!("Starting dedicated DbWriter worker thread...");
    let (mut conn, mut active_packed) = match open_connection(&data_folder) {
        Ok(c) => c,
        Err(e) => {
            error!("Fatal error initializing SQLite connection: {}", e);
            return;
        }
    };

    while let Some(batch) = rx.blocking_recv() {
        let current_packed = current_packed();
        if current_packed != active_packed {
            info!("Rotating database file to new ISO week...");
            drop(conn);
            match open_connection(&data_folder) {
                Ok((new_conn, new_packed)) => {
                    conn = new_conn;
                    active_packed = new_packed;

                    let backup_actor = Box::new(BackupOneShotActor::new());
                    if let Err(e) = supervisor_tx.try_send(ControlMessage::Spawn(backup_actor)) {
                        error!("Failed to request Backup Actor spawn: {}", e);
                    } else {
                        info!("Requested Backup Actor spawn via Supervisor");
                    }
                }
                Err(e) => {
                    error!("Failed to rotate database connection: {}", e);
                    return;
                }
            }
        }

        if let Err(e) = write_batch(&mut conn, batch) {
            error!("Failed to write batch to SQLite: {}", e);
        }
    }
    info!("DbWriter worker channel closed, worker thread exiting cleanly.");
}

pub fn open_connection(data_folder: &str) -> anyhow::Result<(rusqlite::Connection, u32)> {
    let current_db_path = format!("{}/sqlitedata/current", data_folder);
    std::fs::create_dir_all(&current_db_path)?;

    let (year, week) = get_date_components(Utc::now());
    let db_filename = format!("{}/crypto_{}_{:02}.db", current_db_path, year, week);

    let conn = rusqlite::Connection::open(&db_filename)?;

    // Bare-metal PRAGMA optimizations for high-throughput write performance
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA synchronous = OFF;
        PRAGMA cache_size = -64000;
        PRAGMA mmap_size = 1073741824;
        PRAGMA temp_store = MEMORY;
        PRAGMA locking_mode = EXCLUSIVE;
        PRAGMA wal_autocheckpoint = 10000;
        "#,
    )?;

    let schema = include_str!("../migrations/schema.sql");
    conn.execute_batch(schema)?;

    let packed = pack_year_week(year, week);
    Ok((conn, packed))
}

fn write_batch(conn: &mut rusqlite::Connection, batch: DbWriteBatch) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    match batch {
        DbWriteBatch::AggTrade(trades) => {
            AggTradeRepository::insert_batch(&tx, &trades)?;
        }
        DbWriteBatch::OrderBook(books) => {
            OrderBookRepository::insert_batch(&tx, &books)?;
        }
        DbWriteBatch::Kline(klines) => {
            KlineRepository::insert_batch(&tx, &klines)?;
        }
        DbWriteBatch::MarkPrice(prices) => {
            MarkPriceRepository::insert_batch(&tx, &prices)?;
        }
        DbWriteBatch::OpenInterest(interests) => {
            OpenInterestRepository::insert_batch(&tx, &interests)?;
        }
        DbWriteBatch::ForceOrder(orders) => {
            ForceOrderRepository::insert_batch(&tx, &orders)?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn pack_year_week(year: i32, week: u32) -> u32 {
    (year as u32) << 6 | (week & 0x3f)
}

pub fn current_packed() -> u32 {
    let (year, week) = get_date_components(Utc::now());
    pack_year_week(year, week)
}

pub fn get_date_components(date: DateTime<Utc>) -> (i32, u32) {
    let iso = date.iso_week();
    (iso.year(), iso.week())
}

pub fn get_previous_iso_week_components(date: DateTime<Utc>) -> (i32, u32) {
    let prev = date - Duration::weeks(1);
    get_date_components(prev)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_dec_29_2025_handling() {
        let dt = Utc.with_ymd_and_hms(2025, 12, 29, 12, 0, 0).unwrap();
        let (year, week) = get_date_components(dt);

        assert_eq!(year, 2026, "Expected ISO year for Dec 29, 2025 to be 2026");
        assert_eq!(week, 1, "Expected ISO week for Dec 29, 2025 to be 1");
    }

    #[test]
    fn test_previous_week_calculation_fix() {
        let dt = Utc.with_ymd_and_hms(2025, 12, 29, 12, 0, 0).unwrap();
        let (cur_year, cur_week) = get_date_components(dt);
        assert_eq!(cur_year, 2026);
        assert_eq!(cur_week, 1);

        let (prev_year, prev_week) = get_previous_iso_week_components(dt);
        assert_eq!(prev_year, 2025, "Expected previous year to be 2025");
        assert_eq!(prev_week, 52, "Expected previous week to be 52");
    }

    #[test]
    fn test_open_connection_pragmas_and_schema() {
        let temp_dir = std::env::temp_dir().join(format!("rusqlite_test_{}", std::process::id()));
        let temp_path = temp_dir.to_str().unwrap();

        let (conn, packed) = open_connection(temp_path).expect("Failed to open connection");
        assert!(packed > 0);

        let journal_mode: String = conn
            .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
            .expect("Failed to query journal_mode");
        assert_eq!(journal_mode.to_lowercase(), "wal");

        let sync_mode: i32 = conn
            .query_row("PRAGMA synchronous;", [], |row| row.get(0))
            .expect("Failed to query synchronous");
        assert_eq!(sync_mode, 0); // 0 = OFF

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
