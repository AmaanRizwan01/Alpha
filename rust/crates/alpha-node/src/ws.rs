use crate::error::NodeError;
use alpha_types::{Bar, BookTop, MarketTrade};
use alpha_venues::{BinanceCombinedStreamMessage, BinanceWsMessage};
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveMarketEvent {
    BookTicker(BookTop),
    Trade(MarketTrade),
    Kline {
        symbol: String,
        bar: Bar,
        is_closed: bool,
    },
}

#[must_use]
pub fn build_stream_url(symbols: &[String], intervals: &[&str]) -> String {
    let mut streams = Vec::new();
    for sym in symbols {
        let s = sym.to_lowercase();
        streams.push(format!("{}@bookTicker", s));
        streams.push(format!("{}@trade", s));
        for interval in intervals {
            streams.push(format!("{}@kline_{}", s, interval));
        }
    }
    format!(
        "wss://stream.binance.com:9443/stream?streams={}",
        streams.join("/")
    )
}

pub fn parse_ws_frame(text: &str, now_ms: i64) -> Result<Option<LiveMarketEvent>, NodeError> {
    let msg: BinanceCombinedStreamMessage = match serde_json::from_str(text) {
        Ok(m) => m,
        Err(_) => return Ok(None),
    };

    let event = match msg.data {
        BinanceWsMessage::BookTicker(b) => LiveMarketEvent::BookTicker(b.to_book_top(now_ms)?),
        BinanceWsMessage::Trade(t) => LiveMarketEvent::Trade(t.to_market_trade()?),
        BinanceWsMessage::Kline(k) => LiveMarketEvent::Kline {
            symbol: k.symbol.clone(),
            bar: k.to_bar()?,
            is_closed: k.is_closed(),
        },
    };
    Ok(Some(event))
}

pub async fn connect_binance_stream(
    stream_url: &str,
) -> Result<WebSocketStream<MaybeTlsStream<TcpStream>>, NodeError> {
    let (stream, _) = tokio_tungstenite::connect_async(stream_url).await?;
    Ok(stream)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_build_stream_url() {
        let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
        let url = build_stream_url(&symbols, &["1h", "4h"]);
        assert!(url.starts_with("wss://stream.binance.com:9443/stream?streams="));
        assert!(url.contains("btcusdt@bookTicker"));
        assert!(url.contains("btcusdt@trade"));
        assert!(url.contains("btcusdt@kline_1h"));
        assert!(url.contains("btcusdt@kline_4h"));
        assert!(url.contains("ethusdt@bookTicker"));
    }

    #[test]
    fn test_parse_book_ticker_frame() {
        let frame = r#"{"stream":"btcusdt@bookTicker","data":{"u":400900217,"s":"BTCUSDT","b":"65100.10","B":"1.50","a":"65101.20","A":"2.30"}}"#;
        let event = parse_ws_frame(frame, 1700000000000).unwrap();
        assert!(matches!(event, Some(LiveMarketEvent::BookTicker(_))));
        if let Some(LiveMarketEvent::BookTicker(top)) = event {
            assert_eq!(top.bid_price, dec!(65100.10));
            assert_eq!(top.ask_price, dec!(65101.20));
        }
    }

    #[test]
    fn test_parse_trade_frame() {
        let frame = r#"{"stream":"btcusdt@trade","data":{"s":"BTCUSDT","t":999,"p":"65050.00","q":"0.25","T":1700000000000,"m":false}}"#;
        let event = parse_ws_frame(frame, 1700000000000).unwrap();
        assert!(matches!(event, Some(LiveMarketEvent::Trade(_))));
        if let Some(LiveMarketEvent::Trade(t)) = event {
            assert_eq!(t.symbol, "BTCUSDT");
            assert_eq!(t.price, dec!(65050.00));
            assert!(!t.is_buyer_maker);
        }
    }

    #[test]
    fn test_parse_kline_frame() {
        let frame = r#"{"stream":"btcusdt@kline_1h","data":{"s":"BTCUSDT","k":{"t":1699990000000,"T":1700000000000,"s":"BTCUSDT","i":"1h","f":100,"L":200,"o":"64000.0","c":"65000.0","h":"65500.0","l":"63900.0","v":"150.0","n":1200,"x":true,"q":"9700000.0","V":"85.0","Q":"5500000.0","B":"0"}}}"#;
        let event = parse_ws_frame(frame, 1700000000000).unwrap();
        assert!(matches!(event, Some(LiveMarketEvent::Kline { .. })));
        if let Some(LiveMarketEvent::Kline {
            symbol,
            bar,
            is_closed,
        }) = event
        {
            assert_eq!(symbol, "BTCUSDT");
            assert!(is_closed);
            assert_eq!(bar.close, dec!(65000.0));
        }
    }
}
