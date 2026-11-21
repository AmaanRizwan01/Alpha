use crate::error::CoreError;
use crate::indicators::{average_true_range, kaufman_efficiency_ratio};
use alpha_types::Bar;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ChopVerdict {
    Pass,
    AbstainChop {
        er: f64,
        threshold: f64,
    },
    AbstainLowVolatility {
        atr_pct: Decimal,
        threshold: Decimal,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChopFilter {
    pub er_period: usize,
    pub er_threshold: f64,
    pub atr_period: usize,
    pub min_atr_pct: Decimal,
}

impl ChopFilter {
    pub fn new(
        er_period: usize,
        er_threshold: f64,
        atr_period: usize,
        min_atr_pct: Decimal,
    ) -> Result<Self, CoreError> {
        if er_period == 0 || atr_period == 0 {
            return Err(CoreError::InvalidParameter {
                name: "period",
                reason: "periods must be strictly positive",
            });
        }
        if !er_threshold.is_finite() || er_threshold < 0.0 || er_threshold > 1.0 {
            return Err(CoreError::InvalidParameter {
                name: "er_threshold",
                reason: "er_threshold must be finite and within [0.0, 1.0]",
            });
        }
        if min_atr_pct < Decimal::ZERO {
            return Err(CoreError::InvalidParameter {
                name: "min_atr_pct",
                reason: "min_atr_pct must be non-negative",
            });
        }

        Ok(Self {
            er_period,
            er_threshold,
            atr_period,
            min_atr_pct,
        })
    }

    pub fn evaluate(&self, bars: &[Bar]) -> Result<ChopVerdict, CoreError> {
        let max_req = self.er_period.max(self.atr_period) + 1;
        if bars.len() < max_req {
            return Err(CoreError::InsufficientBars {
                required: max_req,
                provided: bars.len(),
            });
        }

        let closes: Vec<Decimal> = bars.iter().map(|b| b.close).collect();
        let er = kaufman_efficiency_ratio(&closes, self.er_period)?;

        if er < self.er_threshold {
            return Ok(ChopVerdict::AbstainChop {
                er,
                threshold: self.er_threshold,
            });
        }

        let atr = average_true_range(bars, self.atr_period)?;
        let current_close = bars
            .last()
            .ok_or(CoreError::InsufficientBars {
                required: 1,
                provided: 0,
            })?
            .close;

        let atr_pct = atr / current_close;
        if atr_pct < self.min_atr_pct {
            return Ok(ChopVerdict::AbstainLowVolatility {
                atr_pct,
                threshold: self.min_atr_pct,
            });
        }

        Ok(ChopVerdict::Pass)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn make_test_bar(close: Decimal, range: Decimal) -> Bar {
        Bar::new(
            100,
            200,
            close,
            close + range,
            close - range,
            close,
            dec!(100),
            dec!(50),
            close * dec!(100),
            10,
        )
        .unwrap()
    }

    #[test]
    fn passes_strong_trend_with_high_volatility() {
        let mut bars = Vec::new();
        for i in 0..30 {
            let p = Decimal::from(100 + i * 2);
            bars.push(make_test_bar(p, dec!(5.0)));
        }

        let filter = ChopFilter::new(24, 0.30, 14, dec!(0.01)).unwrap();
        let verdict = filter.evaluate(&bars).unwrap();
        assert_eq!(verdict, ChopVerdict::Pass);
    }

    #[test]
    fn abstains_on_sawtooth_chop() {
        let mut bars = Vec::new();
        for i in 0..30 {
            let p = if i % 2 == 0 { dec!(100) } else { dec!(102) };
            bars.push(make_test_bar(p, dec!(2.0)));
        }

        let filter = ChopFilter::new(24, 0.30, 14, dec!(0.005)).unwrap();
        let verdict = filter.evaluate(&bars).unwrap();
        assert!(matches!(verdict, ChopVerdict::AbstainChop { .. }));
    }
}
