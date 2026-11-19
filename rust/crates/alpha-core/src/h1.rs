use crate::error::CoreError;
use crate::indicators::{donchian_channel, kaufman_efficiency_ratio, volume_sma};
use alpha_types::Bar;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct H1Config {
    pub er_period: usize,
    pub er_threshold: f64,
    pub donchian_period: usize,
    pub volume_sma_period: usize,
    pub volume_multiplier: Decimal,
    pub min_taker_buy_ratio: Decimal,
}

impl Default for H1Config {
    fn default() -> Self {
        Self {
            er_period: 24,
            er_threshold: 0.30,
            donchian_period: 20,
            volume_sma_period: 20,
            volume_multiplier: dec!(1.25),
            min_taker_buy_ratio: dec!(0.53),
        }
    }
}

impl H1Config {
    pub fn new(
        er_period: usize,
        er_threshold: f64,
        donchian_period: usize,
        volume_sma_period: usize,
        volume_multiplier: Decimal,
        min_taker_buy_ratio: Decimal,
    ) -> Result<Self, CoreError> {
        if er_period == 0 || donchian_period == 0 || volume_sma_period == 0 {
            return Err(CoreError::InvalidParameter {
                name: "period",
                reason: "periods must be strictly positive",
            });
        }
        if !er_threshold.is_finite() || !(0.0..=1.0).contains(&er_threshold) {
            return Err(CoreError::InvalidParameter {
                name: "er_threshold",
                reason: "er_threshold must be finite and within [0.0, 1.0]",
            });
        }
        if volume_multiplier <= Decimal::ZERO || min_taker_buy_ratio <= Decimal::ZERO {
            return Err(CoreError::InvalidParameter {
                name: "ratios",
                reason: "multipliers and ratios must be strictly positive",
            });
        }

        Ok(Self {
            er_period,
            er_threshold,
            donchian_period,
            volume_sma_period,
            volume_multiplier,
            min_taker_buy_ratio,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum H1Verdict {
    Signal {
        bar_idx: usize,
        close: Decimal,
        upper_band: Decimal,
        er: f64,
        volume: Decimal,
        volume_sma: Decimal,
        taker_buy_ratio: Decimal,
    },
    AbstainChop {
        er: f64,
        threshold: f64,
    },
    AbstainNoBreakout {
        close: Decimal,
        upper_band: Decimal,
    },
    AbstainLowVolume {
        volume: Decimal,
        required_volume: Decimal,
    },
    AbstainLowTakerRatio {
        ratio: Decimal,
        threshold: Decimal,
    },
}

impl H1Verdict {
    #[must_use]
    pub fn is_signal(&self) -> bool {
        matches!(self, Self::Signal { .. })
    }
}

pub fn evaluate_h1_signal(bars: &[Bar], config: &H1Config) -> Result<H1Verdict, CoreError> {
    let req_bars = config
        .er_period
        .max(config.donchian_period)
        .max(config.volume_sma_period)
        + 1;

    if bars.len() < req_bars {
        return Err(CoreError::InsufficientBars {
            required: req_bars,
            provided: bars.len(),
        });
    }

    let current_idx = bars.len().saturating_sub(1);
    let current_bar = bars.get(current_idx).ok_or(CoreError::InsufficientBars {
        required: current_idx + 1,
        provided: bars.len(),
    })?;

    let prior_bars = bars.get(..current_idx).ok_or(CoreError::InsufficientBars {
        required: current_idx,
        provided: bars.len(),
    })?;

    let closes: Vec<Decimal> = bars.iter().map(|b| b.close).collect();
    let er = kaufman_efficiency_ratio(&closes, config.er_period)?;
    if er < config.er_threshold {
        return Ok(H1Verdict::AbstainChop {
            er,
            threshold: config.er_threshold,
        });
    }

    let (upper_band, _) = donchian_channel(prior_bars, config.donchian_period)?;
    if current_bar.close <= upper_band {
        return Ok(H1Verdict::AbstainNoBreakout {
            close: current_bar.close,
            upper_band,
        });
    }

    let vol_sma = volume_sma(prior_bars, config.volume_sma_period)?;
    let required_volume = vol_sma * config.volume_multiplier;
    if current_bar.volume <= required_volume {
        return Ok(H1Verdict::AbstainLowVolume {
            volume: current_bar.volume,
            required_volume,
        });
    }

    if current_bar.volume.is_zero() {
        return Ok(H1Verdict::AbstainLowVolume {
            volume: Decimal::ZERO,
            required_volume,
        });
    }

    let taker_buy_ratio = current_bar.taker_buy_volume / current_bar.volume;
    if taker_buy_ratio <= config.min_taker_buy_ratio {
        return Ok(H1Verdict::AbstainLowTakerRatio {
            ratio: taker_buy_ratio,
            threshold: config.min_taker_buy_ratio,
        });
    }

    Ok(H1Verdict::Signal {
        bar_idx: current_idx,
        close: current_bar.close,
        upper_band,
        er,
        volume: current_bar.volume,
        volume_sma: vol_sma,
        taker_buy_ratio,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn make_bar(
        close: Decimal,
        high: Decimal,
        low: Decimal,
        vol: Decimal,
        taker_vol: Decimal,
    ) -> Bar {
        Bar::new(
            0,
            1,
            close,
            high,
            low,
            close,
            vol,
            taker_vol,
            vol * close,
            10,
        )
        .unwrap()
    }

    #[test]
    fn h1_fires_when_all_conditions_met() {
        let mut bars = Vec::new();
        for _ in 0..25 {
            bars.push(make_bar(dec!(100), dec!(102), dec!(98), dec!(10), dec!(5)));
        }
        bars.push(make_bar(dec!(108), dec!(109), dec!(99), dec!(20), dec!(14)));

        let config = H1Config {
            er_period: 10,
            er_threshold: 0.20,
            donchian_period: 10,
            volume_sma_period: 10,
            volume_multiplier: dec!(1.25),
            min_taker_buy_ratio: dec!(0.53),
        };

        let verdict = evaluate_h1_signal(&bars, &config).unwrap();
        assert!(verdict.is_signal());
    }

    #[test]
    fn h1_abstains_when_taker_ratio_low() {
        let mut bars = Vec::new();
        for _ in 0..25 {
            bars.push(make_bar(dec!(100), dec!(102), dec!(98), dec!(10), dec!(5)));
        }
        bars.push(make_bar(dec!(108), dec!(109), dec!(99), dec!(20), dec!(8)));

        let config = H1Config::default();
        let verdict = evaluate_h1_signal(&bars, &config).unwrap();
        assert!(matches!(verdict, H1Verdict::AbstainLowTakerRatio { .. }));
    }
}
