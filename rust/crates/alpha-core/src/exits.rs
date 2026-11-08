use crate::error::CoreError;
use alpha_types::Bar;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExitConfig {
    pub k_stop: Decimal,
    pub k_trail: Decimal,
    pub min_stop_distance_pct: Decimal,
    pub enable_e1_stagnation: bool,
    pub stagnation_bars: usize,
    pub stagnation_mfe_r: Decimal,
    pub stagnation_min_r: Decimal,
    pub stagnation_max_r: Decimal,
    pub enable_e2_breakeven: bool,
    pub breakeven_trigger_r: Decimal,
    pub breakeven_buffer_pct: Decimal,
}

impl Default for ExitConfig {
    fn default() -> Self {
        Self {
            k_stop: dec!(2.0),
            k_trail: dec!(2.5),
            min_stop_distance_pct: dec!(0.022),
            enable_e1_stagnation: false,
            stagnation_bars: 3,
            stagnation_mfe_r: dec!(0.40),
            stagnation_min_r: dec!(-0.35),
            stagnation_max_r: dec!(0.15),
            enable_e2_breakeven: false,
            breakeven_trigger_r: dec!(1.25),
            breakeven_buffer_pct: dec!(0.0020),
        }
    }
}

pub fn calculate_initial_stop(
    entry_price: Decimal,
    atr: Decimal,
    config: &ExitConfig,
) -> Result<Decimal, CoreError> {
    if entry_price <= Decimal::ZERO || atr <= Decimal::ZERO {
        return Err(CoreError::InvalidParameter {
            name: "prices",
            reason: "entry_price and atr must be strictly positive",
        });
    }

    let raw_stop = entry_price - (config.k_stop * atr);
    let max_allowed_stop = entry_price * (Decimal::ONE - config.min_stop_distance_pct);
    let final_stop = raw_stop.min(max_allowed_stop);

    if final_stop <= Decimal::ZERO {
        return Err(CoreError::InvalidParameter {
            name: "stop_loss",
            reason: "calculated stop loss must be strictly positive",
        });
    }

    Ok(final_stop)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitReason {
    StopLossHit,
    TrailingStopHit,
    StagnationTimeStop,
    BreakevenStopHit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExitVerdict {
    Hold {
        current_stop: Decimal,
        highest_high: Decimal,
        bars_held: usize,
    },
    Exit {
        reason: ExitReason,
        exit_price: Decimal,
        bars_held: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveTrade {
    pub entry_price: Decimal,
    pub initial_stop: Decimal,
    pub current_stop: Decimal,
    pub initial_risk_distance: Decimal,
    pub highest_high: Decimal,
    pub bars_held: usize,
    pub is_breakeven_active: bool,
}

impl ActiveTrade {
    pub fn new(entry_price: Decimal, initial_stop: Decimal) -> Result<Self, CoreError> {
        if entry_price <= Decimal::ZERO || initial_stop <= Decimal::ZERO {
            return Err(CoreError::InvalidParameter {
                name: "prices",
                reason: "entry_price and initial_stop must be strictly positive",
            });
        }
        if initial_stop >= entry_price {
            return Err(CoreError::InvalidParameter {
                name: "initial_stop",
                reason: "initial_stop must be strictly less than entry_price",
            });
        }

        let initial_risk_distance = entry_price - initial_stop;
        Ok(Self {
            entry_price,
            initial_stop,
            current_stop: initial_stop,
            initial_risk_distance,
            highest_high: entry_price,
            bars_held: 0,
            is_breakeven_active: false,
        })
    }

    pub fn update_bar(
        &mut self,
        bar: &Bar,
        atr: Decimal,
        config: &ExitConfig,
    ) -> Result<ExitVerdict, CoreError> {
        self.bars_held += 1;

        if bar.low <= self.current_stop {
            let reason = if self.is_breakeven_active && self.current_stop >= self.entry_price {
                ExitReason::BreakevenStopHit
            } else if self.current_stop > self.initial_stop {
                ExitReason::TrailingStopHit
            } else {
                ExitReason::StopLossHit
            };

            return Ok(ExitVerdict::Exit {
                reason,
                exit_price: self.current_stop,
                bars_held: self.bars_held,
            });
        }

        self.highest_high = self.highest_high.max(bar.high);

        if config.enable_e1_stagnation && self.bars_held >= config.stagnation_bars {
            if let Some(verdict) = self.check_stagnation(bar.close, config) {
                return Ok(verdict);
            }
        }

        if config.enable_e2_breakeven && !self.is_breakeven_active {
            let be_threshold =
                self.entry_price + (config.breakeven_trigger_r * self.initial_risk_distance);
            if self.highest_high >= be_threshold {
                let be_stop = self.entry_price * (Decimal::ONE + config.breakeven_buffer_pct);
                self.current_stop = self.current_stop.max(be_stop);
                self.is_breakeven_active = true;
            }
        }

        let candidate_trail = self.highest_high - (config.k_trail * atr);
        self.current_stop = self.current_stop.max(candidate_trail);

        Ok(ExitVerdict::Hold {
            current_stop: self.current_stop,
            highest_high: self.highest_high,
            bars_held: self.bars_held,
        })
    }

    fn check_stagnation(&self, current_close: Decimal, config: &ExitConfig) -> Option<ExitVerdict> {
        if self.initial_risk_distance.is_zero() {
            return None;
        }

        let mfe_r = (self.highest_high - self.entry_price) / self.initial_risk_distance;
        let current_r = (current_close - self.entry_price) / self.initial_risk_distance;

        if mfe_r < config.stagnation_mfe_r
            && current_r >= config.stagnation_min_r
            && current_r <= config.stagnation_max_r
        {
            return Some(ExitVerdict::Exit {
                reason: ExitReason::StagnationTimeStop,
                exit_price: current_close,
                bars_held: self.bars_held,
            });
        }

        None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn make_test_bar(high: Decimal, low: Decimal, close: Decimal) -> Bar {
        Bar::new(
            0,
            1,
            close,
            high,
            low,
            close,
            dec!(10),
            dec!(5),
            dec!(100),
            10,
        )
        .unwrap()
    }

    #[test]
    fn initial_stop_enforces_minimum_distance() {
        let config = ExitConfig::default(); // min_stop_distance_pct = 0.022 (2.20%)
        let entry = dec!(100);
        let tiny_atr = dec!(0.5); // 2.0 * 0.5 = 1.0 (1.0% stop, too tight)
        let stop = calculate_initial_stop(entry, tiny_atr, &config).unwrap();
        // Clamped to 100 * (1 - 0.022) = 97.80
        assert_eq!(stop, dec!(97.80));
    }

    #[test]
    fn stop_loss_hit_triggers_exit() {
        let config = ExitConfig::default();
        let mut trade = ActiveTrade::new(dec!(100), dec!(97.5)).unwrap();
        let bar = make_test_bar(dec!(101), dec!(97.0), dec!(97.2));
        let verdict = trade.update_bar(&bar, dec!(1.0), &config).unwrap();

        assert_eq!(
            verdict,
            ExitVerdict::Exit {
                reason: ExitReason::StopLossHit,
                exit_price: dec!(97.5),
                bars_held: 1,
            }
        );
    }

    #[test]
    fn chandelier_trail_ratchets_monotonically() {
        let config = ExitConfig::default();
        let mut trade = ActiveTrade::new(dec!(100), dec!(97.0)).unwrap();

        // Bar 1 surges to high 110, ATR = 2.0 => trail = 110 - (2.5 * 2) = 105.0
        let b1 = make_test_bar(dec!(110), dec!(100), dec!(108));
        let v1 = trade.update_bar(&b1, dec!(2.0), &config).unwrap();
        assert_eq!(
            v1,
            ExitVerdict::Hold {
                current_stop: dec!(105.0),
                highest_high: dec!(110),
                bars_held: 1,
            }
        );

        // Bar 2 pulls back to high 107, ATR = 2.5 => trail would be 107 - 6.25 = 100.75
        // Monotonic ratchet preserves stop at 105.0!
        let b2 = make_test_bar(dec!(107), dec!(106), dec!(106.5));
        let v2 = trade.update_bar(&b2, dec!(2.5), &config).unwrap();
        assert_eq!(
            v2,
            ExitVerdict::Hold {
                current_stop: dec!(105.0),
                highest_high: dec!(110),
                bars_held: 2,
            }
        );
    }

    #[test]
    fn e1_stagnation_cuts_stalled_breakout() {
        let config = ExitConfig {
            enable_e1_stagnation: true,
            stagnation_bars: 3,
            ..Default::default()
        };

        // Entry 100, Stop 98 => 1R = 2.0
        let mut trade = ActiveTrade::new(dec!(100), dec!(98)).unwrap();

        // 2 bars of stalling around 100.2 (MFE = +0.1R)
        let b1 = make_test_bar(dec!(100.3), dec!(99.8), dec!(100.1));
        trade.update_bar(&b1, dec!(1.0), &config).unwrap();
        let b2 = make_test_bar(dec!(100.4), dec!(99.9), dec!(100.0));
        trade.update_bar(&b2, dec!(1.0), &config).unwrap();

        // 3rd bar still stagnant at close 99.8 (-0.1R)
        let b3 = make_test_bar(dec!(100.2), dec!(99.5), dec!(99.8));
        let v3 = trade.update_bar(&b3, dec!(1.0), &config).unwrap();

        assert_eq!(
            v3,
            ExitVerdict::Exit {
                reason: ExitReason::StagnationTimeStop,
                exit_price: dec!(99.8),
                bars_held: 3,
            }
        );
    }

    #[test]
    fn e2_breakeven_ratchets_after_positive_excursion() {
        let config = ExitConfig {
            enable_e2_breakeven: true,
            ..Default::default()
        };

        // Entry 100, Stop 96 => 1R = 4.0. Trigger threshold = 100 + (1.25 * 4) = 105.0
        let mut trade = ActiveTrade::new(dec!(100), dec!(96)).unwrap();

        // Bar reaches high of 106 (> 105.0), but ATR is huge (5.0) so Chandelier trail = 106 - 12.5 = 93.5
        // E2 raises stop to 100 * 1.002 = 100.20
        let bar = make_test_bar(dec!(106), dec!(101), dec!(103));
        let v = trade.update_bar(&bar, dec!(5.0), &config).unwrap();

        assert_eq!(
            v,
            ExitVerdict::Hold {
                current_stop: dec!(100.20),
                highest_high: dec!(106),
                bars_held: 1,
            }
        );
        assert!(trade.is_breakeven_active);
    }
}
