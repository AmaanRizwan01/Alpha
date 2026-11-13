use alpha_node::db::{DbManager, PositionRecord};
use alpha_node::reconcile::{reconcile_cold_boot, ReconcileOutcome};
use alpha_types::{OrderIntent, OrderType, Side, Venue};
use alpha_venues::binance::build_binance_atomic_otoco;
use alpha_venues::okx::{build_okx_atomic_order, OkxDeadManPayload};
use rust_decimal_macros::dec;

#[test]
fn drill_1_atomic_stop_attached_at_payload_level() {
    let intent_okx = OrderIntent::new(
        Venue::Okx,
        "BTC-USDT",
        Side::Buy,
        OrderType::LimitMaker,
        Some(dec!(65000.0)),
        dec!(0.05),
        Some(dec!(63500.0)),
        None,
        "cl-drill-001",
    )
    .unwrap();

    let okx_payload = build_okx_atomic_order(&intent_okx).unwrap();
    assert_eq!(okx_payload.attach_algo_ords.len(), 1);
    let algo = &okx_payload.attach_algo_ords[0];
    assert_eq!(algo.sl_trigger_px.as_deref(), Some("63500.0"));
    assert_eq!(algo.sl_ord_px.as_deref(), Some("-1"));

    let intent_binance = OrderIntent::new(
        Venue::Binance,
        "BTCUSDT",
        Side::Buy,
        OrderType::LimitMaker,
        Some(dec!(65000.0)),
        dec!(0.05),
        Some(dec!(63500.0)),
        None,
        "cl-drill-002",
    )
    .unwrap();

    let binance_params = build_binance_atomic_otoco(&intent_binance, 10).unwrap();
    assert_eq!(binance_params.pending_below_type, "STOP_LOSS_LIMIT");
    assert_eq!(binance_params.pending_below_stop_price, dec!(63500.0));
    assert_eq!(binance_params.pending_quantity, dec!(0.05));
}

#[test]
fn drill_2_offline_stop_execution_recovery() {
    let db = DbManager::open_in_memory().unwrap();
    db.upsert_position(
        "BTCUSDT",
        "Binance",
        dec!(0.05),
        dec!(65000.0),
        dec!(63500.0),
        dec!(66000.0),
        false,
        2,
        1000,
    )
    .unwrap();

    let outcome = reconcile_cold_boot(&db, "BTCUSDT", None, false, 2000).unwrap();
    assert_eq!(
        outcome,
        ReconcileOutcome::PositionClosedOffline {
            symbol: "BTCUSDT".into()
        }
    );
    assert!(db.get_position("BTCUSDT").unwrap().is_none());
}

#[test]
fn drill_3_rogue_unhedged_coin_balance_trips_halt() {
    let db = DbManager::open_in_memory().unwrap();
    let rogue_pos = PositionRecord {
        symbol: "BTCUSDT".into(),
        venue: "Binance".into(),
        qty: dec!(0.05),
        entry_price: dec!(65000.0),
        current_stop_price: dec!(63500.0),
        highest_price: dec!(65000.0),
        breakeven_active: false,
        bars_held: 0,
    };

    let outcome = reconcile_cold_boot(&db, "BTCUSDT", Some(&rogue_pos), false, 1500).unwrap();
    assert_eq!(
        outcome,
        ReconcileOutcome::UnhedgedPositionDetected {
            symbol: "BTCUSDT".into(),
            qty: dec!(0.05)
        }
    );

    let (halted, reason) = db.get_halt_state().unwrap();
    assert!(halted);
    assert!(reason.unwrap().contains("unhedged exchange position"));
}

#[test]
fn drill_4_drawdown_circuit_breaker_persists_across_restart() {
    let db = DbManager::open_in_memory().unwrap();
    let (hwm, _) = db.update_equity_hwm(dec!(100.0), 1000).unwrap();
    assert_eq!(hwm, dec!(100.0));

    let (_, dd) = db.update_equity_hwm(dec!(93.5), 2000).unwrap();
    assert_eq!(dd, dec!(6.5));

    let (halted, _) = db.get_halt_state().unwrap();
    assert!(halted);

    let outcome = reconcile_cold_boot(&db, "BTCUSDT", None, false, 3000).unwrap();
    assert!(matches!(outcome, ReconcileOutcome::Halted { .. }));
}

#[test]
fn drill_5_dead_man_switch_timeout_bounds() {
    assert!(OkxDeadManPayload::new(5).is_err());
    assert!(OkxDeadManPayload::new(150).is_err());
    let payload = OkxDeadManPayload::new(60).unwrap();
    let json = serde_json::to_string(&payload).unwrap();
    assert_eq!(json, r#"{"timeOut":"60"}"#);
}

#[test]
fn drill_6_adl_desync_watchdog_triggers_halt_and_emergency_action() {
    let db = DbManager::open_in_memory().unwrap();
    let tg = alpha_node::telegram::TelegramNotifier::new(None, None);

    // Initial position: 1.0 BTC spot long hedged by 1.0 BTC perp short
    let initial_check =
        alpha_node::adl_watchdog::check_position_adl_desync("BTCUSDT", dec!(1.0), dec!(-1.0), None);
    assert!(!initial_check.is_desynced);

    // ADL liquidation closes 5% of perp short: perp drops to -0.95 -> desync!
    let desync_check = alpha_node::adl_watchdog::check_position_adl_desync(
        "BTCUSDT",
        dec!(1.0),
        dec!(-0.95),
        None,
    );
    assert!(desync_check.is_desynced);

    // Trigger watchdog action: must persist halt and notify Telegram
    alpha_node::adl_watchdog::handle_adl_desync_trigger(
        &db,
        &tg,
        &desync_check.symbol,
        desync_check.spot_qty,
        desync_check.perp_qty,
        4000,
    )
    .unwrap();

    let (halted, reason) = db.get_halt_state().unwrap();
    assert!(halted);
    let r = reason.unwrap();
    assert!(r.contains("HALTED_ADL_DESYNC"));
    assert!(r.contains("spot_qty=1.0"));
    assert!(r.contains("perp_qty=-0.95"));
}
