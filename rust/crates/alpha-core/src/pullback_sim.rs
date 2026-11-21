use crate::error::CoreError;
use crate::pullback::{
    evaluate_pullback_bar_exit, PullbackConfig, PullbackExitReason, PullbackExitState,
    PullbackVerdict,
};
use crate::series::{calc_atr_series, calc_ma_series, calc_rsi_series};
use alpha_types::Bar;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulatedPullbackTrade {
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
    pub exit_reason: PullbackExitReason,
}

#[inline]
#[allow(clippy::too_many_arguments)]
fn build_trade(
    e_i: usize,
    x_i: usize,
    bars: &[Bar],
    e_p: Decimal,
    x_p: Decimal,
    d_s: Decimal,
    c_rt: Decimal,
    reason: PullbackExitReason,
) -> Option<SimulatedPullbackTrade> {
    let e_b = bars.get(e_i)?;
    let x_b = bars.get(x_i)?;
    let gross_ret = (x_p - e_p) / e_p;
    let net_ret = gross_ret - c_rt;
    let (g_r, f_r, n_r) = if d_s > Decimal::ZERO {
        (gross_ret / d_s, c_rt / d_s, net_ret / d_s)
    } else {
        (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO)
    };
    Some(SimulatedPullbackTrade {
        entry_idx: e_i,
        exit_idx: x_i,
        entry_time_ms: e_b.open_time_ms,
        exit_time_ms: x_b.close_time_ms,
        entry_price: e_p,
        exit_price: x_p,
        initial_stop: e_p * (Decimal::ONE - d_s),
        bars_held: x_i.saturating_sub(e_i),
        gross_return_pct: gross_ret,
        fee_pct_total: c_rt,
        net_return_pct: net_ret,
        gross_pnl_r: g_r,
        fee_r: f_r,
        net_pnl_r: n_r,
        exit_reason: reason,
    })
}

pub fn simulate_pullback_series(
    bars: &[Bar],
    cfg: &PullbackConfig,
    cost_round_trip: Decimal,
) -> Result<Vec<SimulatedPullbackTrade>, CoreError> {
    let n = bars.len();
    if n <= cfg.trend_sma_period {
        return Ok(Vec::new());
    }
    let closes: Vec<Decimal> = bars.iter().map(|b| b.close).collect();
    let opens: Vec<Decimal> = bars.iter().map(|b| b.open).collect();
    let sma = calc_ma_series(&closes, cfg.trend_sma_period, false);
    let rsi = calc_rsi_series(&closes, cfg.rsi_period);
    let atr = calc_atr_series(bars, cfg.atr_period);
    let ema = calc_ma_series(&closes, cfg.trail_ema_period, true);
    let mut trades = Vec::new();
    let mut in_pos = false;
    let (mut e_p, mut s_p, mut d_s, mut h_p, mut e_i, mut rch) = (
        Decimal::ZERO,
        Decimal::ZERO,
        Decimal::ZERO,
        Decimal::ZERO,
        0,
        false,
    );

    for i in cfg.trend_sma_period..(n - 1) {
        if !in_pos {
            if closes[i] > sma[i] && rsi[i] <= cfg.rsi_entry_threshold && atr[i] > Decimal::ZERO {
                let nxt_o = opens[i + 1];
                let a = atr[i];
                d_s = cfg.min_stop_distance.max((cfg.k_stop * a) / nxt_o);
                e_p = nxt_o;
                s_p = e_p * (Decimal::ONE - d_s);
                h_p = nxt_o;
                e_i = i + 1;
                rch = false;
                in_pos = true;
            }
            continue;
        }
        let nxt_o = if i + 1 < n { opens[i + 1] } else { closes[i] };
        let bar = match bars.get(i) {
            Some(b) => b,
            None => continue,
        };
        let st = PullbackExitState {
            entry_price: e_p,
            stop_price: s_p,
            highest_price: h_p,
            stop_distance: d_s,
            ratcheted: rch,
            bars_held: i.saturating_sub(e_i),
            ema_val: ema[i],
            next_open: nxt_o,
        };
        let is_last = i == n - 2;
        match evaluate_pullback_bar_exit(bar, &st, cfg) {
            PullbackVerdict::ExitLong { exit_price, reason } => {
                let x_i = if reason == PullbackExitReason::StopLoss {
                    i
                } else {
                    (i + 1).min(n - 1)
                };
                if let Some(t) = build_trade(
                    e_i,
                    x_i,
                    bars,
                    e_p,
                    exit_price,
                    d_s,
                    cost_round_trip,
                    reason,
                ) {
                    trades.push(t);
                }
                in_pos = false;
            }
            PullbackVerdict::Hold {
                stop_price,
                highest_price,
                ratcheted,
            } => {
                if is_last {
                    if let Some(t) = build_trade(
                        e_i,
                        i,
                        bars,
                        e_p,
                        closes[i],
                        d_s,
                        cost_round_trip,
                        PullbackExitReason::EndOfData,
                    ) {
                        trades.push(t);
                    }
                    in_pos = false;
                } else {
                    s_p = stop_price;
                    h_p = highest_price;
                    rch = ratcheted;
                }
            }
            PullbackVerdict::EnterLong { .. } | PullbackVerdict::Abstain { .. } => {}
        }
    }
    Ok(trades)
}
