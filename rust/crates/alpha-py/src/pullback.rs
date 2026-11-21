use crate::common::{build_bars_from_arrays, dec_from_f64, f64_from_dec};
use alpha_core::pullback::PullbackConfig;
use alpha_core::pullback_sim::{simulate_pullback_series, SimulatedPullbackTrade};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct PyPullbackVerdict {
    #[pyo3(get)]
    pub action: String,
    #[pyo3(get)]
    pub entry_price: Option<f64>,
    #[pyo3(get)]
    pub stop_price: Option<f64>,
    #[pyo3(get)]
    pub stop_distance: Option<f64>,
    #[pyo3(get)]
    pub exit_price: Option<f64>,
    #[pyo3(get)]
    pub exit_reason: Option<String>,
    #[pyo3(get)]
    pub atr: Option<f64>,
    #[pyo3(get)]
    pub rsi: Option<f64>,
    #[pyo3(get)]
    pub reason: Option<String>,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyPullbackTrade {
    #[pyo3(get)]
    pub entry_idx: usize,
    #[pyo3(get)]
    pub exit_idx: usize,
    #[pyo3(get)]
    pub entry_time_ms: i64,
    #[pyo3(get)]
    pub exit_time_ms: i64,
    #[pyo3(get)]
    pub entry_price: f64,
    #[pyo3(get)]
    pub exit_price: f64,
    #[pyo3(get)]
    pub initial_stop: f64,
    #[pyo3(get)]
    pub bars_held: usize,
    #[pyo3(get)]
    pub gross_return_pct: f64,
    #[pyo3(get)]
    pub fee_pct_total: f64,
    #[pyo3(get)]
    pub net_return_pct: f64,
    #[pyo3(get)]
    pub gross_pnl_r: f64,
    #[pyo3(get)]
    pub fee_r: f64,
    #[pyo3(get)]
    pub net_pnl_r: f64,
    #[pyo3(get)]
    pub exit_reason: String,
}

#[allow(clippy::too_many_arguments)]
fn build_config(
    trend_sma: usize,
    rsi_p: usize,
    rsi_entry: f64,
    atr_p: usize,
    k_stop: f64,
    min_stop: f64,
    r_trig: f64,
    r_lock: f64,
    trail_ema: usize,
    max_hold: usize,
) -> PyResult<PullbackConfig> {
    PullbackConfig::new(
        trend_sma,
        rsi_p,
        dec_from_f64(rsi_entry, "rsi_entry")?,
        atr_p,
        dec_from_f64(k_stop, "k_stop")?,
        dec_from_f64(min_stop, "min_stop")?,
        dec_from_f64(r_trig, "r_trig")?,
        dec_from_f64(r_lock, "r_lock")?,
        trail_ema,
        max_hold,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))
}

fn map_trade(t: SimulatedPullbackTrade) -> PyResult<PyPullbackTrade> {
    Ok(PyPullbackTrade {
        entry_idx: t.entry_idx,
        exit_idx: t.exit_idx,
        entry_time_ms: t.entry_time_ms,
        exit_time_ms: t.exit_time_ms,
        entry_price: f64_from_dec(t.entry_price, "entry_price")?,
        exit_price: f64_from_dec(t.exit_price, "exit_price")?,
        initial_stop: f64_from_dec(t.initial_stop, "initial_stop")?,
        bars_held: t.bars_held,
        gross_return_pct: f64_from_dec(t.gross_return_pct, "gross_return_pct")?,
        fee_pct_total: f64_from_dec(t.fee_pct_total, "fee_pct_total")?,
        net_return_pct: f64_from_dec(t.net_return_pct, "net_return_pct")?,
        gross_pnl_r: f64_from_dec(t.gross_pnl_r, "gross_pnl_r")?,
        fee_r: f64_from_dec(t.fee_r, "fee_r")?,
        net_pnl_r: f64_from_dec(t.net_pnl_r, "net_pnl_r")?,
        exit_reason: format!("{:?}", t.exit_reason),
    })
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn simulate_pullback_series_py(
    timestamps: Vec<i64>,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    trend_sma: usize,
    rsi_p: usize,
    rsi_entry: f64,
    atr_p: usize,
    k_stop: f64,
    min_stop: f64,
    r_trig: f64,
    r_lock: f64,
    trail_ema: usize,
    max_hold: usize,
    c_rt: f64,
) -> PyResult<Vec<PyPullbackTrade>> {
    let bars = build_bars_from_arrays(&timestamps, &opens, &highs, &lows, &closes)?;
    let cfg = build_config(
        trend_sma, rsi_p, rsi_entry, atr_p, k_stop, min_stop, r_trig, r_lock, trail_ema, max_hold,
    )?;
    let dec_crt = dec_from_f64(c_rt, "c_rt")?;
    let trades = simulate_pullback_series(&bars, &cfg, dec_crt)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let mut out = Vec::with_capacity(trades.len());
    for t in trades {
        out.push(map_trade(t)?);
    }
    Ok(out)
}
