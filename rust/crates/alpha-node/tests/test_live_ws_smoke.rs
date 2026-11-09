use alpha_node::ws::{build_stream_url, connect_binance_stream, parse_ws_frame, LiveMarketEvent};
use futures_util::StreamExt;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

fn current_time_ms() -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    i64::try_from(now.as_millis()).unwrap_or(0)
}

#[tokio::test]
async fn test_live_binance_stream_connectivity() {
    let symbols = vec!["BTCUSDT".to_string()];
    let url = build_stream_url(&symbols, &["1h"]);

    // Attempt connection with 10s timeout
    let connect_res = timeout(Duration::from_secs(10), connect_binance_stream(&url)).await;
    let mut ws_stream = match connect_res {
        Ok(Ok(stream)) => stream,
        _ => {
            println!("[smoke-test] Live WebSocket connection skipped (network/offline).");
            return;
        }
    };

    let mut frames_received = 0;
    let mut events_parsed = 0;

    // Read up to 5 live frames within 10 seconds
    let read_future = async {
        while let Some(Ok(msg)) = ws_stream.next().await {
            if let Message::Text(text) = msg {
                frames_received += 1;
                let now_ms = current_time_ms();
                if let Ok(Some(event)) = parse_ws_frame(&text, now_ms) {
                    events_parsed += 1;
                    match event {
                        LiveMarketEvent::BookTicker(b) => {
                            println!(
                                "[live-tick] BookTicker bid={} ask={}",
                                b.bid_price, b.ask_price
                            );
                        }
                        LiveMarketEvent::Trade(t) => {
                            println!("[live-tick] Trade px={} qty={}", t.price, t.qty);
                        }
                        LiveMarketEvent::Kline {
                            symbol,
                            bar,
                            is_closed,
                        } => {
                            println!(
                                "[live-tick] Kline {} close={} is_closed={}",
                                symbol, bar.close, is_closed
                            );
                        }
                    }
                }
                if frames_received >= 5 {
                    break;
                }
            }
        }
    };

    let _ = timeout(Duration::from_secs(10), read_future).await;
    println!(
        "[smoke-test] Verified live stream: frames_received={}, events_parsed={}",
        frames_received, events_parsed
    );
    assert!(
        frames_received > 0,
        "Expected at least 1 live frame from Binance"
    );
    assert!(events_parsed > 0, "Expected at least 1 parsed market event");
}
