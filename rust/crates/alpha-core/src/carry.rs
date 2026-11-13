use crate::error::CoreError;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

const CYCLES_PER_YEAR: Decimal = dec!(1095); // 3 funding cycles per day * 365 days

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FundingCarryConfig {
    pub entry_apr: Decimal,
    pub exit_apr: Decimal,
    pub ema_alpha: Decimal,
    pub inversion_stop_rate: Decimal,
    pub round_trip_friction: Decimal,
    pub fee_hurdle_mult: Decimal,
    pub min_decay_consecutive: usize,
    pub perp_fraction: Decimal,
    pub predictive_pre_exit_rate: Decimal,
    pub adl_tolerance_ratio: Decimal,
}

impl Default for FundingCarryConfig {
    fn default() -> Self {
        Self {
            entry_apr: dec!(0.120),
            exit_apr: dec!(0.022),
            ema_alpha: dec!(0.20),
            inversion_stop_rate: dec!(-0.00015),
            round_trip_friction: dec!(0.0032),
            fee_hurdle_mult: dec!(1.0),
            min_decay_consecutive: 3,
            perp_fraction: dec!(2) / dec!(3),
            predictive_pre_exit_rate: dec!(-0.00005),
            adl_tolerance_ratio: dec!(0.99),
        }
    }
}

