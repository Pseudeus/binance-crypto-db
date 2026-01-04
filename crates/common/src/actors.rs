use std::time::Duration;

use async_trait::async_trait;
use tokio::{sync::mpsc, task::JoinHandle};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActorType {
    AggTradeActor,
    KlinesActor,
    OrderBookActor,
    GatewayActor,
    MarkPriceActor,
    ForceOrderActor,
    Dynamic,
}

/// Messages sent from Actors to the Supervisor
pub enum ControlMessage {
    Spawn(Box<dyn Actor + Send + Sync>),
    Heartbeat(Uuid),
    Shutdown(Uuid),
    Error(Uuid, String),
}

impl std::fmt::Debug for ControlMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(_) => write!(f, "Spawn(Box<dyn Actor>)"),
            Self::Heartbeat(actor_type) => write!(f, "Heartbeat({:?})", actor_type),
            Self::Shutdown(actor_type) => write!(f, "Shutdown({:?})", actor_type),
            Self::Error(actor_type, err) => write!(f, "Error({:?}, {})", actor_type, err),
        }
    }
}

/// The trait that all restartable services must implement
#[async_trait]
pub trait Actor: Send + Sync {
    /// The unique name of the actor (e.g., "AggTrade")
    fn name(&self) -> ActorType;

    fn id(&self) -> Uuid;

    /// The main loop of the actor.
    /// It must periodically send `ControlMessage::Heartbeat` to the supervisor.
    async fn run(&mut self, supervisor_tx: mpsc::Sender<ControlMessage>) -> anyhow::Result<()>;

    fn spawn_heartbeat(&self, supervisor_tx: mpsc::Sender<ControlMessage>) -> JoinHandle<()> {
        let id = self.id();
        tokio::spawn(async move {
            loop {
                if supervisor_tx
                    .send(ControlMessage::Heartbeat(id))
                    .await
                    .is_err()
                {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        })
    }
}
