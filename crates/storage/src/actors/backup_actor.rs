use anyhow::bail;
use async_trait::async_trait;
use chrono::Utc;
use common::actors::{Actor, ActorType, ControlMessage};
use std::env;
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

use crate::{actors::BackupScriptError, db::get_previous_iso_week_components};

pub struct BackupOneShotActor {
    id: Uuid,
}

#[async_trait]
impl Actor for BackupOneShotActor {
    fn id(&self) -> Uuid {
        self.id
    }
    fn name(&self) -> ActorType {
        ActorType::Dynamic
    }

    async fn run(&mut self, supervisor_tx: mpsc::Sender<ControlMessage>) -> anyhow::Result<()> {
        let hearbeat_handle = self.spawn_heartbeat(supervisor_tx.clone());

        let data_folder_env = env::var("WORKDIR").expect("WORKDIR must be set");
        let data_folder = format!("{}/sqlitedata", data_folder_env);

        let (prev_year, prev_week) = get_previous_iso_week_components(Utc::now());

        let utils_path = env::var("UTILS").expect("UTILS must be set");

        let result = Command::new(format!("{}/dump_db.sh", utils_path))
            .arg(data_folder)
            .arg(format!("crypto_{}_{:02}.db", prev_year, prev_week))
            .output()
            .await;

        match result {
            Ok(output) => {
                if output.status.success() {
                    info!("Backup finished successfully!");
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    info!("{}", stdout);
                } else {
                    let code = output.status.code().unwrap_or(-1);

                    let error_enum = BackupScriptError::from(code);
                    let stderr = String::from_utf8_lossy(&output.stderr);

                    error!("Backup failed: {}", error_enum);
                    error!("Script Stderr: {}", stderr);
                    hearbeat_handle.abort();
                    bail!(error_enum);
                }
            }
            Err(err) => {
                bail!("Failed to execute command: {}", err);
            }
        }

        if supervisor_tx
            .send(ControlMessage::Shutdown(self.id))
            .await
            .is_err()
        {
            hearbeat_handle.abort();
        };
        Ok(())
    }
}

impl BackupOneShotActor {
    pub fn new() -> Self {
        Self { id: Uuid::new_v4() }
    }
}
