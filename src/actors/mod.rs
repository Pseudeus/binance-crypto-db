use async_trait::async_trait;
use tokio::sync::mpsc;

pub mod supervisor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ActorType {
    AggTradeActor,
    KlinesActor,
    OrderBookActor,
    GatewayActor,
}

/// Messages sent from Actors to the Supervisor
#[derive(Debug)]
pub enum ControlMessage {
    Heartbeat(ActorType),
    Shutdown(ActorType),
    Error(ActorType, String), // ActorName, ErrorMessage
}

/// The trait that all restartable services must implement
#[async_trait]
pub trait Actor: Send + Sync {
    /// The unique name of the actor (e.g., "AggTrade")
    fn name(&self) -> ActorType;

    /// The main loop of the actor.
    /// It must periodically send `ControlMessage::Heartbeat` to the supervisor.
    async fn run(&mut self, supervisor_tx: mpsc::Sender<ControlMessage>) -> anyhow::Result<()>;
}
