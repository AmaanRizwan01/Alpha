use crate::error::CoreError;
use crate::sizing::{ExchangeFilter, PositionSize};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RiskVerdict {
    Approved {
        size: PositionSize,
    },
    AbstainMinNotional {
        notional: Decimal,
        min_notional: Decimal,
    },
    AbstainMinQuantity {
        qty: Decimal,
        min_qty: Decimal,
    },
    AbstainExceedsMaxEquityFraction {
        fraction: Decimal,
        max_fraction: Decimal,
    },
    AbstainDrawdownBreaker {
        current_drawdown: Decimal,
        max_drawdown: Decimal,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskPolicy {
    pub max_equity_fraction: Decimal,
    pub max_drawdown_limit: Decimal,
}

impl RiskPolicy {
    pub fn new(
        max_equity_fraction: Decimal,
        max_drawdown_limit: Decimal,
    ) -> Result<Self, CoreError> {
        if max_equity_fraction <= Decimal::ZERO || max_equity_fraction > Decimal::ONE {
            return Err(CoreError::InvalidParameter {
                name: "max_equity_fraction",
                reason: "must be within (0, 1.0]",
            });
        }
        if max_drawdown_limit <= Decimal::ZERO || max_drawdown_limit >= Decimal::ONE {
            return Err(CoreError::InvalidParameter {
                name: "max_drawdown_limit",
                reason: "must be within (0, 1.0)",
            });
        }

        Ok(Self {
            max_equity_fraction,
            max_drawdown_limit,
        })
    }

    pub fn evaluate_entry(
        &self,
        equity: Decimal,
        equity_hwm: Decimal,
        proposed: &PositionSize,
        filter: &ExchangeFilter,
    ) -> RiskVerdict {
        if equity_hwm > Decimal::ZERO && equity < equity_hwm {
            let drawdown = (equity_hwm - equity) / equity_hwm;
            if drawdown >= self.max_drawdown_limit {
                return RiskVerdict::AbstainDrawdownBreaker {
                    current_drawdown: drawdown,
                    max_drawdown: self.max_drawdown_limit,
                };
            }
        }

        if proposed.qty < filter.min_qty {
            return RiskVerdict::AbstainMinQuantity {
                qty: proposed.qty,
                min_qty: filter.min_qty,
            };
        }

        if proposed.notional < filter.min_notional {
            return RiskVerdict::AbstainMinNotional {
                notional: proposed.notional,
                min_notional: filter.min_notional,
            };
        }

        let fraction = proposed.notional / equity;
        if fraction > self.max_equity_fraction {
            return RiskVerdict::AbstainExceedsMaxEquityFraction {
                fraction,
                max_fraction: self.max_equity_fraction,
            };
        }

        RiskVerdict::Approved {
            size: proposed.clone(),
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_size(qty: Decimal, notional: Decimal) -> PositionSize {
        PositionSize {
            qty,
            notional,
            rounded_entry: dec!(50000.0),
            rounded_stop: dec!(48500.0),
            stop_distance: dec!(1500.0),
            stop_distance_pct: dec!(0.03),
            risk_quote: dec!(1.0),
        }
    }

    fn sample_filter() -> ExchangeFilter {
        ExchangeFilter::new(dec!(0.10), dec!(0.00001), dec!(0.0001), dec!(5.0)).unwrap()
    }

    #[test]
    fn approves_valid_sizing() {
        let policy = RiskPolicy::new(dec!(0.50), dec!(0.06)).unwrap();
        let filter = sample_filter();
        let size = sample_size(dec!(0.0007), dec!(35.0));

        let verdict = policy.evaluate_entry(dec!(100.0), dec!(100.0), &size, &filter);
        assert!(matches!(verdict, RiskVerdict::Approved { .. }));
    }

    #[test]
    fn halts_on_drawdown_circuit_breaker() {
        let policy = RiskPolicy::new(dec!(0.50), dec!(0.06)).unwrap();
        let filter = sample_filter();
        let size = sample_size(dec!(0.0007), dec!(35.0));

        // 7% drawdown from HWM of 100
        let verdict = policy.evaluate_entry(dec!(93.0), dec!(100.0), &size, &filter);
        assert!(matches!(
            verdict,
            RiskVerdict::AbstainDrawdownBreaker { .. }
        ));
    }

    #[test]
    fn abstains_when_below_min_notional() {
        let policy = RiskPolicy::new(dec!(0.50), dec!(0.06)).unwrap();
        let filter = sample_filter(); // min_notional = 5.0, min_qty = 0.0001
        let size = sample_size(dec!(0.0002), dec!(2.50));

        let verdict = policy.evaluate_entry(dec!(100.0), dec!(100.0), &size, &filter);
        assert!(matches!(verdict, RiskVerdict::AbstainMinNotional { .. }));
    }
}