impl FundingCarryConfig {
    pub fn new(
        entry_apr: Decimal,
        exit_apr: Decimal,
        ema_alpha: Decimal,
        inversion_stop_rate: Decimal,
        round_trip_friction: Decimal,
        fee_hurdle_mult: Decimal,
    ) -> Result<Self, CoreError> {
        if entry_apr <= exit_apr {
            return Err(CoreError::InvalidParameter {
                name: "entry_apr",
                reason: "entry APR must exceed exit APR",
            });
        }
        if ema_alpha <= Decimal::ZERO || ema_alpha > Decimal::ONE {
            return Err(CoreError::InvalidParameter {
                name: "ema_alpha",
                reason: "ema_alpha must be in (0.0, 1.0]",
            });
        }
        if round_trip_friction < Decimal::ZERO || fee_hurdle_mult < Decimal::ZERO {
            return Err(CoreError::InvalidParameter {
                name: "friction_or_hurdle",
                reason: "friction and fee hurdle must be non-negative",
            });
        }
        Ok(Self {
            entry_apr,
            exit_apr,
            ema_alpha,
            inversion_stop_rate,
            round_trip_friction,
            fee_hurdle_mult,
            min_decay_consecutive: 3,
            perp_fraction: dec!(2) / dec!(3),
            predictive_pre_exit_rate: dec!(-0.00005),
            adl_tolerance_ratio: dec!(0.99),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CarryExitReason {
    FundingInversionStop,
    FundingDecayExit,
    PredictivePreExit,
    AdlDesyncEmergency,
    EndOfSample,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulatedCarryTrade {
    pub entry_idx: usize,
    pub exit_idx: usize,
    pub entry_time_ms: i64,
    pub exit_time_ms: i64,
    pub epochs_held: usize,
    pub gross_return_pct: Decimal,
    pub fee_pct_total: Decimal,
    pub net_return_pct: Decimal,
    pub gross_pnl_r: Decimal,
    pub fee_r: Decimal,
    pub net_pnl_r: Decimal,
    pub exit_reason: CarryExitReason,
}

#[must_use]
pub fn update_funding_ema(prev_ema: Decimal, current_rate: Decimal, alpha: Decimal) -> Decimal {
    alpha * current_rate + (Decimal::ONE - alpha) * prev_ema
}

#[must_use]
pub fn funding_rate_to_apr(smoothed_rate: Decimal) -> Decimal {
    smoothed_rate * CYCLES_PER_YEAR
}

#[must_use]
pub fn check_adl_desync(spot_qty: Decimal, perp_qty: Decimal, tolerance_ratio: Decimal) -> bool {
    let perp_abs = perp_qty.abs();
    let min_perp_required = spot_qty.abs() * tolerance_ratio;
    perp_abs < min_perp_required
}

pub fn calc_funding_apr_series(rates: &[Decimal], alpha: Decimal) -> Vec<Decimal> {
    if rates.is_empty() {
        return Vec::new();
    }
    let mut apr = Vec::with_capacity(rates.len());
    let mut ema = match rates.first() {
        Some(&r) => r,
        None => Decimal::ZERO,
    };
    apr.push(funding_rate_to_apr(ema));

    for &rate in rates.iter().skip(1) {
        ema = update_funding_ema(ema, rate, alpha);
        apr.push(funding_rate_to_apr(ema));
    }
    apr
}

fn evaluate_epoch_exit(
    cfg: &FundingCarryConfig,
    rate: Decimal,
    apr_val: Decimal,
    decay_count: &mut usize,
    gross_pct: Decimal,
    fee_pct_total: Decimal,
) -> Option<CarryExitReason> {
    if rate < cfg.inversion_stop_rate {
        return Some(CarryExitReason::FundingInversionStop);
    }
    if apr_val > cfg.exit_apr {
        *decay_count = 0;
        return None;
    }
    *decay_count += 1;
    if *decay_count >= cfg.min_decay_consecutive && gross_pct >= fee_pct_total * cfg.fee_hurdle_mult
    {
        return Some(CarryExitReason::FundingDecayExit);
    }
    None
}

fn build_trade(
    entry_idx: usize,
    exit_idx: usize,
    times: &[i64],
    gross: Decimal,
    fee: Decimal,
    r_unit: Decimal,
    reason: CarryExitReason,
) -> SimulatedCarryTrade {
    let net = gross - fee;
    SimulatedCarryTrade {
        entry_idx,
        exit_idx,
        entry_time_ms: *times.get(entry_idx).unwrap_or(&0),
        exit_time_ms: *times.get(exit_idx).unwrap_or(&0),
        epochs_held: exit_idx.saturating_sub(entry_idx) + 1,
        gross_return_pct: gross,
        fee_pct_total: fee,
        net_return_pct: net,
        gross_pnl_r: gross / r_unit,
        fee_r: fee / r_unit,
        net_pnl_r: net / r_unit,
        exit_reason: reason,
    }
}

pub fn simulate_carry_series(
    cfg: &FundingCarryConfig,
    rates: &[Decimal],
    timestamps: &[i64],
    r_unit: Decimal,
) -> Result<Vec<SimulatedCarryTrade>, CoreError> {
    if rates.is_empty() || rates.len() != timestamps.len() {
        return Ok(Vec::new());
    }
    let apr = calc_funding_apr_series(rates, cfg.ema_alpha);
    let fee_pct = cfg.perp_fraction * cfg.round_trip_friction;
    let mut trades = Vec::new();
    let mut in_pos = false;
    let mut entry_idx = 0;
    let mut decay_count = 0;

    for i in 0..rates.len() {
        let rate_i = *rates.get(i).unwrap_or(&Decimal::ZERO);
        let apr_i = *apr.get(i).unwrap_or(&Decimal::ZERO);
        if !in_pos {
            if apr_i >= cfg.entry_apr {
                in_pos = true;
                entry_idx = i;
                decay_count = 0;
            }
            continue;
        }
        let gross_pct = cfg.perp_fraction * rates[entry_idx..=i].iter().sum::<Decimal>();
        if let Some(reason) =
            evaluate_epoch_exit(cfg, rate_i, apr_i, &mut decay_count, gross_pct, fee_pct)
        {
            trades.push(build_trade(
                entry_idx, i, timestamps, gross_pct, fee_pct, r_unit, reason,
            ));
            in_pos = false;
        }
    }
    if in_pos && !rates.is_empty() {
        let last = rates.len().saturating_sub(1);
        let gross = cfg.perp_fraction * rates[entry_idx..=last].iter().sum::<Decimal>();
        trades.push(build_trade(
            entry_idx,
            last,
            timestamps,
            gross,
            fee_pct,
            r_unit,
            CarryExitReason::EndOfSample,
        ));
    }
    Ok(trades)
}
