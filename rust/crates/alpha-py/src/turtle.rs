use crate::common::{build_bars_from_arrays, dec_from_f64, f64_from_dec};
use alpha_core::turtle::{
    evaluate_turtle_entry, evaluate_turtle_exit, TurtleConfig, TurtleVerdict,
};
use alpha_core::turtle_sim::{simulate_turtle_series, SimulatedTurtleTrade};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone, Default)]
pub struct PyTurtleVerdict {
    #[pyo3(get)]
    pub action: String,
    #[pyo3(get)]
    pub entry_price: Option<f64>,
    #[pyo3(get)]
    pub stop_price: Option<f64>,
    #[pyo3(get)]
    pub stop_distance: Option<f64>,
    #[pyo3(get)]
    pub entry_level: Option<f64>,
    #[pyo3(get)]
    pub exit_level: Option<f64>,
    #[pyo3(get)]
    pub exit_price: Option<f64>,
    #[pyo3(get)]
    pub exit_reason: Option<String>,
    #[pyo3(get)]
    pub atr: Option<f64>,
    #[pyo3(get)]
    pub reason: Option<String>,
}

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyTurtleTrade {
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

fn build_config(
    entry_period: usize,
    exit_period: usize,
    atr_period: usize,
    atr_multiplier: f64,
    min_stop_distance: f64,
) -> PyResult<TurtleConfig> {
    TurtleConfig::new(
        entry_period,
        exit_period,
        atr_period,
        dec_from_f64(atr_multiplier, "atr_multiplier")?,
        dec_from_f64(min_stop_distance, "min_stop_distance")?,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))
}

fn map_verdict(v: TurtleVerdict) -> PyResult<PyTurtleVerdict> {
    let mut out = PyTurtleVerdict::default();
    match v {
        TurtleVerdict::EnterLong {
            entry_price,
            stop_price,
            stop_distance,
            entry_level,
            atr,
        } => {
            out.action = "ENTER_LONG".to_string();
            out.entry_price = Some(f64_from_dec(entry_price, "entry_price")?);
            out.stop_price = Some(f64_from_dec(stop_price, "stop_price")?);
            out.stop_distance = Some(f64_from_dec(stop_distance, "stop_distance")?);
            out.entry_level = Some(f64_from_dec(entry_level, "entry_level")?);
            out.atr = Some(f64_from_dec(atr, "atr")?);
        }
        TurtleVerdict::ExitLong { exit_price, reason } => {
            out.action = "EXIT_LONG".to_string();
            out.exit_price = Some(f64_from_dec(exit_price, "exit_price")?);
            out.exit_reason = Some(format!("{:?}", reason));
        }
        TurtleVerdict::Hold {
            stop_price,
            exit_level,
        } => {
            out.action = "HOLD".to_string();
            out.stop_price = Some(f64_from_dec(stop_price, "stop_price")?);
            out.exit_level = Some(f64_from_dec(exit_level, "exit_level")?);
        }
        TurtleVerdict::Abstain { reason } => {
            out.action = "ABSTAIN".to_string();
            out.reason = Some(reason.to_string());
        }
    }
    Ok(out)
}

fn map_trade(t: SimulatedTurtleTrade) -> PyResult<PyTurtleTrade> {
    Ok(PyTurtleTrade {
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
pub fn evaluate_turtle_entry_py(
    timestamps: Vec<i64>,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    entry_period: usize,
    exit_period: usize,
    atr_period: usize,
    atr_multiplier: f64,
    min_stop_distance: f64,
) -> PyResult<PyTurtleVerdict> {
    let bars = build_bars_from_arrays(&timestamps, &opens, &highs, &lows, &closes)?;
    let cfg = build_config(
        entry_period,
        exit_period,
        atr_period,
        atr_multiplier,
        min_stop_distance,
    )?;
    let verdict =
        evaluate_turtle_entry(&bars, &cfg).map_err(|e| PyValueError::new_err(e.to_string()))?;
    map_verdict(verdict)
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_turtle_exit_py(
    timestamps: Vec<i64>,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    current_stop: f64,
    entry_period: usize,
    exit_period: usize,
    atr_period: usize,
    atr_multiplier: f64,
    min_stop_distance: f64,
) -> PyResult<PyTurtleVerdict> {
    let bars = build_bars_from_arrays(&timestamps, &opens, &highs, &lows, &closes)?;
    let cfg = build_config(
        entry_period,
        exit_period,
        atr_period,
        atr_multiplier,
        min_stop_distance,
    )?;
    let dec_stop = dec_from_f64(current_stop, "current_stop")?;
    let verdict = evaluate_turtle_exit(&bars, dec_stop, &cfg)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
    map_verdict(verdict)
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn simulate_turtle_series_py(
    timestamps: Vec<i64>,
    opens: Vec<f64>,
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    entry_period: usize,
    exit_period: usize,
    atr_period: usize,
    atr_multiplier: f64,
    min_stop_distance: f64,
    c_rt: f64,
) -> PyResult<Vec<PyTurtleTrade>> {
    let bars = build_bars_from_arrays(&timestamps, &opens, &highs, &lows, &closes)?;
    let cfg = build_config(
        entry_period,
        exit_period,
        atr_period,
        atr_multiplier,
        min_stop_distance,
    )?;
    let dec_crt = dec_from_f64(c_rt, "c_rt")?;
    let trades = simulate_turtle_series(&bars, &cfg, dec_crt)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let mut out = Vec::with_capacity(trades.len());
    for t in trades {
        out.push(map_trade(t)?);
    }
    Ok(out)
}
