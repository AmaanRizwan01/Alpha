use crate::common::{dec_from_f64, f64_from_dec};
use alpha_core::carry::{
    calc_funding_apr_series, check_adl_desync, simulate_carry_series, CarryExitReason,
    FundingCarryConfig,
};
use alpha_core::carry_sizing::{calculate_carry_sizing, CarrySizingVerdict};
use alpha_core::sizing::ExchangeFilter;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyCarryTrade {
    #[pyo3(get)]
    pub entry_idx: usize,
    #[pyo3(get)]
    pub exit_idx: usize,
    #[pyo3(get)]
    pub entry_time_ms: i64,
    #[pyo3(get)]
    pub exit_time_ms: i64,
    #[pyo3(get)]
    pub epochs_held: usize,
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

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyCarrySizing {
    #[pyo3(get)]
    pub spot_qty: f64,
    #[pyo3(get)]
    pub perp_qty: f64,
    #[pyo3(get)]
    pub spot_notional: f64,
    #[pyo3(get)]
    pub perp_notional: f64,
    #[pyo3(get)]
    pub spot_capital: f64,
    #[pyo3(get)]
    pub margin_capital: f64,
    #[pyo3(get)]
    pub total_capital: f64,
}

#[pyfunction]
pub fn calc_carry_apr(rates: Vec<f64>, alpha: f64) -> PyResult<Vec<f64>> {
    let mut dec_rates = Vec::with_capacity(rates.len());
    for &r in &rates {
        dec_rates.push(dec_from_f64(r, "rate")?);
    }
    let dec_alpha = dec_from_f64(alpha, "alpha")?;
    let apr_series = calc_funding_apr_series(&dec_rates, dec_alpha);

    let mut out = Vec::with_capacity(apr_series.len());
    for a in apr_series {
        out.push(f64_from_dec(a, "apr")?);
    }
    Ok(out)
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn simulate_carry_arbitrage(
    rates: Vec<f64>,
    timestamps: Vec<i64>,
    entry_apr: f64,
    exit_apr: f64,
    ema_alpha: f64,
    inversion_stop: f64,
    round_trip_friction: f64,
    fee_hurdle_mult: f64,
    r_unit: f64,
) -> PyResult<Vec<PyCarryTrade>> {
    let cfg = FundingCarryConfig::new(
        dec_from_f64(entry_apr, "entry_apr")?,
        dec_from_f64(exit_apr, "exit_apr")?,
        dec_from_f64(ema_alpha, "ema_alpha")?,
        dec_from_f64(inversion_stop, "inversion_stop")?,
        dec_from_f64(round_trip_friction, "round_trip_friction")?,
        dec_from_f64(fee_hurdle_mult, "fee_hurdle_mult")?,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let mut dec_rates = Vec::with_capacity(rates.len());
    for &r in &rates {
        dec_rates.push(dec_from_f64(r, "rate")?);
    }
    let dec_r_unit = dec_from_f64(r_unit, "r_unit")?;

    let rust_trades = simulate_carry_series(&cfg, &dec_rates, &timestamps, dec_r_unit)
        .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let mut py_trades = Vec::with_capacity(rust_trades.len());
    for t in rust_trades {
        let reason_str = match t.exit_reason {
            CarryExitReason::FundingInversionStop => "FundingInversionStop",
            CarryExitReason::FundingDecayExit => "FundingDecayExit",
            CarryExitReason::PredictivePreExit => "PredictivePreExit",
            CarryExitReason::AdlDesyncEmergency => "AdlDesyncEmergency",
            CarryExitReason::EndOfSample => "EndOfSample",
        };
        py_trades.push(PyCarryTrade {
            entry_idx: t.entry_idx,
            exit_idx: t.exit_idx,
            entry_time_ms: t.entry_time_ms,
            exit_time_ms: t.exit_time_ms,
            epochs_held: t.epochs_held,
            gross_return_pct: f64_from_dec(t.gross_return_pct, "gross_return_pct")?,
            fee_pct_total: f64_from_dec(t.fee_pct_total, "fee_pct_total")?,
            net_return_pct: f64_from_dec(t.net_return_pct, "net_return_pct")?,
            gross_pnl_r: f64_from_dec(t.gross_pnl_r, "gross_pnl_r")?,
            fee_r: f64_from_dec(t.fee_r, "fee_r")?,
            net_pnl_r: f64_from_dec(t.net_pnl_r, "net_pnl_r")?,
            exit_reason: reason_str.to_string(),
        });
    }
    Ok(py_trades)
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn calc_carry_sizing(
    capital: f64,
    spot_price: f64,
    perp_price: f64,
    tick_size: f64,
    step_size: f64,
    min_qty: f64,
    min_notional: f64,
) -> PyResult<Option<PyCarrySizing>> {
    let filter = ExchangeFilter::new(
        dec_from_f64(tick_size, "tick_size")?,
        dec_from_f64(step_size, "step_size")?,
        dec_from_f64(min_qty, "min_qty")?,
        dec_from_f64(min_notional, "min_notional")?,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    let verdict = calculate_carry_sizing(
        dec_from_f64(capital, "capital")?,
        dec_from_f64(spot_price, "spot_price")?,
        dec_from_f64(perp_price, "perp_price")?,
        &filter,
        &filter,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))?;

    match verdict {
        CarrySizingVerdict::Approved(s) => Ok(Some(PyCarrySizing {
            spot_qty: f64_from_dec(s.spot_qty, "spot_qty")?,
            perp_qty: f64_from_dec(s.perp_qty, "perp_qty")?,
            spot_notional: f64_from_dec(s.spot_notional, "spot_notional")?,
            perp_notional: f64_from_dec(s.perp_notional, "perp_notional")?,
            spot_capital: f64_from_dec(s.spot_capital, "spot_capital")?,
            margin_capital: f64_from_dec(s.margin_capital, "margin_capital")?,
            total_capital: f64_from_dec(s.total_capital, "total_capital")?,
        })),
        _ => Ok(None),
    }
}

#[pyfunction]
pub fn check_adl_desync_py(spot_qty: f64, perp_qty: f64, tolerance: f64) -> PyResult<bool> {
    let s = dec_from_f64(spot_qty, "spot_qty")?;
    let p = dec_from_f64(perp_qty, "perp_qty")?;
    let tol = dec_from_f64(tolerance, "tolerance")?;
    Ok(check_adl_desync(s, p, tol))
}
