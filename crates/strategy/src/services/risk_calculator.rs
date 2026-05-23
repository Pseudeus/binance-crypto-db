use common::models::{Bps, Price, Quantity, Usdt};
use tracing::warn;

pub struct RiskCalculator {
    risk_factor: f64,  // 0.01 = 1% risk per trade
    risk_ceiling: f64, // 0.02 = Maximum 2% risk of total equity in one trade
}

impl RiskCalculator {
    pub fn new(risk_factor: f64, risk_ceiling: f64) -> Self {
        Self {
            risk_factor,
            risk_ceiling,
        }
    }

    /// Calculates the optimal trade quantity based on account equity, market volatility, and dynamic exchange fees.
    ///
    /// This implementation uses a volatility-adjusted risk model to ensure that the amount of capital
    /// at risk remains constant across different market conditions.
    ///
    /// # Mathematical Formula
    /// The calculation follows these steps:
    /// 1. **Usable Equity Calculation**:
    ///    `usable_equity = total_equity * (1.0 - fee_rate)`
    ///    This ensures that the fee paid to the exchange is accounted for *before* sizing, preventing
    ///    orders that would exceed the available balance.
    ///
    /// 2. **Risk Amount**:
    ///    `risk_amount = usable_equity * min(risk_factor, risk_ceiling)`
    ///    The actual USDT value we are willing to lose based on the volatility unit.
    ///
    /// 3. **Quantity**:
    ///    `quantity = risk_amount / (volatility * price)`
    ///
    /// # Parameters
    /// * `total_equity`: The current available balance in the base quote asset (e.g., USDT).
    /// * `volatility`: The rolling standard deviation of the price (as a fraction of price).
    /// * `price`: The current market price of the asset.
    /// * `fee_rate`: The dynamic taker fee rate in basis points.
    ///
    /// # Complexity
    /// * **Time Complexity**: O(1) - Constant time arithmetic.
    /// * **Memory Complexity**: O(1) - Stack-allocated primitives.
    ///
    /// # Safety
    /// Returns `0.0` if any input is non-positive to prevent division by zero or nonsensical sizes.
    ///
    /// # Formula
    /// ```text
    /// usable_equity = total_equity * (1.0 - fee_rate.to_decimal())
    /// effective_risk = min(risk_factor, risk_ceiling)
    /// risk_amount = usable_equity * effective_risk
    /// quantity = risk_amount / (volatility * price)
    /// ```
    pub fn calculate_quantity(
        &self,
        total_equity: Usdt,
        volatility: f64, // volatility as fraction of price
        price: Price,
        fee_rate: Bps,
    ) -> Quantity {
        if volatility <= 0.0 || price.0 <= 0.0 || total_equity.0 <= 0.0 {
            warn!(
                "RiskCalculator: Invalid input values. Volatility={:.4}, Price={:.2}, Equity={:.2}",
                volatility, price.0, total_equity.0
            );
            return Quantity(0.0);
        }

        // Subtract a buffer for fees to ensure we don't size exactly to our total balance.
        let usable_equity = Usdt(total_equity.0 * (1.0 - fee_rate.to_decimal()));

        // Clamp risk factor by ceiling
        let effective_risk = self.risk_factor.min(self.risk_ceiling);

        let risk_amount = Usdt(usable_equity.0 * effective_risk);
        let volatility_unit = Price(volatility * price.0);

        if volatility_unit.0 <= 0.0 {
            return Quantity(0.0);
        }

        let qty: Quantity = risk_amount / volatility_unit;

        qty
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_risk_calculation_standard() {
        let calc = RiskCalculator::new(0.01, 0.02);
        // Equity=1000, Risk=1%, Volatility=2%, Price=100, Fee=0%
        // (1000 * 0.01) / (0.02 * 100) = 10 / 2 = 5.0
        let qty = calc.calculate_quantity(Usdt(1000.0), 0.02, Price(100.0), Bps(0.0));
        assert_eq!(qty.0, 5.0);
    }

    #[test]
    fn test_risk_calculation_with_dynamic_fee() {
        let calc = RiskCalculator::new(0.01, 0.02);
        // Equity=1000, Risk=1%, Volatility=2%, Price=100, Fee=0.1% (10 bps)
        // UsableEquity = 999.0
        // (999 * 0.01) / (0.02 * 100) = 9.99 / 2 = 4.995
        let qty = calc.calculate_quantity(Usdt(1000.0), 0.02, Price(100.0), Bps(10.0));
        assert_eq!(qty.0, 4.995);
    }

    #[test]
    fn test_risk_calculation_ceiling_trigger() {
        // Ceiling=1%, Factor=5%
        let calc = RiskCalculator::new(0.05, 0.01);
        // Effective risk should be 1%
        let qty = calc.calculate_quantity(Usdt(1000.0), 0.02, Price(100.0), Bps(0.0));
        assert_eq!(qty.0, 5.0);
    }

    #[test]
    fn test_risk_calculation_zero_inputs() {
        let calc = RiskCalculator::new(0.01, 0.02);
        assert_eq!(calc.calculate_quantity(Usdt(0.0), 0.02, Price(100.0), Bps(10.0)).0, 0.0);
        assert_eq!(calc.calculate_quantity(Usdt(1000.0), 0.0, Price(100.0), Bps(10.0)).0, 0.0);
        assert_eq!(calc.calculate_quantity(Usdt(1000.0), 0.02, Price(0.0), Bps(10.0)).0, 0.0);
    }

    #[test]
    fn test_risk_calculation_negative_inputs() {
        let calc = RiskCalculator::new(0.01, 0.02);
        assert_eq!(calc.calculate_quantity(Usdt(-1000.0), 0.02, Price(100.0), Bps(10.0)).0, 0.0);
        assert_eq!(calc.calculate_quantity(Usdt(1000.0), -0.02, Price(100.0), Bps(10.0)).0, 0.0);
    }
}

