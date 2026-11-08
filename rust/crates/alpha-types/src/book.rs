use crate::error::TypeError;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookTop {
    pub timestamp_ms: i64,
    pub bid_price: Decimal,
    pub bid_qty: Decimal,
    pub ask_price: Decimal,
    pub ask_qty: Decimal,
}

impl BookTop {
    pub fn new(
        timestamp_ms: i64,
        bid_price: Decimal,
        bid_qty: Decimal,
        ask_price: Decimal,
        ask_qty: Decimal,
    ) -> Result<Self, TypeError> {
        if bid_price <= Decimal::ZERO || ask_price <= Decimal::ZERO {
            return Err(TypeError::InvalidBookTop {
                reason: "prices must be strictly positive",
            });
        }
        if bid_qty <= Decimal::ZERO || ask_qty <= Decimal::ZERO {
            return Err(TypeError::InvalidBookTop {
                reason: "quantities must be strictly positive",
            });
        }
        if bid_price >= ask_price {
            return Err(TypeError::InvalidBookTop {
                reason: "crossed or locked book: bid_price >= ask_price",
            });
        }

        Ok(Self {
            timestamp_ms,
            bid_price,
            bid_qty,
            ask_price,
            ask_qty,
        })
    }

    #[must_use]
    pub fn spread(&self) -> Decimal {
        self.ask_price - self.bid_price
    }

    #[must_use]
    pub fn spread_bps(&self) -> Decimal {
        (self.spread() / self.ask_price) * dec!(10000)
    }

    #[must_use]
    pub fn mid_price(&self) -> Decimal {
        (self.bid_price + self.ask_price) / dec!(2)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn valid_book_top() {
        let top = BookTop::new(
            1700000000000,
            dec!(50000.0),
            dec!(1.5),
            dec!(50005.0),
            dec!(2.0),
        );
        assert!(top.is_ok());
        let t = top.unwrap();
        assert_eq!(t.spread(), dec!(5.0));
        assert_eq!(t.mid_price(), dec!(50002.5));
    }

    #[test]
    fn rejects_crossed_book() {
        let res = BookTop::new(
            1700000000000,
            dec!(50010.0),
            dec!(1.0),
            dec!(50000.0),
            dec!(1.0),
        );
        assert!(res.is_err());
    }
}
