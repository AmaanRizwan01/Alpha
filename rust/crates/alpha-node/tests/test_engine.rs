use alpha_node::config::AppConfig;
use alpha_node::engine::TradeEngine;
use alpha_node::ws::LiveMarketEvent;
use alpha_types::{MarketTrade, OrderIntent, OrderType, Side, Venue};
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
