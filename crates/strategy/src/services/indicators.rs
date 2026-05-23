use ta::Next;
use ta::indicators::{RelativeStrengthIndex, ExponentialMovingAverage};
use common::models::{Price, Quantity};

/// Pure mathematical indicators for market analysis.
///
/// This module contains stateless logic for calculating technical features
/// used by the AI inference engine.

/// Calculates the Order Book Imbalance (OBI).
///
/// Formula: `(BidVolume - AskVolume) / (BidVolume + AskVolume)`
///
/// # Complexity
/// - **Time Complexity**: O(1) - basic arithmetic.
/// - **Memory Complexity**: O(1).
pub fn calculate_obi(bid_vol: f64, ask_vol: f64) -> f64 {
    let total = bid_vol + ask_vol;
    if total > 0.0 {
        (bid_vol - ask_vol) / total
    } else {
        0.0
    }
}

/// Calculates the Trade Flow Imbalance (TFI) using EMAs of buy and sell volumes.
///
/// Formula: `(BuyEMA - SellEMA) / (BuyEMA + SellEMA)`
///
/// # Complexity
/// - **Time Complexity**: O(1) - EMA update and arithmetic.
/// - **Memory Complexity**: O(1).
pub fn calculate_tfi(buy_ema: f64, sell_ema: f64) -> f64 {
    let total = buy_ema + sell_ema;
    if total > 0.0 {
        (buy_ema - sell_ema) / total
    } else {
        0.0
    }
}

/// Helper to update volume EMAs based on buyer-maker status.
///
/// # Complexity
/// - **Time Complexity**: O(1).
/// - **Memory Complexity**: O(1).
pub fn update_volume_emas(
    buy_ema: &mut ExponentialMovingAverage,
    sell_ema: &mut ExponentialMovingAverage,
    quantity: Quantity,
    is_buyer_maker: bool,
) -> (f64, f64) {
    let (buy_q, sell_q) = if is_buyer_maker {
        (0.0, quantity.0)
    } else {
        (quantity.0, 0.0)
    };

    (buy_ema.next(buy_q), sell_ema.next(sell_q))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ta::indicators::ExponentialMovingAverage;

    #[test]
    fn test_obi_calculation() {
        // High buying pressure
        assert_eq!(calculate_obi(100.0, 0.0), 1.0);
        // High selling pressure
        assert_eq!(calculate_obi(0.0, 100.0), -1.0);
        // Balanced
        assert_eq!(calculate_obi(50.0, 50.0), 0.0);
        // Zero volume
        assert_eq!(calculate_obi(0.0, 0.0), 0.0);
    }

    #[test]
    fn test_tfi_calculation() {
        // More buy volume EMA than sell
        assert_eq!(calculate_tfi(10.0, 5.0), (10.0 - 5.0) / 15.0);
        // Equal
        assert_eq!(calculate_tfi(10.0, 10.0), 0.0);
        // More sell
        assert_eq!(calculate_tfi(5.0, 10.0), (5.0 - 10.0) / 15.0);
    }

    #[test]
    fn test_volume_ema_updates() {
        let mut buy_ema = ExponentialMovingAverage::new(2).unwrap();
        let mut sell_ema = ExponentialMovingAverage::new(2).unwrap();
        
        // Is buyer maker = false (Market Buy)
        let (b1, s1) = update_volume_emas(&mut buy_ema, &mut sell_ema, Quantity(10.0), false);
        assert!(b1 > 0.0);
        assert_eq!(s1, 0.0);

        // Is buyer maker = true (Market Sell)
        let (b2, s2) = update_volume_emas(&mut buy_ema, &mut sell_ema, Quantity(20.0), true);
        assert!(s2 > 0.0);
        assert!(b2 < b1); // EMA decay
    }
}
