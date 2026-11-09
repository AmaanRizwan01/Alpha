use crate::error::VenueError;
use alpha_types::{Bar, BookTop, MarketTrade};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinanceWsBookTicker {
    #[serde(rename = "u")]
    pub update_id: u64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "b")]
    pub bid_price: Decimal,
    #[serde(rename = "B")]
    pub bid_qty: Decimal,
    #[serde(rename = "a")]
    pub ask_price: Decimal,
    #[serde(rename = "A")]
    pub ask_qty: Decimal,
}

impl BinanceWsBookTicker {
    pub fn to_book_top(&self, timestamp_ms: i64) -> Result<BookTop, VenueError> {
        Ok(BookTop::new(
            timestamp_ms,
            self.bid_price,
            self.bid_qty,
            self.ask_price,
            self.ask_qty,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinanceWsTrade {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "t")]
    pub trade_id: i64,
    #[serde(rename = "p")]
    pub price: Decimal,
    #[serde(rename = "q")]
    pub qty: Decimal,
    #[serde(rename = "T")]
    pub trade_time_ms: i64,
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,
}

impl BinanceWsTrade {
    pub fn to_market_trade(&self) -> Result<MarketTrade, VenueError> {
        Ok(MarketTrade::new(
            &self.symbol,
            self.trade_id,
            self.price,
            self.qty,
            self.is_buyer_maker,
            self.trade_time_ms,
        )?)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinanceWsKlineData {
    #[serde(rename = "t")]
    pub open_time_ms: i64,
    #[serde(rename = "T")]
    pub close_time_ms: i64,
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "i")]
    pub interval: String,
    #[serde(rename = "o")]
    pub open: Decimal,
    #[serde(rename = "c")]
    pub close: Decimal,
    #[serde(rename = "h")]
    pub high: Decimal,
    #[serde(rename = "l")]
    pub low: Decimal,
    #[serde(rename = "v")]
    pub volume: Decimal,
    #[serde(rename = "n")]
    pub trades_count: u64,
    #[serde(rename = "x")]
    pub is_closed: bool,
    #[serde(rename = "q")]
    pub quote_volume: Decimal,
    #[serde(rename = "V")]
    pub taker_buy_volume: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinanceWsKlineEvent {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(rename = "k")]
    pub kline: BinanceWsKlineData,
}

impl BinanceWsKlineEvent {
    pub fn to_bar(&self) -> Result<Bar, VenueError> {
        let k = &self.kline;
        Ok(Bar::new(
            k.open_time_ms,
            k.close_time_ms,
            k.open,
            k.high,
            k.low,
            k.close,
            k.volume,
            k.taker_buy_volume,
            k.quote_volume,
            k.trades_count,
        )?)
    }

    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.kline.is_closed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum BinanceWsMessage {
    BookTicker(BinanceWsBookTicker),
    Trade(BinanceWsTrade),
    Kline(BinanceWsKlineEvent),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BinanceCombinedStreamMessage {
    pub stream: String,
    pub data: BinanceWsMessage,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_deserialize_book_ticker() {
        let json =
            r#"{"u":400900217,"s":"BTCUSDT","b":"65100.10","B":"1.50","a":"65101.20","A":"2.30"}"#;
        let ticker: BinanceWsBookTicker = serde_json::from_str(json).unwrap();
        assert_eq!(ticker.symbol, "BTCUSDT");
        assert_eq!(ticker.bid_price, dec!(65100.10));

        let top = ticker.to_book_top(1700000000000).unwrap();
        assert_eq!(top.spread(), dec!(1.10));
    }

    #[test]
    fn test_deserialize_trade() {
        let json = r#"{"e":"trade","E":1700000000050,"s":"BTCUSDT","t":12345,"p":"65100.00","q":"0.05","b":88,"a":50,"T":1700000000000,"m":true,"M":true}"#;
        let trade: BinanceWsTrade = serde_json::from_str(json).unwrap();
        assert_eq!(trade.symbol, "BTCUSDT");
        assert_eq!(trade.price, dec!(65100.00));
        assert!(trade.is_buyer_maker);

        let m_trade = trade.to_market_trade().unwrap();
        assert_eq!(m_trade.trade_id, 12345);
    }

    #[test]
    fn test_deserialize_kline_event() {
        let json = r#"{"e":"kline","E":1700000000000,"s":"BTCUSDT","k":{"t":1699990000000,"T":1700000000000,"s":"BTCUSDT","i":"1h","f":100,"L":200,"o":"64000.0","c":"65000.0","h":"65500.0","l":"63900.0","v":"150.0","n":1200,"x":true,"q":"9700000.0","V":"85.0","Q":"5500000.0","B":"0"}}"#;
        let event: BinanceWsKlineEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.symbol, "BTCUSDT");
        assert!(event.is_closed());

        let bar = event.to_bar().unwrap();
        assert_eq!(bar.high, dec!(65500.0));
        assert_eq!(bar.close, dec!(65000.0));
        assert_eq!(bar.taker_buy_volume, dec!(85.0));
    }

    #[test]
    fn test_deserialize_combined_stream() {
        let json = r#"{"stream":"btcusdt@trade","data":{"s":"BTCUSDT","t":12345,"p":"65100.00","q":"0.05","T":1700000000000,"m":true}}"#;
        let msg: BinanceCombinedStreamMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.stream, "btcusdt@trade");
        match msg.data {
            BinanceWsMessage::Trade(t) => assert_eq!(t.price, dec!(65100.00)),
            _ => panic!("expected trade"),
        }
    }
}
