use crate::models::Quantity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSignal {
    pub symbol: String,
    pub side: String, // "BUY" or "SELL"
    pub quantity: Quantity,
    pub reason: String, // "AI_CONFIDENCE_0.85"
}
