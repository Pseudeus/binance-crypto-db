use std::collections::HashMap;
use tracing::info;

pub struct BalanceStore {
    balances: HashMap<String, f64>,
    maker_commission: f64,
    taker_commission: f64,
}

impl BalanceStore {
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
            maker_commission: 0.0,
            taker_commission: 0.0,
        }
    }

    pub fn set_commissions(&mut self, maker: f64, taker: f64) {
        self.maker_commission = maker;
        self.taker_commission = taker;
        info!(
            "BalanceStore: Commissions updated: Maker={:.4}, Taker={:.4}",
            maker, taker
        );
    }

    pub fn get_taker_fee(&self) -> f64 {
        self.taker_commission
    }

    pub fn get_maker_fee(&self) -> f64 {
        self.maker_commission
    }

    pub fn update(&mut self, asset: &str, free: f64) {
        self.balances.insert(asset.to_uppercase(), free);
        info!(
            "BalanceStore: Updated {} to {:.4}",
            asset.to_uppercase(),
            free
        );
    }

    pub fn get(&self, asset: &str) -> f64 {
        *self.balances.get(&asset.to_uppercase()).unwrap_or(&0.0)
    }

    pub fn has_sufficient_balance(&self, asset: &str, required: f64) -> bool {
        self.get(asset) >= required
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balance_store_updates() {
        let mut store = BalanceStore::new();
        store.update("USDT", 1000.0);
        assert_eq!(store.get("USDT"), 1000.0);
        assert_eq!(store.get("usdt"), 1000.0);

        assert!(store.has_sufficient_balance("USDT", 500.0));
        assert!(!store.has_sufficient_balance("USDT", 1500.0));
    }

    #[test]
    fn test_balance_store_empty() {
        let store = BalanceStore::new();
        assert_eq!(store.get("BTC"), 0.0);
        assert!(!store.has_sufficient_balance("BTC", 0.0001));
    }
}
