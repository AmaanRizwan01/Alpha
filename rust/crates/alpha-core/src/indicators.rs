use crate::error::CoreError;
use alpha_types::Bar;
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::Decimal;

pub fn kaufman_efficiency_ratio(closes: &[Decimal], period: usize) -> Result<f64, CoreError> {
    if period == 0 {
        return Err(CoreError::InvalidParameter {
            name: "period",
            reason: "period must be greater than zero",
        });
    }
    if closes.len() <= period {
        return Err(CoreError::InsufficientBars {
            required: period + 1,
            provided: closes.len(),
        });
    }

    let end_idx = closes.len().saturating_sub(1);
    let start_idx = end_idx.saturating_sub(period);

    let p_end = closes.get(end_idx).ok_or(CoreError::InsufficientBars {
        required: end_idx + 1,
        provided: closes.len(),
    })?;
    let p_start = closes.get(start_idx).ok_or(CoreError::InsufficientBars {
        required: start_idx + 1,
        provided: closes.len(),
    })?;

    let direction = (p_end - p_start).abs();
    let dir_f64 = direction.to_f64().ok_or(CoreError::NonFiniteCalculation {
        name: "kaufman_er_direction",
    })?;

    let relevant_slice = closes
        .get(start_idx..=end_idx)
        .ok_or(CoreError::InsufficientBars {
            required: end_idx + 1,
            provided: closes.len(),
        })?;

    let mut volatility = Decimal::ZERO;
    for pair in relevant_slice.windows(2) {
        if let (Some(a), Some(b)) = (pair.first(), pair.get(1)) {
            volatility += (b - a).abs();
        }
    }

    if volatility.is_zero() {
        return Ok(0.0);
    }

    let vol_f64 = volatility.to_f64().ok_or(CoreError::NonFiniteCalculation {
        name: "kaufman_er_volatility",
    })?;

    let er = dir_f64 / vol_f64;
    if !er.is_finite() {
        return Err(CoreError::NonFiniteCalculation {
            name: "kaufman_efficiency_ratio",
        });
    }

    Ok(er.clamp(0.0, 1.0))
}

pub fn true_range(current: &Bar, previous: &Bar) -> Decimal {
    let hl = current.hl_range();
    let hc = (current.high - previous.close).abs();
    let lc = (current.low - previous.close).abs();
    hl.max(hc).max(lc)
}

pub fn average_true_range(bars: &[Bar], period: usize) -> Result<Decimal, CoreError> {
    if period == 0 {
        return Err(CoreError::InvalidParameter {
            name: "period",
            reason: "period must be greater than zero",
        });
    }
    if bars.len() <= period {
        return Err(CoreError::InsufficientBars {
            required: period + 1,
            provided: bars.len(),
        });
    }

    let start_idx = bars.len().saturating_sub(period);
    let mut sum_tr = Decimal::ZERO;

    for i in start_idx..bars.len() {
        let curr = bars.get(i).ok_or(CoreError::InsufficientBars {
            required: i + 1,
            provided: bars.len(),
        })?;
        let prev_idx = i.saturating_sub(1);
        let prev = bars.get(prev_idx).ok_or(CoreError::InsufficientBars {
            required: prev_idx + 1,
            provided: bars.len(),
        })?;
        sum_tr += true_range(curr, prev);
    }

    let period_dec = Decimal::from(period);
    Ok(sum_tr / period_dec)
}

pub fn donchian_channel(bars: &[Bar], period: usize) -> Result<(Decimal, Decimal), CoreError> {
    if period == 0 {
        return Err(CoreError::InvalidParameter {
            name: "period",
            reason: "period must be greater than zero",
        });
    }
    if bars.len() < period {
        return Err(CoreError::InsufficientBars {
            required: period,
            provided: bars.len(),
        });
    }

    let start_idx = bars.len().saturating_sub(period);
    let slice = bars.get(start_idx..).ok_or(CoreError::InsufficientBars {
        required: bars.len(),
        provided: bars.len(),
    })?;

    let first = slice.first().ok_or(CoreError::InsufficientBars {
        required: 1,
        provided: 0,
    })?;

    let mut highest_high = first.high;
    let mut lowest_low = first.low;

    for bar in slice {
        if bar.high > highest_high {
            highest_high = bar.high;
        }
        if bar.low < lowest_low {
            lowest_low = bar.low;
        }
    }

    Ok((highest_high, lowest_low))
}

pub fn volume_sma(bars: &[Bar], period: usize) -> Result<Decimal, CoreError> {
    if period == 0 {
        return Err(CoreError::InvalidParameter {
            name: "period",
            reason: "period must be greater than zero",
        });
    }
    if bars.len() < period {
        return Err(CoreError::InsufficientBars {
            required: period,
            provided: bars.len(),
        });
    }

    let start_idx = bars.len().saturating_sub(period);
    let slice = bars.get(start_idx..).ok_or(CoreError::InsufficientBars {
        required: bars.len(),
        provided: bars.len(),
    })?;

    let mut sum_vol = Decimal::ZERO;
    for bar in slice {
        sum_vol += bar.volume;
    }

    let period_dec = Decimal::from(period);
    Ok(sum_vol / period_dec)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn sample_bar(open: Decimal, high: Decimal, low: Decimal, close: Decimal, vol: Decimal) -> Bar {
        Bar::new(
            100,
            200,
            open,
            high,
            low,
            close,
            vol,
            vol / dec!(2),
            vol * close,
            10,
        )
        .unwrap()
    }

    #[test]
    fn perfect_monotonic_trend_yields_unit_er() {
        let closes = vec![dec!(10.0), dec!(11.0), dec!(12.0), dec!(13.0), dec!(14.0)];
        let er = kaufman_efficiency_ratio(&closes, 4).unwrap();
        assert!((er - 1.0).abs() < 1e-6);
    }

    #[test]
    fn perfect_sawtooth_chop_yields_low_er() {
        let closes = vec![dec!(10.0), dec!(12.0), dec!(10.0), dec!(12.0), dec!(10.0)];
        let er = kaufman_efficiency_ratio(&closes, 4).unwrap();
        // net change: |10 - 10| = 0 => ER = 0.0
        assert_eq!(er, 0.0);
    }

    #[test]
    fn atr_calculation_matches_expected() {
        let b1 = sample_bar(dec!(100), dec!(110), dec!(90), dec!(105), dec!(10));
        let b2 = sample_bar(dec!(105), dec!(115), dec!(95), dec!(100), dec!(10));
        let b3 = sample_bar(dec!(100), dec!(120), dec!(98), dec!(110), dec!(10));

        let atr = average_true_range(&[b1, b2, b3], 2).unwrap();
        // TR(b2) = max(115-95=20, |115-105|=10, |95-105|=10) = 20
        // TR(b3) = max(120-98=22, |120-100|=20, |98-100|=2) = 22
        // ATR = (20 + 22) / 2 = 21
        assert_eq!(atr, dec!(21));
    }

    #[test]
    fn donchian_channel_identifies_extrema() {
        let b1 = sample_bar(dec!(100), dec!(105), dec!(95), dec!(102), dec!(5));
        let b2 = sample_bar(dec!(102), dec!(112), dec!(98), dec!(108), dec!(8));
        let b3 = sample_bar(dec!(108), dec!(109), dec!(91), dec!(95), dec!(12));

        let (high, low) = donchian_channel(&[b1, b2, b3], 3).unwrap();
        assert_eq!(high, dec!(112));
        assert_eq!(low, dec!(91));
    }
}
