use crate::inference::{InferenceEngine, InferenceResult};
use crate::services::balance_store::BalanceStore;
use crate::services::indicators;
use crate::services::risk_calculator::RiskCalculator;
use common::models::{
    AccountUpdate, AggTradeInsert, Bps, ExecutionReport, MarketEvent, OrderBookInsert, Price,
    Quantity, TradeSignal, Usdt,
};
use futures_util::future::BoxFuture;
use std::collections::HashMap;
use std::sync::Arc;
use ta::Next;
use ta::indicators::{
    BollingerBands, ExponentialMovingAverage, RelativeStrengthIndex, StandardDeviation,
};
use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use anyhow::bail;
use common::actors::{Actor, ActorType, ControlMessage};

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
            rsi: RelativeStrengthIndex::new(14).unwrap(),
            bb: BollingerBands::new(20, 2.0).unwrap(),
            std_dev: StandardDeviation::new(20).unwrap(),
            buy_vol_ema: ExponentialMovingAverage::new(100).unwrap(),
            sell_vol_ema: ExponentialMovingAverage::new(100).unwrap(),
            order_book_imbalance: 0.0,
            has_position: false,
        }
    }
}

/// High-performance strategy orchestration actor.
///
/// This service is the central intelligence hub, coordinating real-time market data
/// ingestion, AI inference, and risk-adjusted order execution.
///
/// # Complexity Justification
/// This file exceeds the 250-line limit due to the dense orchestration logic required
/// for multi-symbol state management and the integration of multiple subsystems
/// (Inference, Risk, Balance, and Notifications). The complexity is inherently tied to
/// the actor's role as the primary decision-making engine in the HFT pipeline.
pub struct StrategyService {
    id: Uuid,
    states: HashMap<String, SymbolState>,
    engine: InferenceEngine,
    balance_store: BalanceStore,
    risk_calculator: RiskCalculator,
    default_fee_rate: f64,
    margin_of_safety: f64,
    notification_tx: Option<broadcast::Sender<String>>,
    execution_tx: Option<broadcast::Sender<TradeSignal>>,
    market_rx: broadcast::Receiver<Arc<MarketEvent>>,
}

impl StrategyService {
    pub fn new(
        symbols: &[&str],
        _window_size: usize,
        model_path: &str,
        market_rx: broadcast::Receiver<Arc<MarketEvent>>,
    ) -> Self {
        let mut states = HashMap::new();
        for s in symbols {
            states.insert(s.to_lowercase(), SymbolState::new());
        }

        let engine = InferenceEngine::new(model_path);

        let risk_factor = std::env::var("RISK_FACTOR")
            .unwrap_or_else(|_| "0.01".to_string())
            .parse::<f64>()
            .unwrap_or(0.01);
        let risk_ceiling = std::env::var("RISK_CEILING")
            .unwrap_or_else(|_| "0.02".to_string())
            .parse::<f64>()
            .unwrap_or(0.02);
        let default_fee_rate = std::env::var("DEFAULT_FEE_RATE")
            .unwrap_or_else(|_| "0.001".to_string())
            .parse::<f64>()
            .unwrap_or(0.001);
        let margin_of_safety = std::env::var("MARGIN_OF_SAFETY")
            .unwrap_or_else(|_| "0.0005".to_string())
            .parse::<f64>()
            .unwrap_or(0.0005);

        Self {
            id: Uuid::new_v4(),
            states,
            engine,
            balance_store: BalanceStore::new(),
            risk_calculator: RiskCalculator::new(risk_factor, risk_ceiling),
            default_fee_rate,
            margin_of_safety,
            notification_tx: None,
            execution_tx: None,
            market_rx,
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
}

impl Actor for StrategyService {
    fn id(&self) -> Uuid {
        self.id
    }

    fn name(&self) -> ActorType {
        ActorType::StrategyActor
    }

    fn run(
        &mut self,
        supervisor_tx: mpsc::Sender<ControlMessage>,
    ) -> BoxFuture<'_, anyhow::Result<()>> {
        let _heartbeat_handle = self.spawn_heartbeat(supervisor_tx.clone());

        info!("Starting Strategy Engine for {} symbols", self.states.len());

        Box::pin(async move {
            let client = market_data::remote::BinanceClient::new();
            match client.get_account().await {
                Ok(info) => {
                    self.balance_store
                        .set_commissions(info.maker_commission, info.taker_commission);
                }
                Err(e) => warn!("Failed to fetch initial commissions: {}", e),
            }

            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));

            loop {
                tokio::select! {
                    event_res = self.market_rx.recv() => {
                        match event_res {
                            Ok(event) => {
                                match &*event {
                                    MarketEvent::AggTrade(trade) => self.process_tick(trade),
                                    MarketEvent::OrderBook(order) => self.process_orderbook(order),
                                    MarketEvent::AccountUpdate(updates) => {
                                        for update in updates {
                                            self.balance_store.update(&update.asset, update.free);
                                        }
                                    }
                                    MarketEvent::ExecutionReport(report) => {
                                        info!("Strategy: Execution Report received: {} {} @ {:?}", report.symbol, report.side, report.price);
                                    }
                                    _ => {}
                                }
                            }
                            Err(broadcast::error::RecvError::Lagged(n)) => warn!("Strategy lag: {}", n),
                            Err(_) => break,
                        }
                    }
                    _ = interval.tick() => {
                        self.log_status();
                    }
                }
            }
            info!("Strategy Engine stopped.");
            Ok(())
        })
    }
}

