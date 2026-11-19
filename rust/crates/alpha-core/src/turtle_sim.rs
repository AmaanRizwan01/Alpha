use crate::error::CoreError;
use crate::turtle::{
    evaluate_turtle_entry, evaluate_turtle_exit, TurtleConfig, TurtleExitReason, TurtleVerdict,
};
use alpha_types::Bar;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulatedTurtleTrade {
    pub entry_idx: usize,
    pub exit_idx: usize,
    pub entry_time_ms: i64,
    pub exit_time_ms: i64,
    pub entry_price: Decimal,
    pub exit_price: Decimal,
    pub initial_stop: Decimal,
    pub bars_held: usize,
    pub gross_return_pct: Decimal,
    pub fee_pct_total: Decimal,
    pub net_return_pct: Decimal,
    pub gross_pnl_r: Decimal,
    pub fee_r: Decimal,
    pub net_pnl_r: Decimal,
    pub exit_reason: TurtleExitReason,
}

#[inline]
#[allow(clippy::too_many_arguments)]
fn build_turtle_trade(
    entry_idx: usize,
    exit_idx: usize,
    bars: &[Bar],
    entry_p: Decimal,
    exit_p: Decimal,
    init_stop: Decimal,
    c_rt: Decimal,
    reason: TurtleExitReason,
) -> Option<SimulatedTurtleTrade> {
    let t_entry = bars.get(entry_idx)?.open_time_ms;
    let t_exit = bars.get(exit_idx)?.close_time_ms;
    let risk_d = entry_p - init_stop;
    if entry_p <= Decimal::ZERO || risk_d <= Decimal::ZERO {
        return None;
    }

    let gross_ret = (exit_p - entry_p) / entry_p;
    let net_ret = gross_ret - c_rt;
    let gross_r = (exit_p - entry_p) / risk_d;
    let fee_r = (entry_p * c_rt) / risk_d;
    let net_r = gross_r - fee_r;

    Some(SimulatedTurtleTrade {
        entry_idx,
        exit_idx,
        entry_time_ms: t_entry,
        exit_time_ms: t_exit,
        entry_price: entry_p,
        exit_price: exit_p,
        initial_stop: init_stop,
        bars_held: exit_idx.saturating_sub(entry_idx),
        gross_return_pct: gross_ret,
        fee_pct_total: c_rt,
        net_return_pct: net_ret,
        gross_pnl_r: gross_r,
        fee_r,
        net_pnl_r: net_r,
        exit_reason: reason,
    })
}

pub fn simulate_turtle_series(
    bars: &[Bar],
    config: &TurtleConfig,
    c_rt: Decimal,
) -> Result<Vec<SimulatedTurtleTrade>, CoreError> {
    let min_req = config.entry_period.max(config.atr_period + 1) + 1;
    if bars.len() < min_req {
        return Ok(Vec::new());
    }

    let mut trades = Vec::new();
    let mut in_pos = false;
    let mut entry_idx = 0;
    let mut entry_p = Decimal::ZERO;
    let mut fixed_stop = Decimal::ZERO;

    for i in min_req..=bars.len() {
        let prefix = bars.get(..i).ok_or(CoreError::InsufficientBars {
            required: i,
            provided: bars.len(),
        })?;
        let curr_idx = i - 1;

        if in_pos {
            match evaluate_turtle_exit(prefix, fixed_stop, config)? {
                TurtleVerdict::ExitLong { exit_price, reason } => {
                    if let Some(t) = build_turtle_trade(
                        entry_idx, curr_idx, bars, entry_p, exit_price, fixed_stop, c_rt, reason,
                    ) {
                        trades.push(t);
                    }
                    in_pos = false;
                }
                TurtleVerdict::Hold { .. }
                | TurtleVerdict::EnterLong { .. }
                | TurtleVerdict::Abstain { .. } => {}
            }
        } else {
            match evaluate_turtle_entry(prefix, config)? {
                TurtleVerdict::EnterLong {
                    entry_price,
                    stop_price,
                    ..
                } => {
                    entry_p = entry_price;
                    fixed_stop = stop_price;
                    entry_idx = curr_idx;
                    in_pos = true;
                }
                TurtleVerdict::Abstain { .. }
                | TurtleVerdict::ExitLong { .. }
                | TurtleVerdict::Hold { .. } => {}
            }
        }
    }

    Ok(trades)
}
