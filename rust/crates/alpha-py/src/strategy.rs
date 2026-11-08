use crate::common::{dec_from_f64, f64_from_dec};
use alpha_core::{evaluate_h1_signal, H1Config, H1Verdict};
use alpha_types::Bar;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;

#[pyclass]
#[derive(Debug, Clone)]
pub struct PyH1Result {
    #[pyo3(get)]
    pub is_signal: bool,
    #[pyo3(get)]
    pub verdict: String,
    #[pyo3(get)]
    pub close: f64,
    #[pyo3(get)]
    pub upper_band: f64,
    #[pyo3(get)]
    pub er: f64,
    #[pyo3(get)]
    pub volume: f64,
    #[pyo3(get)]
    pub volume_sma: f64,
    #[pyo3(get)]
    pub taker_buy_ratio: f64,
}

#[allow(clippy::too_many_arguments)]
pub fn build_h1_config(
    er_period: usize,
    er_threshold: f64,
    donchian_period: usize,
    volume_sma_period: usize,
    volume_multiplier: f64,
    min_taker_buy_ratio: f64,
) -> PyResult<H1Config> {
    H1Config::new(
        er_period,
        er_threshold,
        donchian_period,
        volume_sma_period,
        dec_from_f64(volume_multiplier, "volume_multiplier")?,
        dec_from_f64(min_taker_buy_ratio, "min_taker_buy_ratio")?,
    )
    .map_err(|e| PyValueError::new_err(e.to_string()))
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn evaluate_h1(
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    volumes: Vec<f64>,
    taker_volumes: Vec<f64>,
    er_period: usize,
    er_threshold: f64,
    donchian_period: usize,
    volume_sma_period: usize,
    volume_multiplier: f64,
    min_taker_buy_ratio: f64,
) -> PyResult<PyH1Result> {
    let n = highs
        .len()
        .min(lows.len())
        .min(closes.len())
        .min(volumes.len())
        .min(taker_volumes.len());

    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let h = dec_from_f64(*highs.get(i).unwrap_or(&0.0), "high")?;
        let l = dec_from_f64(*lows.get(i).unwrap_or(&0.0), "low")?;
        let c = dec_from_f64(*closes.get(i).unwrap_or(&0.0), "close")?;
        let v = dec_from_f64(*volumes.get(i).unwrap_or(&0.0), "volume")?;
        let tv = dec_from_f64(*taker_volumes.get(i).unwrap_or(&0.0), "taker_volume")?;

        let bar = Bar::new(i as i64, (i + 1) as i64, c, h, l, c, v, tv, v * c, 1)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        bars.push(bar);
    }

    let config = build_h1_config(
        er_period,
        er_threshold,
        donchian_period,
        volume_sma_period,
        volume_multiplier,
        min_taker_buy_ratio,
    )?;

    let verdict =
        evaluate_h1_signal(&bars, &config).map_err(|e| PyValueError::new_err(e.to_string()))?;

    match verdict {
        H1Verdict::Signal {
            close,
            upper_band,
            er,
            volume,
            volume_sma,
            taker_buy_ratio,
            ..
        } => Ok(PyH1Result {
            is_signal: true,
            verdict: "Signal".to_string(),
            close: f64_from_dec(close, "close")?,
            upper_band: f64_from_dec(upper_band, "upper_band")?,
            er,
            volume: f64_from_dec(volume, "volume")?,
            volume_sma: f64_from_dec(volume_sma, "volume_sma")?,
            taker_buy_ratio: f64_from_dec(taker_buy_ratio, "taker_buy_ratio")?,
        }),
        H1Verdict::AbstainChop { er, .. } => Ok(PyH1Result {
            is_signal: false,
            verdict: "AbstainChop".to_string(),
            close: 0.0,
            upper_band: 0.0,
            er,
            volume: 0.0,
            volume_sma: 0.0,
            taker_buy_ratio: 0.0,
        }),
        H1Verdict::AbstainNoBreakout { close, upper_band } => Ok(PyH1Result {
            is_signal: false,
            verdict: "AbstainNoBreakout".to_string(),
            close: f64_from_dec(close, "close")?,
            upper_band: f64_from_dec(upper_band, "upper_band")?,
            er: 0.0,
            volume: 0.0,
            volume_sma: 0.0,
            taker_buy_ratio: 0.0,
        }),
        H1Verdict::AbstainLowVolume {
            volume,
            required_volume,
        } => Ok(PyH1Result {
            is_signal: false,
            verdict: "AbstainLowVolume".to_string(),
            close: 0.0,
            upper_band: 0.0,
            er: 0.0,
            volume: f64_from_dec(volume, "volume")?,
            volume_sma: f64_from_dec(required_volume, "required_volume")?,
            taker_buy_ratio: 0.0,
        }),
        H1Verdict::AbstainLowTakerRatio { ratio, .. } => Ok(PyH1Result {
            is_signal: false,
            verdict: "AbstainLowTakerRatio".to_string(),
            close: 0.0,
            upper_band: 0.0,
            er: 0.0,
            volume: 0.0,
            volume_sma: 0.0,
            taker_buy_ratio: f64_from_dec(ratio, "taker_buy_ratio")?,
        }),
    }
}

#[pyfunction]
#[allow(clippy::too_many_arguments)]
pub fn scan_h1_signals(
    highs: Vec<f64>,
    lows: Vec<f64>,
    closes: Vec<f64>,
    volumes: Vec<f64>,
    taker_volumes: Vec<f64>,
    er_period: usize,
    er_threshold: f64,
    donchian_period: usize,
    volume_sma_period: usize,
    volume_multiplier: f64,
    min_taker_buy_ratio: f64,
) -> PyResult<Vec<bool>> {
    let n = highs
        .len()
        .min(lows.len())
        .min(closes.len())
        .min(volumes.len())
        .min(taker_volumes.len());

    let mut bars = Vec::with_capacity(n);
    for i in 0..n {
        let h = dec_from_f64(*highs.get(i).unwrap_or(&0.0), "high")?;
        let l = dec_from_f64(*lows.get(i).unwrap_or(&0.0), "low")?;
        let c = dec_from_f64(*closes.get(i).unwrap_or(&0.0), "close")?;
        let v = dec_from_f64(*volumes.get(i).unwrap_or(&0.0), "volume")?;
        let tv = dec_from_f64(*taker_volumes.get(i).unwrap_or(&0.0), "taker_volume")?;

        let bar = Bar::new(i as i64, (i + 1) as i64, c, h, l, c, v, tv, v * c, 1)
            .map_err(|e| PyValueError::new_err(e.to_string()))?;
        bars.push(bar);
    }

    let config = build_h1_config(
        er_period,
        er_threshold,
        donchian_period,
        volume_sma_period,
        volume_multiplier,
        min_taker_buy_ratio,
    )?;

    let req_bars = config
        .er_period
        .max(config.donchian_period)
        .max(config.volume_sma_period)
        + 1;

    let mut signals = vec![false; n];
    for end_idx in req_bars..=n {
        if let Some(window) = bars.get(..end_idx) {
            if let Ok(H1Verdict::Signal { .. }) = evaluate_h1_signal(window, &config) {
                if let Some(slot) = signals.get_mut(end_idx - 1) {
                    *slot = true;
                }
            }
        }
    }

    Ok(signals)
}
