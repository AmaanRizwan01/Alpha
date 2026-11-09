use crate::error::NodeError;
use alpha_types::Bar;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::Duration;

fn parse_decimal(
    val: Option<&serde_json::Value>,
    field: &'static str,
) -> Result<Decimal, NodeError> {
    let s = val
        .and_then(|v| v.as_str())
        .ok_or_else(|| NodeError::StateError {
            reason: format!("missing or non-string kline field: {}", field),
        })?;
    Decimal::from_str(s).map_err(NodeError::DecimalError)
}

pub fn parse_binance_kline_value(v: &serde_json::Value) -> Result<Bar, NodeError> {
    let arr = v.as_array().ok_or_else(|| NodeError::StateError {
        reason: "expected kline JSON array".to_string(),
    })?;

    let open_time_ms =
        arr.first()
            .and_then(|x| x.as_i64())
            .ok_or_else(|| NodeError::StateError {
                reason: "missing open_time_ms".to_string(),
            })?;
    let open = parse_decimal(arr.get(1), "open")?;
    let high = parse_decimal(arr.get(2), "high")?;
    let low = parse_decimal(arr.get(3), "low")?;
    let close = parse_decimal(arr.get(4), "close")?;
    let volume = parse_decimal(arr.get(5), "volume")?;
    let close_time_ms =
        arr.get(6)
            .and_then(|x| x.as_i64())
            .ok_or_else(|| NodeError::StateError {
                reason: "missing close_time_ms".to_string(),
            })?;
    let quote_volume = parse_decimal(arr.get(7), "quote_volume")?;
    let trades_count =
        arr.get(8)
            .and_then(|x| x.as_u64())
            .ok_or_else(|| NodeError::StateError {
                reason: "missing trades_count".to_string(),
            })?;
    let taker_buy_volume = parse_decimal(arr.get(9), "taker_buy_volume")?;

    Ok(Bar::new(
        open_time_ms,
        close_time_ms,
        open,
        high,
        low,
        close,
        volume,
        taker_buy_volume,
        quote_volume,
        trades_count,
    )?)
}

pub async fn fetch_binance_klines(
    symbol: &str,
    interval: &str,
    limit: usize,
) -> Result<Vec<Bar>, NodeError> {
    let url = format!(
        "https://api.binance.com/api/v3/klines?symbol={}&interval={}&limit={}",
        symbol.to_uppercase(),
        interval,
        limit
    );

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?;
    let response = client.get(&url).send().await?.error_for_status()?;
    let raw_array: Vec<serde_json::Value> = response.json().await?;

    let mut bars = Vec::with_capacity(raw_array.len());
    for item in &raw_array {
        bars.push(parse_binance_kline_value(item)?);
    }
    Ok(bars)
}

pub async fn bootstrap_symbol_buffers(
    symbols: &[String],
    timeframe: &str,
    buffers: &mut std::collections::HashMap<String, crate::buffer::BarBuffer>,
) {
    for sym in symbols {
        println!(
            "[bootstrap] Fetching historical {} klines for {}...",
            timeframe, sym
        );
        match fetch_binance_klines(sym, timeframe, 50).await {
            Ok(bars) => {
                let count = bars.len();
                if let Some(buf) = buffers.get_mut(sym) {
                    for bar in bars {
                        let _ = buf.push(bar);
                    }
                }
                println!("[bootstrap] Successfully loaded {} bars for {}", count, sym);
            }
            Err(e) => {
                println!(
                    "[bootstrap] Warning: failed to fetch historical klines for {}: {}. Will warm buffer from live stream.",
                    sym, e
                );
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_parse_binance_kline_value() {
        let json_str = r#"[
            1499040000000,
            "60000.00",
            "61000.00",
            "59500.00",
            "60500.00",
            "100.50",
            1499043599999,
            "6050000.00",
            500,
            "60.25",
            "3645000.00",
            "0"
        ]"#;
        let v: serde_json::Value = serde_json::from_str(json_str).unwrap();
        let bar = parse_binance_kline_value(&v).unwrap();

        assert_eq!(bar.open_time_ms, 1499040000000);
        assert_eq!(bar.close_time_ms, 1499043599999);
        assert_eq!(bar.open, dec!(60000.00));
        assert_eq!(bar.high, dec!(61000.00));
        assert_eq!(bar.low, dec!(59500.00));
        assert_eq!(bar.close, dec!(60500.00));
        assert_eq!(bar.volume, dec!(100.50));
        assert_eq!(bar.taker_buy_volume, dec!(60.25));
        assert_eq!(bar.trades_count, 500);
    }
}
