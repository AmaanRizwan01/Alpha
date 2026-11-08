#![allow(unsafe_op_in_unsafe_fn, clippy::useless_conversion)]

mod common;
pub mod exits;
pub mod strategy;

use common::{dec_from_f64, f64_from_dec};
use exits::{calc_initial_stop, simulate_trade_exit, PyExitResult};
use strategy::{evaluate_h1, scan_h1_signals, PyH1Result};

use alpha_core::{
    average_true_range, calculate_position_size, donchian_channel, kaufman_efficiency_ratio,
    ChopFilter, ChopVerdict, ExchangeFilter, FeeBudget,
};
use alpha_types::Bar;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rust_decimal::Decimal;

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyPositionSize {
    #[pyo3(get)]
    pub qty: f64,
    #[pyo3(get)]
    pub notional: f64,
    #[pyo3(get)]
    pub rounded_entry: f64,
    #[pyo3(get)]
    pub rounded_stop: f64,
    #[pyo3(get)]
    pub stop_distance: f64,
    #[pyo3(get)]
    pub stop_distance_pct: f64,
    #[pyo3(get)]
    pub risk_quote: f64,
}

#[pyfunction]
pub fn kaufman_er(closes: Vec<f64>, period: usize) -> PyResult<f64> {
    let mut dec_closes = Vec::with_capacity(closes.len());
    for &c in &closes {
        dec_closes.push(dec_from_f64(c, "close")?);
    }
    kaufman_efficiency_ratio(&dec_closes, period).map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pyfunction]
pub fn atr(highs: Vec<f64>, lows: Vec<f64>, closes: Vec<f64>, period: usize) -> PyResult<f64> {
    let n = highs.len().min(lows.len()).min(closes.len());
    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let h = dec_from_f64(*highs.get(i).unwrap_or(&0.0), "high")?;
        let l = dec_from_f64(*lows.get(i).unwrap_or(&0.0), "low")?;
        let c = dec_from_f64(*closes.get(i).unwrap_or(&0.0), "close")?;
        let b = Bar::new(
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
        bars.push(b);
    }
    let res =
        average_true_range(&bars, period).map_err(|e| PyValueError::new_err(e.to_string()))?;
    f64_from_dec(res, "atr")
}

#[pyfunction]
pub fn donchian(highs: Vec<f64>, lows: Vec<f64>, period: usize) -> PyResult<(f64, f64)> {
    let n = highs.len().min(lows.len());
    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let h = dec_from_f64(*highs.get(i).unwrap_or(&0.0), "high")?;
        let l = dec_from_f64(*lows.get(i).unwrap_or(&0.0), "low")?;
        let b = Bar::new(
            i as i64,
            (i + 1) as i64,
            l,
            h,
            l,
            l,
            Decimal::ONE,
            Decimal::ZERO,
            l,
            1,
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
        bars.push(b);
    }
    let (hh, ll) =
        donchian_channel(&bars, period).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok((f64_from_dec(hh, "hh")?, f64_from_dec(ll, "ll")?))
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn calc_position_size(
    equity: f64,
    risk_fraction: f64,
    raw_entry: f64,
    raw_stop: f64,
    maker_fee: f64,
    taker_fee: f64,
    slippage: f64,
    tick_size: f64,
    step_size: f64,
    min_qty: f64,
    min_notional: f64,
    min_stop_pct: f64,
) -> PyResult<PyPositionSize> {
    let fees = FeeBudget::new(
        dec_from_f64(maker_fee, "maker_fee")?,
        dec_from_f64(taker_fee, "taker_fee")?,
        dec_from_f64(slippage, "slippage")?,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let filter = ExchangeFilter::new(
        dec_from_f64(tick_size, "tick_size")?,
        dec_from_f64(step_size, "step_size")?,
        dec_from_f64(min_qty, "min_qty")?,
        dec_from_f64(min_notional, "min_notional")?,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let size = calculate_position_size(
        dec_from_f64(equity, "equity")?,
        dec_from_f64(risk_fraction, "risk_fraction")?,
        dec_from_f64(raw_entry, "raw_entry")?,
        dec_from_f64(raw_stop, "raw_stop")?,
        &fees,
        &filter,
        dec_from_f64(min_stop_pct, "min_stop_pct")?,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    Ok(PyPositionSize {
        qty: f64_from_dec(size.qty, "qty")?,
        notional: f64_from_dec(size.notional, "notional")?,
        rounded_entry: f64_from_dec(size.rounded_entry, "rounded_entry")?,
        rounded_stop: f64_from_dec(size.rounded_stop, "rounded_stop")?,
        stop_distance: f64_from_dec(size.stop_distance, "stop_distance")?,
        stop_distance_pct: f64_from_dec(size.stop_distance_pct, "stop_distance_pct")?,
        risk_quote: f64_from_dec(size.risk_quote, "risk_quote")?,
    })
}

#[pyfunction]
pub fn evaluate_chop(
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    er_period: usize,
    er_threshold: f64,
    atr_period: usize,
    min_atr_pct: f64,
) -> PyResult<Option<String>> {
    let filter = ChopFilter::new(
        er_period,
        er_threshold,
        atr_period,
        dec_from_f64(min_atr_pct, "min_atr_pct")?,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let n = highs.len().min(lows.len()).min(closes.len());
    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let h = dec_from_f64(*highs.get(i).unwrap_or(&0.0), "high")?;
        let l = dec_from_f64(*lows.get(i).unwrap_or(&0.0), "low")?;
        let c = dec_from_f64(*closes.get(i).unwrap_or(&0.0), "close")?;
        let b = Bar::new(
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
        bars.push(b);
    }

    match filter
        .evaluate(&bars)
        .map_err(|e| PyValueError::new_err(e.to_string()))?
    {
        ChopVerdict::Pass => Ok(None),
        ChopVerdict::AbstainChop { .. } => Ok(Some("AbstainChop".to_string())),
        ChopVerdict::AbstainLowVolatility { .. } => Ok(Some("AbstainLowVolatility".to_string())),
    }
}

#[pymodule]
fn alpha_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPositionSize>()?;
    m.add_class::<PyH1Result>()?;
    m.add_class::<PyExitResult>()?;

    m.add_function(wrap_pyfunction!(kaufman_er, m)?)?;
    m.add_function(wrap_pyfunction!(atr, m)?)?;
    m.add_function(wrap_pyfunction!(donchian, m)?)?;
    m.add_function(wrap_pyfunction!(calc_position_size, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_chop, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_h1, m)?)?;
    m.add_function(wrap_pyfunction!(scan_h1_signals, m)?)?;
    m.add_function(wrap_pyfunction!(calc_initial_stop, m)?)?;
    m.add_function(wrap_pyfunction!(simulate_trade_exit, m)?)?;

    Ok(())
}
