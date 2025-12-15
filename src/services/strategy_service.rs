use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};
use ta::indicators::{RelativeStrengthIndex, BollingerBands};
use ta::Next;
use crate::models::{AggTradeInsert, OrderBookInsert};
use crate::inference::InferenceEngine;

struct SymbolState {
    rsi: RelativeStrengthIndex,
    bb: BollingerBands,
    order_book_imbalance: f64,
}

impl SymbolState {
    fn new() -> Self {
        Self {
            // Standard RSI(14)
            rsi: RelativeStrengthIndex::new(14).unwrap(),
            // Standard BB(20, 2.0)
            bb: BollingerBands::new(20, 2.0).unwrap(),
            order_book_imbalance: 0.0,
        }
    }
}

pub struct StrategyService {
    // Map symbol (lowercase) -> State
    states: HashMap<String, SymbolState>,
    window_size: usize,
    engine: InferenceEngine,
}

impl StrategyService {
    pub fn new(symbols: &[&str], window_size: usize, model_path: &str) -> Self {
        let mut states = HashMap::new();
        for s in symbols {
            states.insert(s.to_lowercase(), SymbolState::new());
        }

        // Initialize AI Inference Engine
        let engine = InferenceEngine::new(model_path);

        Self {
            states,
            window_size,
            engine,
        }
    }

    pub async fn start(
        mut self, 
        mut trade_rx: broadcast::Receiver<Arc<AggTradeInsert>>,
        mut order_rx: broadcast::Receiver<Arc<OrderBookInsert>>
    ) {
        info!("Starting Strategy Engine for {} symbols", self.states.len());
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

        loop {
            tokio::select! {
                trade_res = trade_rx.recv() => {
                    match trade_res {
                        Ok(trade) => self.process_tick(&trade),
                        Err(broadcast::error::RecvError::Lagged(n)) => warn!("Strategy trade lag: {}", n),
                        Err(_) => break,
                    }
                }
                order_res = order_rx.recv() => {
                    match order_res {
                        Ok(order) => self.process_orderbook(&order),
                        Err(broadcast::error::RecvError::Lagged(n)) => warn!("Strategy order lag: {}", n),
                        Err(_) => break,
                    }
                }
                _ = interval.tick() => {
                    self.log_status();
                }
            }
        }
        info!("Strategy Engine stopped.");
    }

    fn log_status(&self) {
        // Log a brief summary for a few key symbols to prove liveness
        let keys = ["btcusdt", "ethusdt", "solusdt", "dogeusdt"];
        let mut summary = String::from("STATUS: ");
        
        for k in keys {
            if let Some(state) = self.states.get(k) {
                // Peek at current RSI (hacky access or just use last value if stored? 
                // TA crates don't always expose 'current' easily without 'next', 
                // but we can trust the OBI is fresh).
                // Actually, `ta` crate doesn't let us peek easily without modifying state.
                // We will just log OBI which is stored in our struct.
                summary.push_str(&format!("[{}: OBI={:.2}] ", k.to_uppercase(), state.order_book_imbalance));
            }
        }
        info!("{}", summary);
    }

    fn process_tick(&mut self, trade: &AggTradeInsert) {
        let symbol = trade.symbol.to_lowercase();
        
        if let Some(state) = self.states.get_mut(&symbol) {
             let price = trade.price;

            // Calculate Indicators
            let rsi_val = state.rsi.next(price);
            let _bb_val = state.bb.next(price);
            let obi = state.order_book_imbalance;

            // AI Inference
            // Feature Vector: [RSI, OBI]
            let features = vec![rsi_val as f32, obi as f32];
            match self.engine.predict(&features) {
                Ok(prob) => {
                    // Only log high conviction AI signals to reduce noise
                    if prob > 0.8 {
                        info!("AI STRONG BUY ({:.2}) for {}: Price={:.2}", prob, symbol, price);
                    } else if prob < 0.2 {
                        info!("AI STRONG SELL ({:.2}) for {}: Price={:.2}", prob, symbol, price);
                    }
                }
                Err(e) => warn!("AI Inference Error: {}", e),
            }

            // Legacy Rule-Based Signal Logging (for comparison)
            if rsi_val > 70.0 && obi < -0.2 {
                debug!("RULE: {} RSI Overbought ({:.2}) AND Selling Pressure (OBI {:.2}). Potential SHORT at {:.2}", symbol, rsi_val, obi, price);
            } else if rsi_val < 30.0 && obi > 0.2 {
                debug!("RULE: {} RSI Oversold ({:.2}) AND Buying Pressure (OBI {:.2}). Potential LONG at {:.2}", symbol, rsi_val, obi, price);
            }
        }
    }

    fn process_orderbook(&mut self, order: &OrderBookInsert) {
        let symbol = order.symbol.to_lowercase();
        if let Some(state) = self.states.get_mut(&symbol) {
            let bid_vol = Self::calculate_volume(&order.bids);
            let ask_vol = Self::calculate_volume(&order.asks);

            // OBI Formula: (Bid - Ask) / (Bid + Ask)
            let total = bid_vol + ask_vol;
            if total > 0.0 {
                state.order_book_imbalance = (bid_vol - ask_vol) / total;
            }
        }
    }

    fn calculate_volume(data: &[u8]) -> f64 {
        // Data is packed as [Price(f32), Qty(f32)] in little endian
        let mut total_vol = 0.0;
        for chunk in data.chunks_exact(8) {
            // We care about Quantity, which is the 2nd f32 (bytes 4..8)
            let qty_bytes: [u8; 4] = chunk[4..8].try_into().unwrap_or([0; 4]);
            let qty = f32::from_le_bytes(qty_bytes) as f64;
            total_vol += qty;
        }
        total_vol
    }
}
