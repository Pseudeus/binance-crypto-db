use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeSignal {
    pub symbol: String,
    pub side: String, // "BUY" or "SELL"
    pub quantity: f64,
    pub reason: String, // "AI_CONFIDENCE_0.85"
}
