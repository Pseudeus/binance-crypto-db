use crate::inference::{InferenceEngine, InferenceResult};
use common::models::{AggTradeInsert, OrderBookInsert, TradeSignal};
use std::collections::HashMap;
use std::sync::Arc;
use ta::Next;
use ta::indicators::{
    BollingerBands, ExponentialMovingAverage, RelativeStrengthIndex, StandardDeviation,
};
use tokio::sync::broadcast;
use tracing::{debug, error, info, warn};

struct SymbolState {
    rsi: RelativeStrengthIndex,
    bb: BollingerBands,
    std_dev: StandardDeviation,
    buy_vol_ema: ExponentialMovingAverage,
    sell_vol_ema: ExponentialMovingAverage,
    order_book_imbalance: f64,
    has_position: bool,
}

impl SymbolState {
    fn new() -> Self {
        Self {
            // Standard RSI(14)
            rsi: RelativeStrengthIndex::new(14).unwrap(),
            // Standard BB(20, 2.0)
            bb: BollingerBands::new(20, 2.0).unwrap(),
            // Standard Deviation (20) - matching BB length
            std_dev: StandardDeviation::new(20).unwrap(),
            // Volume EMAs (Smoothing factor)
            buy_vol_ema: ExponentialMovingAverage::new(100).unwrap(),
            sell_vol_ema: ExponentialMovingAverage::new(100).unwrap(),
            order_book_imbalance: 0.0,
            has_position: false,
        }
    }
}

pub struct StrategyService {
    // Map symbol (lowercase) -> State
    states: HashMap<String, SymbolState>,
    engine: InferenceEngine,
    notification_tx: Option<broadcast::Sender<String>>,
    execution_tx: Option<broadcast::Sender<TradeSignal>>,
}

impl StrategyService {
    pub fn new(symbols: &[&str], _window_size: usize, model_path: &str) -> Self {
        let mut states = HashMap::new();
        for s in symbols {
            states.insert(s.to_lowercase(), SymbolState::new());
        }

        // Initialize AI Inference Engine
        let engine = InferenceEngine::new(model_path);

        Self {
            states,
            engine,
            notification_tx: None,
            execution_tx: None,
        }
    }

    pub fn with_notifier(mut self, tx: broadcast::Sender<String>) -> Self {
        self.notification_tx = Some(tx);
        self
    }

    pub fn with_executor(mut self, tx: broadcast::Sender<TradeSignal>) -> Self {
        self.execution_tx = Some(tx);
        self
    }

    pub async fn start(
        mut self,
        mut trade_rx: broadcast::Receiver<Arc<AggTradeInsert>>,
        mut order_rx: broadcast::Receiver<Arc<OrderBookInsert>>,
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
                summary.push_str(&format!(
                    "[{}: OBI={:.2}] ",
                    k.to_uppercase(),
                    state.order_book_imbalance
                ));
            }
        }
        info!("{}", summary);
    }

    fn process_tick(&mut self, trade: &AggTradeInsert) {
        let symbol = trade.symbol.to_lowercase();
        let price = trade.price;
        let quantity = trade.quantity;

        let mut pending_action = None;

        if let Some(state) = self.states.get_mut(&symbol) {
            // 1. RSI & Volatility
            let rsi_val = state.rsi.next(price);
            let _bb_val = state.bb.next(price);
            let vol_val = state.std_dev.next(price);
            let obi = state.order_book_imbalance;

            // 2. Volume Imbalance (TFI)
            // is_buyer_maker = true -> Sell, false -> Buy
            let (buy_q, sell_q) = if trade.is_buyer_maker {
                (0.0, quantity)
            } else {
                (quantity, 0.0)
            };

            let buy_ema = state.buy_vol_ema.next(buy_q);
            let sell_ema = state.sell_vol_ema.next(sell_q);

            let total_vol = buy_ema + sell_ema;
            let tfi = if total_vol > 0.0 {
                (buy_ema - sell_ema) / total_vol
            } else {
                0.0
            };

            // AI Inference
            // Feature Vector: [RSI, OBI, TFI, Volatility]
            let features = vec![rsi_val as f32, obi as f32, tfi as f32, vol_val as f32];
            match self.engine.predict(&features) {
                Ok(result) => {
                    let InferenceResult { class, confidence } = result;

                    // Log every prediction for visibility during testing
                    info!(
                        "AI Prediction for {}: Class={} Conf={:.4} (RSI={:.1} OBI={:.2} TFI={:.2} Vol={:.2})",
                        symbol, class, confidence, rsi_val, obi, tfi, vol_val
                    );

                    // Threshold for action
                    let threshold = 0.60; // Lowered slightly as multi-class is harder

                    if confidence > threshold {
                        match class {
                            1 => { // BUY
                                if !state.has_position {
                                    state.has_position = true;
                                    pending_action = Some(("BUY", confidence));
                                }
                            }
                            2 => { // SELL
                                if state.has_position {
                                    state.has_position = false;
                                    pending_action = Some(("SELL", confidence));
                                }
                            }
                            _ => {} // HOLD
                        }
                    }
                }
                Err(e) => warn!("AI Inference Error: {}", e),
            }

            // Legacy Rule-Based Signal Logging (for comparison)
            if rsi_val > 70.0 && obi < -0.2 {
                debug!(
                    "RULE: {} RSI Overbought ({:.2}) AND Selling Pressure (OBI {:.2}). Potential SHORT at {:.2}",
                    symbol, rsi_val, obi, price
                );
            } else if rsi_val < 30.0 && obi > 0.2 {
                debug!(
                    "RULE: {} RSI Oversold ({:.2}) AND Buying Pressure (OBI {:.2}). Potential LONG at {:.2}",
                    symbol, rsi_val, obi, price
                );
            }
        }

        // Execute pending action after mutable borrow is dropped
        if let Some((side, prob)) = pending_action {
            let msg = format!(
                "AI STRONG {} ({:.2}) for {}: Price={:.2}",
                side, prob, symbol, price
            );
            info!("{}", msg);
            self.notify(&msg);
            self.execute(&symbol, side, prob);
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

    fn notify(&self, msg: &str) {
        if let Some(ref tx) = self.notification_tx {
            let _ = tx.send(msg.to_string());
        }
    }

    fn execute(&self, symbol: &str, side: &str, confidence: f32) {
        if let Some(ref tx) = self.execution_tx {
            let quantity = match symbol.to_uppercase().as_str() {
                "BTCUSDT" => 0.0002,
                "ETHUSDT" => 0.005,
                "SOLUSDT" => 0.1,
                "DOGEUSDT" => 50.0,
                "BNBUSDT" => 0.05,
                _ => 0.0, // Safety: Don't trade symbols we haven't calibrated
            };

            if quantity > 0.0 {
                let signal = TradeSignal {
                    symbol: symbol.to_uppercase(),
                    side: side.to_string(),
                    quantity,
                    reason: format!("AI_CONFIDENCE_{:.2}", confidence),
                };
                let _ = tx.send(signal);
            } else {
                warn!(
                    "Signal generated for {} but no quantity config found. Skipping execution.",
                    symbol
                );
            }
        }
    }
}
