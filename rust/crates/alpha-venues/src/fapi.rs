use crate::error::VenueError;
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinanceFundingRateRecord {
    pub symbol: String,
    pub funding_rate: Decimal,
    pub funding_time: i64,
    pub mark_price: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BinancePremiumIndexRecord {
    pub symbol: String,
    pub mark_price: Decimal,
    pub index_price: Decimal,
    pub last_funding_rate: Decimal,
    pub next_funding_time: i64,
    pub time: i64,
}

#[derive(Deserialize)]
struct RawFundingRate {
    symbol: String,
    #[serde(rename = "fundingRate")]
    funding_rate: String,
    #[serde(rename = "fundingTime")]
    funding_time: i64,
    #[serde(rename = "markPrice")]
    mark_price: Option<String>,
}

#[derive(Deserialize)]
struct RawPremiumIndex {
    symbol: String,
    #[serde(rename = "markPrice")]
    mark_price: String,
    #[serde(rename = "indexPrice")]
    index_price: String,
    #[serde(rename = "lastFundingRate")]
    last_funding_rate: String,
    #[serde(rename = "nextFundingTime")]
    next_funding_time: i64,
    time: i64,
}

#[must_use]
pub fn build_funding_rate_path(symbol: &str, limit: Option<u32>) -> String {
    let lim = limit.unwrap_or(100);
    format!("/fapi/v1/fundingRate?symbol={}&limit={}", symbol, lim)
}

#[must_use]
pub fn build_premium_index_path(symbol: &str) -> String {
    format!("/fapi/v1/premiumIndex?symbol={}", symbol)
}

pub fn parse_funding_rates(json_str: &str) -> Result<Vec<BinanceFundingRateRecord>, VenueError> {
    let raw_list: Vec<RawFundingRate> =
        serde_json::from_str(json_str).map_err(|e| VenueError::SerializationError {
            reason: e.to_string(),
        })?;

    let mut records = Vec::with_capacity(raw_list.len());
    for raw in raw_list {
        let rate =
            Decimal::from_str(&raw.funding_rate).map_err(|e| VenueError::SerializationError {
                reason: e.to_string(),
            })?;
        let mark = match raw.mark_price {
            Some(ref mp) if !mp.is_empty() => {
                Some(
                    Decimal::from_str(mp).map_err(|e| VenueError::SerializationError {
                        reason: e.to_string(),
                    })?,
                )
            }
            _ => None,
        };
        records.push(BinanceFundingRateRecord {
            symbol: raw.symbol,
            funding_rate: rate,
            funding_time: raw.funding_time,
            mark_price: mark,
        });
    }
    Ok(records)
}

pub fn parse_premium_index(json_str: &str) -> Result<BinancePremiumIndexRecord, VenueError> {
    let raw: RawPremiumIndex =
        serde_json::from_str(json_str).map_err(|e| VenueError::SerializationError {
            reason: e.to_string(),
        })?;

    let mark = Decimal::from_str(&raw.mark_price).map_err(|e| VenueError::SerializationError {
        reason: e.to_string(),
    })?;
    let index =
        Decimal::from_str(&raw.index_price).map_err(|e| VenueError::SerializationError {
            reason: e.to_string(),
        })?;
    let last_rate =
        Decimal::from_str(&raw.last_funding_rate).map_err(|e| VenueError::SerializationError {
            reason: e.to_string(),
        })?;

    Ok(BinancePremiumIndexRecord {
        symbol: raw.symbol,
        mark_price: mark,
        index_price: index,
        last_funding_rate: last_rate,
        next_funding_time: raw.next_funding_time,
        time: raw.time,
    })
}

pub fn parse_premium_index_list(
    json_str: &str,
) -> Result<Vec<BinancePremiumIndexRecord>, VenueError> {
    let raw_list: Vec<RawPremiumIndex> =
        serde_json::from_str(json_str).map_err(|e| VenueError::SerializationError {
            reason: e.to_string(),
        })?;

    let mut records = Vec::with_capacity(raw_list.len());
    for raw in raw_list {
        let mark =
            Decimal::from_str(&raw.mark_price).map_err(|e| VenueError::SerializationError {
                reason: e.to_string(),
            })?;
        let index =
            Decimal::from_str(&raw.index_price).map_err(|e| VenueError::SerializationError {
                reason: e.to_string(),
            })?;
        let last_rate = Decimal::from_str(&raw.last_funding_rate).map_err(|e| {
            VenueError::SerializationError {
                reason: e.to_string(),
            }
        })?;

        records.push(BinancePremiumIndexRecord {
            symbol: raw.symbol,
            mark_price: mark,
            index_price: index,
            last_funding_rate: last_rate,
            next_funding_time: raw.next_funding_time,
            time: raw.time,
        });
    }
    Ok(records)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_parse_funding_rates_payload() {
        let json = r#"[
            {"symbol":"BTCUSDT","fundingRate":"0.00010000","fundingTime":1700000000000,"markPrice":"65000.50"},
            {"symbol":"BTCUSDT","fundingRate":"-0.00005000","fundingTime":1700028800000,"markPrice":"65100.00"}
        ]"#;
        let rates = parse_funding_rates(json).unwrap();
        assert_eq!(rates.len(), 2);
        assert_eq!(rates[0].funding_rate, dec!(0.00010000));
        assert_eq!(rates[0].funding_time, 1700000000000);
        assert_eq!(rates[0].mark_price, Some(dec!(65000.50)));
        assert_eq!(rates[1].funding_rate, dec!(-0.00005000));
    }

    #[test]
    fn test_parse_premium_index_payload() {
        let json = r#"{
            "symbol":"BTCUSDT",
            "markPrice":"65230.10",
            "indexPrice":"65225.00",
            "estimatedSettlePrice":"65228.00",
            "lastFundingRate":"0.00012000",
            "interestRate":"0.00010000",
            "nextFundingTime":1700028800000,
            "time":1700025000000
        }"#;
        let index = parse_premium_index(json).unwrap();
        assert_eq!(index.symbol, "BTCUSDT");
        assert_eq!(index.mark_price, dec!(65230.10));
        assert_eq!(index.index_price, dec!(65225.00));
        assert_eq!(index.last_funding_rate, dec!(0.00012000));
        assert_eq!(index.next_funding_time, 1700028800000);
    }

    #[test]
    fn test_parse_premium_index_list_payload() {
        let json = r#"[
            {
                "symbol":"BTCUSDT",
                "markPrice":"65230.10",
                "indexPrice":"65225.00",
                "estimatedSettlePrice":"65228.00",
                "lastFundingRate":"0.00012000",
                "interestRate":"0.00010000",
                "nextFundingTime":1700028800000,
                "time":1700025000000
            },
            {
                "symbol":"ETHUSDT",
                "markPrice":"3400.50",
                "indexPrice":"3400.00",
                "estimatedSettlePrice":"3400.20",
                "lastFundingRate":"0.00015000",
                "interestRate":"0.00010000",
                "nextFundingTime":1700028800000,
                "time":1700025000000
            }
        ]"#;
        let list = parse_premium_index_list(json).unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].symbol, "BTCUSDT");
        assert_eq!(list[1].symbol, "ETHUSDT");
        assert_eq!(list[1].mark_price, dec!(3400.50));
    }

    #[test]
    fn test_path_builders() {
        assert_eq!(
            build_funding_rate_path("BTCUSDT", Some(50)),
            "/fapi/v1/fundingRate?symbol=BTCUSDT&limit=50"
        );
        assert_eq!(
            build_premium_index_path("ETHUSDT"),
            "/fapi/v1/premiumIndex?symbol=ETHUSDT"
        );
    }
}
