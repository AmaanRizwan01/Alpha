use crate::error::TypeError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MarketTrade {
    pub symbol: String,
    pub trade_id: i64,
    pub price: Decimal,
    pub qty: Decimal,
    pub is_buyer_maker: bool,
    pub timestamp_ms: i64,
}

impl MarketTrade {
    pub fn new(
        symbol: impl Into<String>,
        trade_id: i64,
        price: Decimal,
        qty: Decimal,
        is_buyer_maker: bool,
        timestamp_ms: i64,
    ) -> Result<Self, TypeError> {
        if price <= Decimal::ZERO {
            return Err(TypeError::InvalidPrice {
                reason: "trade price must be strictly positive",
            });
        }
        if qty <= Decimal::ZERO {
            return Err(TypeError::InvalidQuantity {
                reason: "trade quantity must be strictly positive",
            });
        }

        Ok(Self {
            symbol: symbol.into(),
            trade_id,
            price,
            qty,
            is_buyer_maker,
            timestamp_ms,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn valid_trade_creation() {
        let trade = MarketTrade::new(
            "BTCUSDT",
            1001,
            dec!(65000.0),
            dec!(0.5),
            true,
            1700000000000,
        );
        assert!(trade.is_ok());
        let t = trade.unwrap();
        assert_eq!(t.price, dec!(65000.0));
        assert_eq!(t.qty, dec!(0.5));
    }

    #[test]
    fn invalid_trade_zero_price() {
        let trade = MarketTrade::new(
            "BTCUSDT",
            1002,
            Decimal::ZERO,
            dec!(0.5),
            true,
            1700000000000,
        );
        assert!(trade.is_err());
    }
}