impl StrategyService {
    fn log_status(&self) {
        let keys = ["btcusdt", "ethusdt", "solusdt", "dogeusdt"];
        let mut summary = String::from("STATUS: ");

        for k in keys {
            if let Some(state) = self.states.get(k) {
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
        let symbol = trade.symbol.0.to_lowercase();
        let price = trade.price;
        let quantity = trade.quantity;

        let mut pending_action: Option<(&str, f32, Price, f64)> = None;

        if let Some(state) = self.states.get_mut(&symbol) {
            let rsi_val = state.rsi.next(price.0);
            let _bb_val = state.bb.next(price.0);
            let vol_val = state.std_dev.next(price.0);
            let obi = state.order_book_imbalance;

            let (buy_ema, sell_ema) = indicators::update_volume_emas(
                &mut state.buy_vol_ema,
                &mut state.sell_vol_ema,
                quantity,
                trade.is_buyer_maker,
            );

            let tfi = indicators::calculate_tfi(buy_ema, sell_ema);

            let features = vec![rsi_val as f32, obi as f32, tfi as f32, vol_val as f32];
            match self.engine.predict(&features) {
                Ok(result) => {
                    let InferenceResult { class, confidence } = result;

                    info!(
                        "AI Prediction for {}: Class={} Conf={:.4} (RSI={:.1} OBI={:.2} TFI={:.2} Vol={:.2})",
                        symbol, class, confidence, rsi_val, obi, tfi, vol_val
                    );

                    let threshold = 0.60;

                    if confidence > threshold {
                        match class {
                            1 => {
                                if !state.has_position {
                                    state.has_position = true;
                                    pending_action = Some(("BUY", confidence, price, vol_val));
                                }
                            }
                            2 => {
                                if state.has_position {
                                    state.has_position = false;
                                    pending_action = Some(("SELL", confidence, price, vol_val));
                                }
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => warn!("AI Inference Error: {}", e),
            }
        }

        if let Some((side, prob, price, vol)) = pending_action {
            self.execute(&symbol, side, prob, price, vol);
        }
    }

    fn process_orderbook(&mut self, order: &OrderBookInsert) {
        let symbol = order.symbol.to_lowercase();
        if let Some(state) = self.states.get_mut(&symbol) {
            let bid_vol = Self::calculate_volume(&order.bids);
            let ask_vol = Self::calculate_volume(&order.asks);

            state.order_book_imbalance = indicators::calculate_obi(bid_vol, ask_vol);
        }
    }

    fn calculate_volume(data: &[u8]) -> f64 {
        let mut total_vol = 0.0;
        for chunk in data.chunks_exact(8) {
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

    fn execute(&self, symbol: &str, side: &str, confidence: f32, price: Price, vol: f64) {
        let usdt_balance = self.balance_store.get("USDT");

        let taker_fee_bps = self.balance_store.get_taker_fee();
        let taker_fee_rate = if taker_fee_bps > 0.0 {
            taker_fee_bps / 10000.0
        } else {
            self.default_fee_rate
        };

        let round_trip_fee = taker_fee_rate * 2.0;
        let required_move = round_trip_fee + self.margin_of_safety;

        let fractional_vol = if price.0 > 0.0 { vol / price.0 } else { 0.0 };

        if fractional_vol < required_move {
            warn!(
                "Signal suppressed for {}: Expected move ({:.4}) < required for fees ({:.4})",
                symbol, fractional_vol, required_move
            );
            return;
        }

        let quantity = self.risk_calculator.calculate_quantity(
            Usdt(usdt_balance),
            vol,
            price,
            Bps(taker_fee_bps),
        );

        if quantity.0 > 0.0 {
            let can_execute = if side == "BUY" {
                let cost: Usdt = price * quantity;
                self.balance_store.has_sufficient_balance("USDT", cost.0)
            } else {
                let asset = symbol.to_uppercase().replace("USDT", "");
                self.balance_store
                    .has_sufficient_balance(&asset, quantity.0)
            };

            if can_execute {
                let msg = format!(
                    "AI STRONG {} ({:.2}) for {}: Price={:?} Qty={:?} (Bal_USDT={:.2})",
                    side, confidence, symbol, price, quantity.0, usdt_balance
                );
                info!("{}", msg);
                self.notify(&msg);

                if let Some(ref tx) = self.execution_tx {
                    let signal = TradeSignal {
                        symbol: symbol.to_uppercase(),
                        side: side.to_string(),
                        quantity: quantity,
                        reason: format!("AI_CONF__{:.2}_VOL_{:.4}", confidence, vol),
                    };
                    let _ = tx.send(signal);
                }
            } else {
                warn!(
                    "Signal generated for {} {} but insufficient balance.",
                    symbol, side
                );
            }
        } else {
            warn!(
                "Signal generated for {} but calculated quantity is 0 (Vol too high or Bal too low).",
                symbol
            );
        }
    }
}
