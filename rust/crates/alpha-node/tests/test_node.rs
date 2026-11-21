use alpha_node::db::{DbManager, PositionRecord};
use alpha_node::reconcile::{reconcile_cold_boot, ReconcileOutcome};
use alpha_types::{OrderIntent, OrderType, Side, Venue};
use rust_decimal_macros::dec;

#[test]
fn test_migrations_and_halt_state() {
    let db = DbManager::open_in_memory().unwrap();
    let (halted, reason) = db.get_halt_state().unwrap();
    assert!(!halted);
    assert!(reason.is_none());

    db.set_halt_state(true, Some("manual operator halt"), 1700000)
        .unwrap();
    let (halted, reason) = db.get_halt_state().unwrap();
    assert!(halted);
    assert_eq!(reason.as_deref(), Some("manual operator halt"));
}

#[test]
fn test_order_and_position_lifecycle() {
    let db = DbManager::open_in_memory().unwrap();
    let intent = OrderIntent::new(
        Venue::Binance,
        "BTCUSDT",
        Side::Buy,
        OrderType::LimitMaker,
        Some(dec!(65000.0)),
        dec!(0.05),
        Some(dec!(63500.0)),
        None,
        "ord-test-01",
    )
    .unwrap();

    db.insert_order(&intent, "SUBMITTED", 1000).unwrap();
    db.update_order_status("ord-test-01", "FILLED", 1050)
        .unwrap();

    db.upsert_position(
        "BTCUSDT",
        "Binance",
        dec!(0.05),
        dec!(65000.0),
        dec!(63500.0),
        dec!(66000.0),
        false,
        1,
        1050,
    )
    .unwrap();

    let pos = db.get_position("BTCUSDT").unwrap().unwrap();
    assert_eq!(pos.qty, dec!(0.05));
    assert_eq!(pos.entry_price, dec!(65000.0));
    assert_eq!(pos.highest_price, dec!(66000.0));

    db.delete_position("BTCUSDT").unwrap();
    assert!(db.get_position("BTCUSDT").unwrap().is_none());
}

#[test]
fn test_drawdown_circuit_breaker_trips() {
    let db = DbManager::open_in_memory().unwrap();
    let (hwm, dd) = db.update_equity_hwm(dec!(100.0), 1000).unwrap();
    assert_eq!(hwm, dec!(100.0));
    assert_eq!(dd, dec!(0.0));
    assert!(!db.get_halt_state().unwrap().0);

    let (hwm, dd) = db.update_equity_hwm(dec!(110.0), 2000).unwrap();
    assert_eq!(hwm, dec!(110.0));
    assert_eq!(dd, dec!(0.0));

    let (_, dd) = db.update_equity_hwm(dec!(103.0), 3000).unwrap();
    assert!(dd >= dec!(6.0));

    let (halted, reason) = db.get_halt_state().unwrap();
    assert!(halted);
    assert!(reason.unwrap().contains("circuit breaker"));
}

#[test]
fn test_reconcile_cold_boot_matrix() {
    let db = DbManager::open_in_memory().unwrap();
    // 1. Both flat -> Synchronized
    let res = reconcile_cold_boot(&db, "BTCUSDT", None, false, 1000).unwrap();
    assert_eq!(res, ReconcileOutcome::Synchronized);

    // 2. Position closed while offline
    db.upsert_position(
        "BTCUSDT",
        "Binance",
        dec!(0.05),
        dec!(65000.0),
        dec!(63500.0),
        dec!(66000.0),
        false,
        1,
        1000,
    )
    .unwrap();
    let res = reconcile_cold_boot(&db, "BTCUSDT", None, false, 2000).unwrap();
    assert_eq!(
        res,
        ReconcileOutcome::PositionClosedOffline {
            symbol: "BTCUSDT".into()
        }
    );

    // 3. Unhedged position alert
    let v_pos = PositionRecord {
        symbol: "BTCUSDT".into(),
        venue: "Binance".into(),
        qty: dec!(0.05),
        entry_price: dec!(65000.0),
        current_stop_price: dec!(63500.0),
        highest_price: dec!(65000.0),
        breakeven_active: false,
        bars_held: 0,
    };
    let res = reconcile_cold_boot(&db, "BTCUSDT", Some(&v_pos), false, 3000).unwrap();
    assert_eq!(
        res,
        ReconcileOutcome::UnhedgedPositionDetected {
            symbol: "BTCUSDT".into(),
            qty: dec!(0.05)
        }
    );
}
