use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use rust_decimal::prelude::{FromPrimitive, ToPrimitive};
use rust_decimal::Decimal;

pub fn dec_from_f64(val: f64, name: &'static str) -> PyResult<Decimal> {
    Decimal::from_f64(val).ok_or_else(|| {
        PyValueError::new_err(format!("invalid float for Decimal conversion: {name}"))
    })
}

pub fn f64_from_dec(val: Decimal, name: &'static str) -> PyResult<f64> {
    val.to_f64().ok_or_else(|| {
        PyValueError::new_err(format!("invalid Decimal for float conversion: {name}"))
    })
}
