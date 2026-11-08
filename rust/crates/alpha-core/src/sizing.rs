use crate::error::CoreError;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExchangeFilter {
    pub tick_size: Decimal,
    pub step_size: Decimal,
    pub min_qty: Decimal,
    pub min_notional: Decimal,
}

impl ExchangeFilter {
    pub fn new(
        tick_size: Decimal,
        step_size: Decimal,
        min_qty: Decimal,
        min_notional: Decimal,
    ) -> Result<Self, CoreError> {
        if tick_size <= Decimal::ZERO || step_size <= Decimal::ZERO {
            return Err(CoreError::InvalidParameter {
                name: "filter_sizes",
                reason: "tick_size and step_size must be strictly positive",
            });
        }
        if min_qty <= Decimal::ZERO || min_notional <= Decimal::ZERO {
            return Err(CoreError::InvalidParameter {
                name: "filter_minimums",
                reason: "min_qty and min_notional must be strictly positive",
            });
        }

        Ok(Self {
            tick_size,
            step_size,
            min_qty,
            min_notional,
        })
    }

    #[must_use]
    pub fn round_qty_down(&self, qty: Decimal) -> Decimal {
        (qty / self.step_size).floor() * self.step_size
    }

    #[must_use]
    pub fn round_price_down(&self, price: Decimal) -> Decimal {
        (price / self.tick_size).floor() * self.tick_size
    }

    #[must_use]
    pub fn round_price_up(&self, price: Decimal) -> Decimal {
        (price / self.tick_size).ceil() * self.tick_size
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FeeBudget {
    pub maker_fee_rate: Decimal,
    pub taker_fee_rate: Decimal,
    pub slippage_allowance: Decimal,
}

impl FeeBudget {
    pub fn new(
        maker_fee_rate: Decimal,
        taker_fee_rate: Decimal,
        slippage_allowance: Decimal,
    ) -> Result<Self, CoreError> {
        if maker_fee_rate < Decimal::ZERO
            || taker_fee_rate < Decimal::ZERO
            || slippage_allowance < Decimal::ZERO
        {
            return Err(CoreError::InvalidParameter {
                name: "fee_budget",
                reason: "fee rates and slippage must be non-negative",
            });
        }

        Ok(Self {
            maker_fee_rate,
            taker_fee_rate,
            slippage_allowance,
        })
    }

    #[must_use]
    pub fn total_friction(&self) -> Decimal {
        self.maker_fee_rate + self.taker_fee_rate + self.slippage_allowance
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PositionSize {
    pub qty: Decimal,
    pub notional: Decimal,
    pub rounded_entry: Decimal,
    pub rounded_stop: Decimal,
    pub stop_distance: Decimal,
    pub stop_distance_pct: Decimal,
    pub risk_quote: Decimal,
}

pub fn calculate_position_size(
    equity: Decimal,
    risk_fraction: Decimal,
    raw_entry: Decimal,
    raw_stop: Decimal,
    fee_budget: &FeeBudget,
    filter: &ExchangeFilter,
    min_stop_distance_pct: Decimal,
) -> Result<PositionSize, CoreError> {
    if equity <= Decimal::ZERO || risk_fraction <= Decimal::ZERO || risk_fraction > dec!(0.05) {
        return Err(CoreError::InvalidParameter {
            name: "risk_parameters",
            reason: "equity must be positive and risk_fraction between (0, 0.05]",
        });
    }

    let rounded_entry = filter.round_price_down(raw_entry);
    let rounded_stop = filter.round_price_down(raw_stop);
    if rounded_stop >= rounded_entry {
        return Err(CoreError::InvalidParameter {
            name: "stop_placement",
            reason: "rounded stop price must be strictly below entry price",
        });
    }

    let stop_distance = rounded_entry - rounded_stop;
    let stop_dist_pct = stop_distance / rounded_entry;
    if stop_dist_pct < min_stop_distance_pct {
        return Err(CoreError::InvalidParameter {
            name: "stop_distance",
            reason: "stop distance falls below minimum structural threshold",
        });
    }

    let per_unit_loss = stop_dist_pct + fee_budget.total_friction();
    let target_risk_quote = equity * risk_fraction;
    let target_notional = target_risk_quote / per_unit_loss;
    let raw_qty = target_notional / rounded_entry;
    let final_qty = filter.round_qty_down(raw_qty);

    let notional = final_qty * rounded_entry;
    let risk_quote = notional * per_unit_loss;

    Ok(PositionSize {
        qty: final_qty,
        notional,
        rounded_entry,
        rounded_stop,
        stop_distance,
        stop_distance_pct: stop_dist_pct,
        risk_quote,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn standard_filter() -> ExchangeFilter {
        ExchangeFilter::new(dec!(0.10), dec!(0.00001), dec!(0.00001), dec!(5.0)).unwrap()
    }

    fn standard_fees() -> FeeBudget {
        FeeBudget::new(dec!(0.0008), dec!(0.0010), dec!(0.0005)).unwrap()
    }

    #[test]
    fn scale_invariance_100_vs_10000() {
        let filter = standard_filter();
        let fees = standard_fees();
        let entry = dec!(50000.0);
        let stop = dec!(48500.0); // 3.0% stop distance
        let risk_frac = dec!(0.01); // 1.0% risk
        let min_stop = dec!(0.022); // 2.2% min stop

        let p100 = calculate_position_size(
            dec!(100.0),
            risk_frac,
            entry,
            stop,
            &fees,
            &filter,
            min_stop,
        )
        .unwrap();

        let p10000 = calculate_position_size(
            dec!(10000.0),
            risk_frac,
            entry,
            stop,
            &fees,
            &filter,
            min_stop,
        )
        .unwrap();

        // Check notional percentage of equity
        let frac100 = p100.notional / dec!(100.0);
        let frac10000 = p10000.notional / dec!(10000.0);

        // Due to lot quantization on $100 (0.00001 BTC steps = $0.50),
        // frac100 should match frac10000 within 1.5%
        let diff = (frac100 - frac10000).abs();
        assert!(
            diff < dec!(0.015),
            "quantization divergence too high: {}",
            diff
        );

        // Actual dollar risk must never exceed 1.00% of equity
        assert!(p100.risk_quote <= dec!(1.00));
        assert!(p10000.risk_quote <= dec!(100.00));
    }

    #[test]
    fn rejects_sub_threshold_stop_distance() {
        let filter = standard_filter();
        let fees = standard_fees();
        let res = calculate_position_size(
            dec!(1000.0),
            dec!(0.01),
            dec!(50000.0),
            dec!(49500.0), // 1.0% stop < 2.2% min
            &fees,
            &filter,
            dec!(0.022),
        );
        assert!(res.is_err());
    }
}
