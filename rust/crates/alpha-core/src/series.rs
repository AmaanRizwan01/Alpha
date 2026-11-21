use alpha_types::Bar;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub fn calc_rsi_series(closes: &[Decimal], period: usize) -> Vec<Decimal> {
    let n = closes.len();
    let def = dec!(50.0);
    if n <= period || period == 0 {
        return vec![def; n];
    }
    let mut out = vec![def; n];
    let (mut gains, mut losses) = (Vec::with_capacity(n - 1), Vec::with_capacity(n - 1));
    for w in closes.windows(2) {
        if let (Some(&p), Some(&c)) = (w.first(), w.get(1)) {
            let diff = c - p;
            gains.push(if diff > Decimal::ZERO {
                diff
            } else {
                Decimal::ZERO
            });
            losses.push(if diff < Decimal::ZERO {
                -diff
            } else {
                Decimal::ZERO
            });
        }
    }
    let p_dec = Decimal::from(period);
    let mut avg_g: Decimal = gains.iter().take(period).copied().sum::<Decimal>() / p_dec;
    let mut avg_l: Decimal = losses.iter().take(period).copied().sum::<Decimal>() / p_dec;
    let p_sub = Decimal::from(period - 1);
    for i in period..gains.len() {
        if let (Some(&g), Some(&l)) = (gains.get(i), losses.get(i)) {
            avg_g = (avg_g * p_sub + g) / p_dec;
            avg_l = (avg_l * p_sub + l) / p_dec;
            if i + 1 < n {
                out[i + 1] = if avg_l.is_zero() {
                    dec!(100.0)
                } else {
                    dec!(100.0) - (dec!(100.0) / (Decimal::ONE + (avg_g / avg_l)))
                };
            }
        }
    }
    out
}

pub fn calc_atr_series(bars: &[Bar], period: usize) -> Vec<Decimal> {
    let n = bars.len();
    if n < period || period == 0 {
        return vec![Decimal::ZERO; n];
    }
    let mut trs = Vec::with_capacity(n);
    if let Some(b0) = bars.first() {
        trs.push(b0.high - b0.low);
    }
    for i in 1..n {
        if let (Some(c), Some(p)) = (bars.get(i), bars.get(i - 1)) {
            trs.push(
                (c.high - c.low)
                    .max((c.high - p.close).abs())
                    .max((c.low - p.close).abs()),
            );
        }
    }
    let mut out = vec![Decimal::ZERO; n];
    let p_dec = Decimal::from(period);
    out[period - 1] = trs.iter().take(period).copied().sum::<Decimal>() / p_dec;
    let p_sub = Decimal::from(period - 1);
    for i in period..n {
        if let (Some(&tr), Some(&prev)) = (trs.get(i), out.get(i - 1)) {
            out[i] = (prev * p_sub + tr) / p_dec;
        }
    }
    out
}

#[allow(clippy::needless_range_loop)]
pub fn calc_ma_series(arr: &[Decimal], period: usize, is_ema: bool) -> Vec<Decimal> {
    let n = arr.len();
    if n < period || period == 0 {
        return vec![Decimal::ZERO; n];
    }
    let mut out = vec![Decimal::ZERO; n];
    let p_dec = Decimal::from(period);
    if !is_ema {
        let mut sum: Decimal = arr.iter().take(period).copied().sum();
        out[period - 1] = sum / p_dec;
        for i in period..n {
            if let (Some(&add_v), Some(&sub_v)) = (arr.get(i), arr.get(i - period)) {
                sum += add_v - sub_v;
                out[i] = sum / p_dec;
            }
        }
        return out;
    }
    out[period - 1] = arr.iter().take(period).copied().sum::<Decimal>() / p_dec;
    let alpha = Decimal::TWO / Decimal::from(period + 1);
    let one_sub = Decimal::ONE - alpha;
    for i in period..n {
        if let (Some(&v), Some(&prev)) = (arr.get(i), out.get(i - 1)) {
            out[i] = alpha * v + one_sub * prev;
        }
    }
    out
}
