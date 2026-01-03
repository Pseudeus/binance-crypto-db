use chrono::{DateTime, Datelike, Duration, Utc};
use common::actors::ControlMessage;
use sqlx::sqlite::{self, SqliteConnectOptions, SqlitePool};
use std::env;
use std::str::FromStr;
use std::time::Duration as StdDuration;
use tokio::sync::{RwLock, mpsc};
use tracing::{error, info};

use crate::actors::backup_actor::BackupOneShotActor;

pub struct RotatingPool {
    data_folder: String,
    inner: RwLock<(u32, SqlitePool)>,
    supervisor_tx: mpsc::Sender<ControlMessage>,
}

impl RotatingPool {
    pub async fn new(
        data_folder: String,
        supervisor_tx: mpsc::Sender<ControlMessage>,
    ) -> Result<Self, sqlx::Error> {
        let pool = get_weekly_pool(&data_folder).await?;
        let packed = Self::current_packed();
        Ok(Self {
            data_folder,
            inner: RwLock::new((packed, pool)),
            supervisor_tx,
        })
    }

    fn current_packed() -> u32 {
        let (year, week) = get_date_components(Utc::now());
        (year as u32) << 6 | (week & 0x3f)
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

            // Spawn the backup actor via the Supervisor
            let backup_actor = Box::new(BackupOneShotActor::new());
            let spawn_msg = ControlMessage::Spawn(backup_actor);

            if let Err(e) = self.supervisor_tx.send(spawn_msg).await {
                error!("Failed to request Backup Actor spawn: {}", e);
            } else {
                info!("Requested Backup Actor spawn via Supervisor");
            }
        }
        Ok(write.1.clone())
    }
}

async fn get_weekly_pool(data_folder: &str) -> Result<SqlitePool, sqlx::Error> {
    let current_db_path = format!("{}/sqlitedata/current", data_folder);
    std::fs::create_dir_all(&current_db_path)?;

    let (year, week) = get_date_components(Utc::now());
    let db_filename = format!("{}/crypto_{}_{:02}.db", current_db_path, year, week);

    let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_filename))?
        .create_if_missing(true)
        .journal_mode(sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlite::SqliteSynchronous::Normal)
        .busy_timeout(StdDuration::from_secs(30))
        .statement_cache_capacity(100)
        .auto_vacuum(sqlite::SqliteAutoVacuum::Incremental)
        .analysis_limit(Some(400))
        .command_buffer_size(5000);

    let pool = SqlitePool::connect_with(options).await?;

    let schema = include_str!("../../../sql/schema.sql");

    sqlx::query(schema).execute(&pool).await?;
    Ok(pool)
}

pub fn get_date_components(date: DateTime<Utc>) -> (i32, u32) {
    let iso = date.iso_week();
    (iso.year(), iso.week())
}

/// Calculates the ISO year and week of the week prior to the given date.
/// Uses time subtraction to correctly handle 52/53 week years.
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
        // Simulate being in ISO Week 1 of 2026 (e.g., Dec 29, 2025)
        // Dec 29, 2025 is Monday. 12:00:00 UTC.
        let dt = Utc.with_ymd_and_hms(2025, 12, 29, 12, 0, 0).unwrap();

        // Verify current is Week 1
        let (cur_year, cur_week) = get_date_components(dt);
        assert_eq!(cur_year, 2026);
        assert_eq!(cur_week, 1);

        // Calculate previous week
        let (prev_year, prev_week) = get_previous_iso_week_components(dt);

        // EXPECTED CORRECT BEHAVIOR:
        // 1 week before Dec 29 is Dec 22.
        // Dec 22, 2025 is in 2025-W52.
        assert_eq!(prev_year, 2025, "Expected previous year to be 2025");
        assert_eq!(prev_week, 52, "Expected previous week to be 52");
    }
}
