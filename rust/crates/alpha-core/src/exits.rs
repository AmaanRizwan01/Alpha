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
