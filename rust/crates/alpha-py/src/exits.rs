use crate::common::{dec_from_f64, f64_from_dec};
use alpha_core::{calculate_initial_stop, ActiveTrade, ExitConfig, ExitReason, ExitVerdict};
use alpha_types::Bar;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rust_decimal::Decimal;

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyExitResult {
    #[pyo3(get)]
    pub exit_price: f64,
    #[pyo3(get)]
    pub exit_bar: usize,
    #[pyo3(get)]
    pub reason: String,
    #[pyo3(get)]
    pub bars_held: usize,
    #[pyo3(get)]
    pub pnl_r: f64,
    #[pyo3(get)]
    pub return_pct: f64,
}

#[pyfunction]
pub fn calc_initial_stop(
    entry_price: f64,
    atr: f64,
    k_stop: f64,
    min_stop_distance_pct: f64,
) -> PyResult<f64> {
    let config = ExitConfig {
        k_stop: dec_from_f64(k_stop, "k_stop")?,
        min_stop_distance_pct: dec_from_f64(min_stop_distance_pct, "min_stop_distance_pct")?,
        ..Default::default()
    };

    let p_entry = dec_from_f64(entry_price, "entry_price")?;
    let p_atr = dec_from_f64(atr, "atr")?;

    let stop = calculate_initial_stop(p_entry, p_atr, &config)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    f64_from_dec(stop, "stop")
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn simulate_trade_exit(
    entry_price: f64,
    initial_stop: f64,
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    atrs: Vec<f64>,
    k_stop: f64,
    k_trail: f64,
    min_stop_distance_pct: f64,
    enable_e1_stagnation: bool,
    stagnation_bars: usize,
    enable_e2_breakeven: bool,
) -> PyResult<PyExitResult> {
    let p_entry = dec_from_f64(entry_price, "entry_price")?;
    let p_stop = dec_from_f64(initial_stop, "initial_stop")?;

    let config = ExitConfig {
        k_stop: dec_from_f64(k_stop, "k_stop")?,
        k_trail: dec_from_f64(k_trail, "k_trail")?,
        min_stop_distance_pct: dec_from_f64(min_stop_distance_pct, "min_stop_distance_pct")?,
        enable_e1_stagnation,
        stagnation_bars,
        enable_e2_breakeven,
        ..Default::default()
    };

    let mut trade =
        ActiveTrade::new(p_entry, p_stop).map_err(|e| PyValueError::new_err(e.to_string()))?;

    let n = highs
        .len()
        .min(lows.len())
        .min(closes.len())
        .min(atrs.len());

    let r_unit = p_entry - p_stop;

    for i in 0..n {
        let h = dec_from_f64(*highs.get(i).unwrap_or(&0.0), "high")?;
        let l = dec_from_f64(*lows.get(i).unwrap_or(&0.0), "low")?;
        let c = dec_from_f64(*closes.get(i).unwrap_or(&0.0), "close")?;
        let atr_dec = dec_from_f64(*atrs.get(i).unwrap_or(&0.0), "atr")?;

        let bar = Bar::new(
            i as i64,
            (i + 1) as i64,
            c,
            h,
            l,
            c,
            Decimal::ONE,
            Decimal::ZERO,
            c,
            1,
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

        match trade
            .update_bar(&bar, atr_dec, &config)
            .map_err(|e| PyValueError::new_err(e.to_string()))?
        {
            ExitVerdict::Exit {
                reason,
                exit_price: p_exit,
                bars_held,
            } => {
                let reason_str = match reason {
                    ExitReason::StopLossHit => "StopLossHit",
                    ExitReason::TrailingStopHit => "TrailingStopHit",
                    ExitReason::StagnationTimeStop => "StagnationTimeStop",
                    ExitReason::BreakevenStopHit => "BreakevenStopHit",
                };

                let pnl_r = (p_exit - p_entry) / r_unit;
                let ret_pct = (p_exit - p_entry) / p_entry;

                return Ok(PyExitResult {
                    exit_price: f64_from_dec(p_exit, "exit_price")?,
                    exit_bar: i,
                    reason: reason_str.to_string(),
                    bars_held,
                    pnl_r: f64_from_dec(pnl_r, "pnl_r")?,
                    return_pct: f64_from_dec(ret_pct, "return_pct")?,
                });
            }
            ExitVerdict::Hold { .. } => {}
        }
    }

    let last_close = dec_from_f64(*closes.last().unwrap_or(&entry_price), "last_close")?;
    let pnl_r = (last_close - p_entry) / r_unit;
    let ret_pct = (last_close - p_entry) / p_entry;

    Ok(PyExitResult {
        exit_price: f64_from_dec(last_close, "exit_price")?,
        exit_bar: n.saturating_sub(1),
        reason: "Open".to_string(),
        bars_held: n,
        pnl_r: f64_from_dec(pnl_r, "pnl_r")?,
        return_pct: f64_from_dec(ret_pct, "return_pct")?,
    })
}
