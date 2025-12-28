use std::sync::Arc;

use tokio::sync::{broadcast, mpsc::Receiver};

use crate::{RoutingMessage, services::market_gateway::MarketEvent};

async fn router(
    mut receiver: Receiver<RoutingMessage>,
    sender: broadcast::Sender<Arc<MarketEvent>>,
) {
    while let Some(message) = receiver.recv().await {
        match message {
            RoutingMessage::MarketEvent(market_event) => {
                let _ = sender.send(Arc::new(market_event));
            }
            RoutingMessage::Heartbeat(actor_type) => todo!(),
            RoutingMessage::Reset(actor_type) => todo!(),
        }
    }
}
