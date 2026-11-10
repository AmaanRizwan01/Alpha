use alpha_node::config::AppConfig;
use alpha_node::engine::TradeEngine;
use alpha_node::ws::LiveMarketEvent;
use alpha_types::{MarketTrade, OrderIntent, OrderType, Side, Venue};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::fs;

fn create_test_config() -> AppConfig {
    let toml_str = r#"
[system]
mode = "paper"
allow_live_orders = false
log_level = "info"
database_path = "data/test_engine.db"

[universe]
venues = ["binance"]
symbols = ["BTCUSDT"]
timeframe = "4h"

[risk]
max_risk_per_trade = 0.010
min_stop_distance = 0.022
circuit_breaker_drawdown = 0.060
max_open_positions = 2

[execution]
order_type = "post_only"
maker_fee_bps = 0.0
taker_fee_bps = 10.0
slippage_allowance_bps = 5.0

[monitoring]
heartbeat_interval_secs = 60
deadman_interval_secs = 20
deadman_timeout_secs = 60
"#;
    toml::from_str(toml_str).unwrap()
}

#[test]
fn test_trade_engine_lifecycle_and_db_persistence() {
    let _ = fs::create_dir_all("data");
    let _ = fs::remove_file("data/test_engine.db");

    let cfg = create_test_config();
    let mut engine = TradeEngine::new(cfg).unwrap();

    // Submit a working order
    let order = OrderIntent::new(
        Venue::Binance,
        "BTCUSDT",
        Side::Buy,
        OrderType::LimitMaker,
        Some(dec!(60000.0)),
        dec!(0.001),
        Some(dec!(58500.0)),
        Some(dec!(63000.0)),
        "engine-test-1",
    )
    .unwrap();
    engine.simulator.submit_order(order).unwrap();

    // 1. Process market trade at 60000 -> fills entry and persists to SQLite
    let trade = MarketTrade::new("BTCUSDT", 1, dec!(60000.0), dec!(0.1), true, 1000).unwrap();
    engine
        .handle_market_event(LiveMarketEvent::Trade(trade), 1000)
        .unwrap();

    // Verify position recorded in DB
    let pos = engine.db.get_position("BTCUSDT").unwrap();
    assert!(pos.is_some());
    let p = pos.unwrap();
    assert_eq!(p.qty, dec!(0.001));
    assert_eq!(p.entry_price, dec!(60000.0));

    // 2. Process dump trade hitting stop loss -> exits and deletes position from SQLite
    let dump = MarketTrade::new("BTCUSDT", 2, dec!(58400.0), dec!(0.1), false, 2000).unwrap();
    engine
        .handle_market_event(LiveMarketEvent::Trade(dump), 2000)
        .unwrap();

    // Verify position deleted from DB
    let pos_after = engine.db.get_position("BTCUSDT").unwrap();
    assert!(pos_after.is_none());

    // Clean up
    let _ = fs::remove_file("data/test_engine.db");
}

#[test]
fn test_live_h1_signal_generation_and_order_dispatch() {
    use alpha_types::Bar;

    let _ = fs::create_dir_all("data");
    let _ = fs::remove_file("data/test_signal_engine.db");

    let toml_str = r#"
[system]
mode = "paper"
allow_live_orders = false
log_level = "info"
database_path = "data/test_signal_engine.db"

[universe]
venues = ["binance"]
symbols = ["BTCUSDT"]
timeframe = "1h"

[risk]
max_risk_per_trade = 0.010
min_stop_distance = 0.022
circuit_breaker_drawdown = 0.060
max_open_positions = 2

[execution]
order_type = "post_only"
maker_fee_bps = 0.0
taker_fee_bps = 10.0
slippage_allowance_bps = 5.0

[monitoring]
heartbeat_interval_secs = 60
deadman_interval_secs = 20
deadman_timeout_secs = 60
"#;
    let cfg: AppConfig = toml::from_str(toml_str).unwrap();
    let mut engine = TradeEngine::new(cfg).unwrap();

    // 1. Feed 24 baseline bars (steady range around 60000)
    for i in 0..24 {
        let px = dec!(60000.0) + (dec!(10.0) * Decimal::from(i));
        let t = i64::from(i) * 3_600_000;
        let bar = Bar::new(
            t,
            t + 3_600_000,
            px,
            px + dec!(50.0),
            px - dec!(30.0),
            px,
            dec!(1000.0),
            dec!(600.0),
            dec!(50_000_000.0),
            500,
        )
        .unwrap();

        engine
            .handle_market_event(
                LiveMarketEvent::Kline {
                    symbol: "BTCUSDT".to_string(),
                    interval: "1h".to_string(),
                    bar,
                    is_closed: true,
                },
                t + 3_600_000,
            )
            .unwrap();
    }

    assert_eq!(engine.bar_buffers.get("BTCUSDT").unwrap().len(), 24);
    assert!(engine.simulator.working_order.is_none());

    // 2. Feed 25th bar: Breakout surge!
    let breakout_t = 24 * 3_600_000;
    let breakout_bar = Bar::new(
        breakout_t,
        breakout_t + 3_600_000,
        dec!(60240.0),
        dec!(61500.0),
        dec!(60200.0),
        dec!(61400.0),
        dec!(3000.0),
        dec!(2200.0),
        dec!(180_000_000.0),
        1500,
    )
    .unwrap();

    engine
        .handle_market_event(
            LiveMarketEvent::Kline {
                symbol: "BTCUSDT".to_string(),
                interval: "1h".to_string(),
                bar: breakout_bar,
                is_closed: true,
            },
            breakout_t + 3_600_000,
        )
        .unwrap();

    // Verify order was created in simulator and in SQLite
    let working = engine.simulator.working_order.as_ref();
    assert!(working.is_some());
    let intent = working.unwrap();
    assert_eq!(intent.symbol, "BTCUSDT");
    assert_eq!(intent.side, Side::Buy);
    assert_eq!(intent.price, Some(dec!(61400.0)));
    assert!(intent.stop_loss_price.unwrap() < dec!(61400.0));
    assert!(intent.qty > Decimal::ZERO);

    // 3. Process fill trade
    let fill_trade = MarketTrade::new(
        "BTCUSDT",
        999,
        dec!(61400.0),
        dec!(0.1),
        true,
        breakout_t + 3_601_000,
    )
    .unwrap();
    engine
        .handle_market_event(LiveMarketEvent::Trade(fill_trade), breakout_t + 3_601_000)
        .unwrap();

    assert!(engine.simulator.working_order.is_none());
    assert!(engine.simulator.active_position.is_some());

    let db_pos = engine.db.get_position("BTCUSDT").unwrap();
    assert!(db_pos.is_some());
    assert_eq!(db_pos.unwrap().entry_price, dec!(61400.0));

    // Clean up
    let _ = fs::remove_file("data/test_signal_engine.db");
}
