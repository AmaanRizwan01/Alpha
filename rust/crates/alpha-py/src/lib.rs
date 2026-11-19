#![allow(unsafe_op_in_unsafe_fn, clippy::useless_conversion)]

pub mod carry;
mod common;
pub mod exits;
pub mod indicators;
pub mod pullback;
pub mod strategy;
pub mod turtle;

use carry::{
    calc_carry_apr, calc_carry_sizing, check_adl_desync_py, simulate_carry_arbitrage,
    PyCarrySizing, PyCarryTrade,
};
use exits::{calc_initial_stop, simulate_trade_exit, PyExitResult};
use indicators::{atr, calc_position_size, donchian, evaluate_chop, kaufman_er, PyPositionSize};
use pullback::{simulate_pullback_series_py, PyPullbackTrade, PyPullbackVerdict};
use strategy::{evaluate_h1, scan_h1_signals, PyH1Result};
use turtle::{
    evaluate_turtle_entry_py, evaluate_turtle_exit_py, simulate_turtle_series_py, PyTurtleTrade,
    PyTurtleVerdict,
};

use pyo3::prelude::*;

#[pymodule]
fn alpha_py(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPositionSize>()?;
    m.add_class::<PyH1Result>()?;
    m.add_class::<PyExitResult>()?;
    m.add_class::<PyCarryTrade>()?;
    m.add_class::<PyCarrySizing>()?;
    m.add_class::<PyTurtleVerdict>()?;
    m.add_class::<PyTurtleTrade>()?;
    m.add_class::<PyPullbackVerdict>()?;
    m.add_class::<PyPullbackTrade>()?;

    m.add_function(wrap_pyfunction!(kaufman_er, m)?)?;
    m.add_function(wrap_pyfunction!(atr, m)?)?;
    m.add_function(wrap_pyfunction!(donchian, m)?)?;
    m.add_function(wrap_pyfunction!(calc_position_size, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_chop, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_h1, m)?)?;
    m.add_function(wrap_pyfunction!(scan_h1_signals, m)?)?;
    m.add_function(wrap_pyfunction!(calc_initial_stop, m)?)?;
    m.add_function(wrap_pyfunction!(simulate_trade_exit, m)?)?;
    m.add_function(wrap_pyfunction!(calc_carry_apr, m)?)?;
    m.add_function(wrap_pyfunction!(simulate_carry_arbitrage, m)?)?;
    m.add_function(wrap_pyfunction!(calc_carry_sizing, m)?)?;
    m.add_function(wrap_pyfunction!(check_adl_desync_py, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_turtle_entry_py, m)?)?;
    m.add_function(wrap_pyfunction!(evaluate_turtle_exit_py, m)?)?;
    m.add_function(wrap_pyfunction!(simulate_turtle_series_py, m)?)?;
    m.add_function(wrap_pyfunction!(simulate_pullback_series_py, m)?)?;

    Ok(())
}
