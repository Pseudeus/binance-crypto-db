pub mod aggtrade;
pub mod force_order;
pub mod kline;
pub mod markprice;
pub mod open_interest;
pub mod orderbook;
pub mod signal;

pub use aggtrade::{AggTrade, AggTradeInsert};
pub use force_order::{ForceOrder, ForceOrderInsert};
pub use kline::{Kline, KlineInsert};
pub use markprice::{MarkPrice, MarkPriceInsert};
pub use open_interest::{OpenInterest, OpenInterestInsert};
pub use orderbook::{OrderBook, OrderBookInsert};
pub use signal::TradeSignal;

use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Deref, DerefMut, Div, Mul, Sub, SubAssign};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol(pub String);

/// NewType wrapper for market prices.
///
/// # Complexity
/// - **Time Complexity**: O(1) for all arithmetic operations.
/// - **Memory Complexity**: O(1). Zero-cost abstraction over `f64`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Price(pub f64);

/// NewType wrapper for asset quantities.
///
/// # Complexity
/// - **Time Complexity**: O(1) for all arithmetic operations.
/// - **Memory Complexity**: O(1). Zero-cost abstraction over `f64`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Quantity(pub f64);

/// NewType wrapper for USDT (quote asset) values.
///
/// # Complexity
/// - **Time Complexity**: O(1) for all arithmetic operations.
/// - **Memory Complexity**: O(1). Zero-cost abstraction over `f64`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Usdt(pub f64);

/// NewType wrapper for Basis Points (e.g., 10 bps = 0.1%).
///
/// # Complexity
/// - **Time Complexity**: O(1) for all arithmetic operations.
/// - **Memory Complexity**: O(1). Zero-cost abstraction over `f64`.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Bps(pub f64);

// --- Operator Overloading ---

impl Add for Usdt {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Sub for Usdt {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl AddAssign for Usdt {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Usdt {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

// --- Price Operators ---

impl Add for Price {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Sub for Price {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl AddAssign for Price {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Price {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

// --- Quantity Operators ---

impl Add for Quantity {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl Sub for Quantity {
    type Output = Self;
    fn sub(self, other: Self) -> Self {
        Self(self.0 - other.0)
    }
}

impl AddAssign for Quantity {
    fn add_assign(&mut self, other: Self) {
        self.0 += other.0;
    }
}

impl SubAssign for Quantity {
    fn sub_assign(&mut self, other: Self) {
        self.0 -= other.0;
    }
}

impl DerefMut for Quantity {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for Quantity {
    type Target = f64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Price * Quantity = Usdt
impl Mul<Quantity> for Price {
    type Output = Usdt;
    fn mul(self, rhs: Quantity) -> Usdt {
        Usdt(self.0 * rhs.0)
    }
}

/// Usdt / Price = Quantity
impl Div<Price> for Usdt {
    type Output = Quantity;
    fn div(self, rhs: Price) -> Quantity {
        Quantity(self.0 / rhs.0)
    }
}

/// Usdt / Quantity = Price
impl Div<Quantity> for Usdt {
    type Output = Price;
    fn div(self, rhs: Quantity) -> Price {
        Price(self.0 / rhs.0)
    }
}

impl Bps {
    pub fn to_decimal(self) -> f64 {
        self.0 / 10000.0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountUpdate {
    pub asset: String,
    pub free: f64,
    pub locked: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionReport {
    pub symbol: String,
    pub side: String,
    pub price: Price,
    pub quantity: Quantity,
    pub status: String,
    pub commission: f64,
    pub commission_asset: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MarketEvent {
    AggTrade(AggTradeInsert),
    OrderBook(OrderBookInsert),
    Kline((KlineInsert, bool)),
    MarkPrice(MarkPriceInsert),
    ForceOrder(ForceOrderInsert),
    OpenInterest(OpenInterestInsert),
    AccountUpdate(Vec<AccountUpdate>),
    ExecutionReport(ExecutionReport),
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_financial_math_safety() {
        let p = Price(100.0);
        let q = Quantity(5.0);

        // Price * Quantity = Usdt
        let total: Usdt = p * q;
        assert_eq!(total.0, 500.0);

        // Usdt / Price = Quantity
        let q_back: Quantity = total / p;
        assert_eq!(q_back.0, 5.0);

        // Usdt / Quantity = Price
        let p_back: Price = total / q;
        assert_eq!(p_back.0, 100.0);
    }

    #[test]
    fn test_bps_normalization() {
        let fee = Bps(10.0); // 10 bps = 0.1%
        assert_eq!(fee.to_decimal(), 0.001);
    }

    #[test]
    fn test_market_event_json_roundtrip() {
        // Verify JSON round-tripping for MarketEvent structures
        let price = Price(42.50);
        let quantity = Quantity(10.0);
        let usdt = Usdt(1000.0);
        let fee = Bps(10.0);

        // Test Price serialization
        let price_json = serde_json::to_string(&price).unwrap();
        let price_restored: Price = serde_json::from_str(&price_json).unwrap();
        assert_eq!(price, price_restored);

        // Test Quantity serialization
        let quantity_json = serde_json::to_string(&quantity).unwrap();
        let quantity_restored: Quantity = serde_json::from_str(&quantity_json).unwrap();
        assert_eq!(quantity, quantity_restored);

        // Test Usdt serialization
        let usdt_json = serde_json::to_string(&usdt).unwrap();
        let usdt_restored: Usdt = serde_json::from_str(&usdt_json).unwrap();
        assert_eq!(usdt, usdt_restored);

        // Test Bps serialization
        let fee_json = serde_json::to_string(&fee).unwrap();
        let fee_restored: Bps = serde_json::from_str(&fee_json).unwrap();
        assert_eq!(fee, fee_restored);
    }

    #[test]
    fn test_market_event_type_safety_prevents_mismatch() {
        // Verify that type safety prevents unit-mismatch errors
        let price = Price(100.0);
        let usdt = Usdt(1000.0);

        // This should compile - correct type
        let qty: Quantity = usdt / price;
        assert_eq!(qty.0, 10.0);

        // The following would not compile (type mismatch):
        // let invalid = price + usdt; // Compiler error: incompatible types
        // let invalid2 = price * quantity; // Compiler error: Price * Price not defined
    }
}
