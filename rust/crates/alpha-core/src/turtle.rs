use crate::error::CoreError;
use crate::indicators::{average_true_range, donchian_channel};
use alpha_types::Bar;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TurtleConfig {
    pub entry_period: usize,
    pub exit_period: usize,
    pub atr_period: usize,
    pub atr_multiplier: Decimal,
    pub min_stop_distance: Decimal,
}

impl Default for TurtleConfig {
    fn default() -> Self {
        Self {
            entry_period: 20,
            exit_period: 10,
            atr_period: 20,
            atr_multiplier: dec!(2.0),
            min_stop_distance: dec!(0.020),
        }
    }
}

impl TurtleConfig {
    pub fn new(
        entry_period: usize,
        exit_period: usize,
        atr_period: usize,
        atr_multiplier: Decimal,
        min_stop_distance: Decimal,
    ) -> Result<Self, CoreError> {
        if entry_period == 0 || exit_period == 0 || atr_period == 0 {
            return Err(CoreError::InvalidParameter {
                name: "period",
                reason: "periods must be strictly positive",
            });
        }
        if atr_multiplier <= Decimal::ZERO || min_stop_distance <= Decimal::ZERO {
            return Err(CoreError::InvalidParameter {
                name: "multipliers",
                reason: "atr_multiplier and min_stop_distance must be strictly positive",
            });
        }
        Ok(Self {
            entry_period,
            exit_period,
            atr_period,
            atr_multiplier,
            min_stop_distance,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TurtleExitReason {
    StopLoss,
    TrailingChannel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TurtleVerdict {
    EnterLong {
        entry_price: Decimal,
        stop_price: Decimal,
        stop_distance: Decimal,
        entry_level: Decimal,
        atr: Decimal,
    },
    ExitLong {
        exit_price: Decimal,
        reason: TurtleExitReason,
    },
    Hold {
        stop_price: Decimal,
        exit_level: Decimal,
    },
    Abstain {
        reason: &'static str,
    },
}

#[inline]
fn compute_turtle_stop(
    entry_price: Decimal,
    atr: Decimal,
    multiplier: Decimal,
    min_dist: Decimal,
) -> (Decimal, Decimal) {
    let raw_stop = multiplier * atr;
    let stop_distance = (raw_stop / entry_price).max(min_dist);
    let stop_price = entry_price * (Decimal::ONE - stop_distance);
    (stop_price, stop_distance)
}

pub fn evaluate_turtle_entry(
    bars: &[Bar],
    config: &TurtleConfig,
) -> Result<TurtleVerdict, CoreError> {
    let required = config.entry_period.max(config.atr_period + 1) + 1;
    if bars.len() < required {
        return Ok(TurtleVerdict::Abstain {
            reason: "InsufficientBarsForTurtleEntry",
        });
    }

    let end_idx = bars.len().saturating_sub(1);
    let prev_bars = bars.get(..end_idx).ok_or(CoreError::InsufficientBars {
        required: end_idx,
        provided: bars.len(),
    })?;

    let (entry_level, _) = donchian_channel(prev_bars, config.entry_period)?;
    let atr = average_true_range(prev_bars, config.atr_period)?;

    let latest = bars.get(end_idx).ok_or(CoreError::InsufficientBars {
        required: end_idx + 1,
        provided: bars.len(),
    })?;

    if latest.high <= entry_level {
        return Ok(TurtleVerdict::Abstain {
            reason: "PriceBelowEntryChannel",
        });
    }

    let entry_price = latest.open.max(entry_level);
    if entry_price <= Decimal::ZERO {
        return Err(CoreError::InvalidParameter {
            name: "entry_price",
            reason: "entry_price must be strictly positive",
        });
    }

    let (stop_price, stop_distance) = compute_turtle_stop(
        entry_price,
        atr,
        config.atr_multiplier,
        config.min_stop_distance,
    );

    Ok(TurtleVerdict::EnterLong {
        entry_price,
        stop_price,
        stop_distance,
        entry_level,
        atr,
    })
}

pub fn evaluate_turtle_exit(
    bars: &[Bar],
    current_stop: Decimal,
    config: &TurtleConfig,
) -> Result<TurtleVerdict, CoreError> {
    let required = config.exit_period + 1;
    if bars.len() < required {
        return Ok(TurtleVerdict::Abstain {
            reason: "InsufficientBarsForTurtleExit",
        });
    }

    let end_idx = bars.len().saturating_sub(1);
    let prev_bars = bars.get(..end_idx).ok_or(CoreError::InsufficientBars {
        required: end_idx,
        provided: bars.len(),
    })?;

    let (_, exit_level) = donchian_channel(prev_bars, config.exit_period)?;
    let latest = bars.get(end_idx).ok_or(CoreError::InsufficientBars {
        required: end_idx + 1,
        provided: bars.len(),
    })?;

    if latest.low <= current_stop {
        return Ok(TurtleVerdict::ExitLong {
            exit_price: current_stop,
            reason: TurtleExitReason::StopLoss,
        });
    }

    if latest.low < exit_level {
        let exit_price = latest.open.min(exit_level);
        return Ok(TurtleVerdict::ExitLong {
            exit_price,
            reason: TurtleExitReason::TrailingChannel,
        });
    }

    Ok(TurtleVerdict::Hold {
        stop_price: current_stop,
        exit_level,
    })
}
