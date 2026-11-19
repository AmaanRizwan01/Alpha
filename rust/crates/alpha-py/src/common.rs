use alpha_types::Bar;
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

pub fn build_bars_from_arrays(
    timestamps: &[i64],
    opens: &[f64],
    highs: &[f64],
    lows: &[f64],
    closes: &[f64],
) -> PyResult<Vec<Bar>> {
    let n = timestamps
        .len()
        .min(opens.len())
        .min(highs.len())
        .min(lows.len())
        .min(closes.len());
    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let t = *timestamps.get(i).unwrap_or(&0);
        let o = dec_from_f64(*opens.get(i).unwrap_or(&0.0), "open")?;
        let h = dec_from_f64(*highs.get(i).unwrap_or(&0.0), "high")?;
        let l = dec_from_f64(*lows.get(i).unwrap_or(&0.0), "low")?;
        let c = dec_from_f64(*closes.get(i).unwrap_or(&0.0), "close")?;
        let bar = Bar::new(
            t,
            t + 86_400_000,
            o,
            h,
            l,
            c,
            Decimal::ONE,
            Decimal::ZERO,
            c,
            1,
        )
        .map_err(|e| PyValueError::new_err(e.to_string()))?;
        bars.push(bar);
    }
    Ok(bars)
}
