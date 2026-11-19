use crate::error::CoreError;
use alpha_types::Bar;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PullbackConfig {
    pub trend_sma_period: usize,
    pub rsi_period: usize,
    pub rsi_entry_threshold: Decimal,
    pub atr_period: usize,
    pub k_stop: Decimal,
    pub min_stop_distance: Decimal,
    pub ratchet_trigger_r: Decimal,
    pub ratchet_lock_r: Decimal,
    pub trail_ema_period: usize,
    pub max_hold_bars: usize,
}

impl Default for PullbackConfig {
    fn default() -> Self {
        Self {
            trend_sma_period: 200,
            rsi_period: 3,
            rsi_entry_threshold: dec!(30.0),
            atr_period: 14,
            k_stop: dec!(2.0),
            min_stop_distance: dec!(0.020),
            ratchet_trigger_r: dec!(1.0),
            ratchet_lock_r: dec!(0.15),
            trail_ema_period: 10,
            max_hold_bars: 40,
        }
    }
}

impl PullbackConfig {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        trend_sma_period: usize,
        rsi_period: usize,
        rsi_entry_threshold: Decimal,
        atr_period: usize,
        k_stop: Decimal,
        min_stop_distance: Decimal,
        ratchet_trigger_r: Decimal,
        ratchet_lock_r: Decimal,
        trail_ema_period: usize,
        max_hold_bars: usize,
    ) -> Result<Self, CoreError> {
        if trend_sma_period == 0 || rsi_period == 0 || atr_period == 0 || trail_ema_period == 0 {
            return Err(CoreError::InvalidParameter {
                name: "period",
                reason: "all periods must be strictly positive",
            });
        }
        if max_hold_bars == 0 {
            return Err(CoreError::InvalidParameter {
                name: "max_hold_bars",
                reason: "max_hold_bars must be strictly positive",
            });
        }
        if k_stop <= Decimal::ZERO || min_stop_distance <= Decimal::ZERO {
            return Err(CoreError::InvalidParameter {
                name: "stop_parameters",
                reason: "k_stop and min_stop_distance must be strictly positive",
            });
        }
        Ok(Self {
            trend_sma_period,
            rsi_period,
            rsi_entry_threshold,
            atr_period,
            k_stop,
            min_stop_distance,
            ratchet_trigger_r,
            ratchet_lock_r,
            trail_ema_period,
            max_hold_bars,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PullbackExitReason {
    StopLoss,
    EmaTrail,
    Stagnation,
    EndOfData,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PullbackVerdict {
    EnterLong {
        entry_price: Decimal,
        stop_price: Decimal,
        stop_distance: Decimal,
        atr: Decimal,
        rsi: Decimal,
    },
    ExitLong {
        exit_price: Decimal,
        reason: PullbackExitReason,
    },
    Hold {
        stop_price: Decimal,
        highest_price: Decimal,
        ratcheted: bool,
    },
    Abstain {
        reason: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PullbackExitState {
    pub entry_price: Decimal,
    pub stop_price: Decimal,
    pub highest_price: Decimal,
    pub stop_distance: Decimal,
    pub ratcheted: bool,
    pub bars_held: usize,
    pub ema_val: Decimal,
    pub next_open: Decimal,
}

pub fn evaluate_pullback_bar_exit(
    bar: &Bar,
    state: &PullbackExitState,
    config: &PullbackConfig,
) -> PullbackVerdict {
    if bar.low <= state.stop_price {
        let exit_p = if bar.open <= state.stop_price {
            bar.open.min(state.stop_price)
        } else {
            state.stop_price
        };
        return PullbackVerdict::ExitLong {
            exit_price: exit_p,
            reason: PullbackExitReason::StopLoss,
        };
    }
    if state.ratcheted && bar.close < state.ema_val {
        return PullbackVerdict::ExitLong {
            exit_price: state.next_open,
            reason: PullbackExitReason::EmaTrail,
        };
    }
    if state.bars_held >= config.max_hold_bars {
        return PullbackVerdict::ExitLong {
            exit_price: state.next_open,
            reason: PullbackExitReason::Stagnation,
        };
    }
    let highest = state.highest_price.max(bar.high);
    let risk_dollars = state.entry_price * state.stop_distance;
    let mut ratcheted = state.ratcheted;
    let mut stop_p = state.stop_price;
    if !ratcheted && risk_dollars > Decimal::ZERO {
        let excursion_r = (highest - state.entry_price) / risk_dollars;
        if excursion_r >= config.ratchet_trigger_r {
            ratcheted = true;
            let locked =
                state.entry_price * (Decimal::ONE + config.ratchet_lock_r * state.stop_distance);
            stop_p = stop_p.max(locked);
        }
    }
    PullbackVerdict::Hold {
        stop_price: stop_p,
        highest_price: highest,
        ratcheted,
    }
}

pub fn evaluate_pullback_entry(
    bars: &[Bar],
    config: &PullbackConfig,
) -> Result<PullbackVerdict, CoreError> {
    let req_bars = config.trend_sma_period + 5;
    if bars.len() < req_bars {
        return Ok(PullbackVerdict::Abstain {
            reason: "InsufficientBarsForPullback",
        });
    }
    let n = bars.len();
    let latest_idx = n - 1;
    let latest = match bars.get(latest_idx) {
        Some(b) => b,
        None => return Ok(PullbackVerdict::Abstain { reason: "NoBars" }),
    };
    let closes: Vec<Decimal> = bars.iter().map(|b| b.close).collect();
    let sma = crate::series::calc_ma_series(&closes, config.trend_sma_period, false);
    let rsi = crate::series::calc_rsi_series(&closes, config.rsi_period);
    let atr = crate::series::calc_atr_series(bars, config.atr_period);

    let (s, r, a) = match (
        sma.get(latest_idx),
        rsi.get(latest_idx),
        atr.get(latest_idx),
    ) {
        (Some(&s), Some(&r), Some(&a)) => (s, r, a),
        _ => {
            return Ok(PullbackVerdict::Abstain {
                reason: "IndicatorMissing",
            })
        }
    };

    if latest.close > s && r <= config.rsi_entry_threshold && a > Decimal::ZERO {
        let entry_price = latest.close;
        let d_s = config
            .min_stop_distance
            .max((config.k_stop * a) / entry_price);
        let stop_price = entry_price * (Decimal::ONE - d_s);
        return Ok(PullbackVerdict::EnterLong {
            entry_price,
            stop_price,
            stop_distance: d_s,
            atr: a,
            rsi: r,
        });
    }
    Ok(PullbackVerdict::Abstain {
        reason: "PullbackFilterNotTriggered",
    })
}
