use crate::carry::{calc_funding_apr_series_custom, FundingCarryConfig};
use crate::error::CoreError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

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
    if r_unit <= Decimal::ZERO {
        return Err(CoreError::InvalidParameter {
            name: "r_unit",
            reason: "r_unit must be strictly positive",
        });
    }
    if rates.len() != timestamps.len() {
        return Err(CoreError::InvalidParameter {
            name: "timestamps",
            reason: "rates and timestamps length mismatch",
        });
    }
    if rates.is_empty() {
        return Ok(Vec::new());
    }
    let apr = calc_funding_apr_series_custom(rates, cfg.ema_alpha, cfg.cycles_per_year);
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
        let gross_pct = match rates.get(entry_idx..=i) {
            Some(slice) => cfg.perp_fraction * slice.iter().sum::<Decimal>(),
            None => Decimal::ZERO,
        };
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
        let gross = match rates.get(entry_idx..=last) {
            Some(slice) => cfg.perp_fraction * slice.iter().sum::<Decimal>(),
            None => Decimal::ZERO,
        };
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
