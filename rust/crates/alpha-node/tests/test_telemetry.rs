use alpha_node::db::DbManager;
use alpha_node::status::{display_status, fetch_recent_orders};
use alpha_node::telegram::TelegramNotifier;
use alpha_types::{OrderIntent, OrderType, Side, Venue};
use rust_decimal_macros::dec;
use std::fs;

#[test]
fn test_telegram_notifier_unconfigured_safe() {
    let tg = TelegramNotifier::new(None, None);
    assert!(!tg.is_configured());
    tg.notify_startup("paper", &["BTCUSDT".to_string()], dec!(100.0));
    tg.notify_signal_approved(
        "BTCUSDT",
        dec!(85000.0),
        dec!(83000.0),
        dec!(2.35),
        dec!(0.001),
    );
    tg.notify_order_submitted(
        "BTCUSDT",
        "BUY",
        dec!(0.001),
        Some(dec!(85000.0)),
        "LimitMaker",
    );
    tg.notify_fill(
        "BTCUSDT",
        "BUY",
        dec!(0.001),
        dec!(85000.0),
        dec!(0.085),
        dec!(14.915),
    );
    tg.notify_position_closed("BTCUSDT", dec!(87000.0), dec!(2.0), dec!(1.0), "TakeProfit");
    tg.notify_circuit_breaker("Drawdown reached 6.0%", dec!(6.2));
}

#[test]
fn test_status_display_and_recent_orders() {
    let _ = fs::create_dir_all("data");
    let test_db = "data/test_status_telemetry.db";
    let _ = fs::remove_file(test_db);

    let db = DbManager::open(test_db).unwrap();
    let now_ms = 1_700_000_000_000;

    let intent = OrderIntent::new(
        Venue::Binance,
        "BTCUSDT",
        Side::Buy,
        OrderType::LimitMaker,
        Some(dec!(80000.0)),
        dec!(0.005),
        Some(dec!(78000.0)),
        None,
        "status-test-order-1",
    )
    .unwrap();
    db.insert_order(&intent, "WORKING", now_ms).unwrap();

    db.upsert_position(
        "BTCUSDT",
        "Binance",
        dec!(0.005),
        dec!(80000.0),
        dec!(78000.0),
        dec!(81000.0),
        false,
        2,
        now_ms,
    )
    .unwrap();

    let orders = fetch_recent_orders(test_db, 5).unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].symbol, "BTCUSDT");
    assert_eq!(orders[0].status, "WORKING");

    let res = display_status(test_db);
    assert!(res.is_ok());

    let _ = fs::remove_file(test_db);
}
