use crate::error::TypeError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Venue {
    Binance,
    Okx,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Side {
    Buy,
    Sell,
}

impl Side {
    #[must_use]
    pub fn opposite(&self) -> Self {
        match self {
            Self::Buy => Self::Sell,
            Self::Sell => Self::Buy,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OrderType {
    LimitMaker,
    Limit,
    Market,
    StopLossLimit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrderIntent {
    pub venue: Venue,
    pub symbol: String,
    pub side: Side,
    pub order_type: OrderType,
    pub price: Option<Decimal>,
    pub qty: Decimal,
    pub stop_loss_price: Option<Decimal>,
    pub take_profit_price: Option<Decimal>,
    pub client_order_id: String,
}

impl OrderIntent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        venue: Venue,
        symbol: impl Into<String>,
        side: Side,
        order_type: OrderType,
        price: Option<Decimal>,
        qty: Decimal,
        stop_loss_price: Option<Decimal>,
        take_profit_price: Option<Decimal>,
        client_order_id: impl Into<String>,
    ) -> Result<Self, TypeError> {
        if qty <= Decimal::ZERO {
            return Err(TypeError::InvalidQuantity {
                reason: "order quantity must be strictly positive",
            });
        }

        if let Some(p) = price {
            if p <= Decimal::ZERO {
                return Err(TypeError::InvalidPrice {
                    reason: "order price must be strictly positive",
                });
            }
        } else if order_type != OrderType::Market {
            return Err(TypeError::InvalidPrice {
                reason: "non-market order requires a price",
            });
        }

        if let (Some(entry), Some(sl)) = (price, stop_loss_price) {
            if sl <= Decimal::ZERO {
                return Err(TypeError::InvalidPrice {
                    reason: "stop loss price must be strictly positive",
                });
            }
            if side == Side::Buy && sl >= entry {
                return Err(TypeError::InvalidPrice {
                    reason: "buy stop loss must be strictly below entry price",
                });
            }
            if side == Side::Sell && sl <= entry {
                return Err(TypeError::InvalidPrice {
                    reason: "sell stop loss must be strictly above entry price",
                });
            }
        }

        Ok(Self {
            venue,
            symbol: symbol.into(),
            side,
            order_type,
            price,
            qty,
            stop_loss_price,
            take_profit_price,
            client_order_id: client_order_id.into(),
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn valid_order_intent_creation() {
        let order = OrderIntent::new(
            Venue::Binance,
            "BTCUSDT",
            Side::Buy,
            OrderType::LimitMaker,
            Some(dec!(50000.0)),
            dec!(0.05),
            Some(dec!(48500.0)),
            Some(dec!(55000.0)),
            "ord-001",
        );
        assert!(order.is_ok());
    }

    #[test]
    fn rejects_buy_stop_above_entry() {
        let res = OrderIntent::new(
            Venue::Okx,
            "BTC-USDT",
            Side::Buy,
            OrderType::LimitMaker,
            Some(dec!(50000.0)),
            dec!(0.05),
            Some(dec!(51000.0)),
            None,
            "ord-002",
        );
        assert!(res.is_err());
    }
}
