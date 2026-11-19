use crate::error::TypeError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Bar {
    pub open_time_ms: i64,
    pub close_time_ms: i64,
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
    pub taker_buy_volume: Decimal,
    pub quote_volume: Decimal,
    pub trades_count: u64,
}

impl Bar {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        open_time_ms: i64,
        close_time_ms: i64,
        open: Decimal,
        high: Decimal,
        low: Decimal,
        close: Decimal,
        volume: Decimal,
        taker_buy_volume: Decimal,
        quote_volume: Decimal,
        trades_count: u64,
    ) -> Result<Self, TypeError> {
        if open_time_ms > close_time_ms {
            return Err(TypeError::InvalidBar {
                reason: "open_time_ms exceeds close_time_ms",
            });
        }
        if open <= Decimal::ZERO
            || high <= Decimal::ZERO
            || low <= Decimal::ZERO
            || close <= Decimal::ZERO
        {
            return Err(TypeError::InvalidBar {
                reason: "prices must be strictly positive",
            });
        }
        if low > open || low > close || low > high {
            return Err(TypeError::InvalidBar {
                reason: "low cannot exceed open, close, or high",
            });
        }
        if high < open || high < close {
            return Err(TypeError::InvalidBar {
                reason: "high cannot be lower than open or close",
            });
        }
        if volume < Decimal::ZERO || taker_buy_volume < Decimal::ZERO {
            return Err(TypeError::InvalidBar {
                reason: "volumes must be non-negative",
            });
        }
        if taker_buy_volume > volume {
            return Err(TypeError::InvalidBar {
                reason: "taker buy volume cannot exceed total volume",
            });
        }

        Ok(Self {
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
        })
    }

    #[must_use]
    pub fn taker_buy_ratio(&self) -> Option<Decimal> {
        if self.volume <= Decimal::ZERO {
            None
        } else {
            Some(self.taker_buy_volume / self.volume)
        }
    }

    #[must_use]
    pub fn hl_range(&self) -> Decimal {
        self.high - self.low
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn valid_bar_creation() {
        let bar = Bar::new(
            1700000000000,
            1700003600000,
            dec!(50000.0),
            dec!(51000.0),
            dec!(49500.0),
            dec!(50500.0),
            dec!(100.0),
            dec!(60.0),
            dec!(5050000.0),
            1200,
        );
        assert!(bar.is_ok());
        let b = bar.unwrap();
        assert_eq!(b.taker_buy_ratio(), Some(dec!(0.6)));
        assert_eq!(b.hl_range(), dec!(1500.0));
    }

    #[test]
    fn rejects_inverted_high_low() {
        let res = Bar::new(
            100,
            200,
            dec!(50000.0),
            dec!(49000.0),
            dec!(51000.0),
            dec!(50000.0),
            dec!(10.0),
            dec!(5.0),
            dec!(500000.0),
            10,
        );
        assert!(res.is_err());
    }

    #[test]
    fn rejects_taker_buy_exceeding_volume() {
        let res = Bar::new(
            100,
            200,
            dec!(50000.0),
            dec!(51000.0),
            dec!(49000.0),
            dec!(50000.0),
            dec!(10.0),
            dec!(15.0),
            dec!(500000.0),
            10,
        );
        assert!(res.is_err());
    }
}
