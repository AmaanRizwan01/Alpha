use alpha_node::db::{DbManager, PositionRecord};
use alpha_node::sub_account::{
    delete_sub_account_position, fetch_sub_account_orders, get_all_sub_account_positions,
    get_portfolio_summary, get_sub_account, get_sub_account_positions, insert_sub_account_order,
    list_sub_accounts, update_sub_account_equity, update_sub_account_order_status,
    upsert_sub_account_position,
};
use alpha_node::telegram_listener::dispatch_command;
use alpha_types::{OrderIntent, OrderType, Side, Venue};
use rust_decimal_macros::dec;
use std::fs;

#[test]
fn test_sub_account_initial_seed_and_listing() {
    let _ = fs::create_dir_all("data");
    let test_db = "data/test_sub_accounts_seed.db";
    let _ = fs::remove_file(test_db);

    let db = DbManager::open(test_db).unwrap();
    let subs = list_sub_accounts(db.conn()).unwrap();
    assert_eq!(subs.len(), 3);

    let names: Vec<&str> = subs.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"alphabreakout"));
    assert!(names.contains(&"alphacarry"));
    assert!(names.contains(&"alphamacro"));

    for s in &subs {
        assert_eq!(s.current_equity, dec!(100.0));
        assert_eq!(s.high_water_mark, dec!(100.0));
        assert_eq!(s.max_drawdown_pct, dec!(0.0));
        assert!(!s.is_halted);
        assert!(s.halt_reason.is_none());
    }

    drop(db);
    let _ = fs::remove_file(test_db);
    let _ = fs::remove_file(format!("{test_db}-wal"));
    let _ = fs::remove_file(format!("{test_db}-shm"));
}

#[test]
fn test_sub_account_isolated_positions_same_symbol() {
    let _ = fs::create_dir_all("data");
    let test_db = "data/test_sub_accounts_pos.db";
    let _ = fs::remove_file(test_db);

    let db = DbManager::open(test_db).unwrap();
    let now_ms = 1_700_000_000_000;

    // Sub-Account 1: Breakout Long BTC
    let pos_bo = PositionRecord {
        symbol: "BTCUSDT".to_string(),
        venue: "OKX".to_string(),
        qty: dec!(0.005),
        entry_price: dec!(85000.0),
        current_stop_price: dec!(83000.0),
        highest_price: dec!(86000.0),
        breakeven_active: false,
        bars_held: 2,
    };
    upsert_sub_account_position(db.conn(), "alphabreakout", &pos_bo, now_ms).unwrap();

    // Sub-Account 2: Carry Delta-Neutral Spot BTC
    let pos_carry = PositionRecord {
        symbol: "BTCUSDT".to_string(),
        venue: "OKX".to_string(),
        qty: dec!(0.010),
        entry_price: dec!(85100.0),
        current_stop_price: dec!(85000.0),
        highest_price: dec!(85200.0),
        breakeven_active: false,
        bars_held: 5,
    };
    upsert_sub_account_position(db.conn(), "alphacarry", &pos_carry, now_ms).unwrap();

    // Verify independent positions for the exact same symbol across sub-accounts
    let bo_positions = get_sub_account_positions(db.conn(), "alphabreakout").unwrap();
    assert_eq!(bo_positions.len(), 1);
    assert_eq!(bo_positions[0].qty, dec!(0.005));

    let carry_positions = get_sub_account_positions(db.conn(), "alphacarry").unwrap();
    assert_eq!(carry_positions.len(), 1);
    assert_eq!(carry_positions[0].qty, dec!(0.010));

    let macro_positions = get_sub_account_positions(db.conn(), "alphamacro").unwrap();
    assert_eq!(macro_positions.len(), 0);

    let all_pos = get_all_sub_account_positions(db.conn()).unwrap();
    assert_eq!(all_pos.len(), 2);

    delete_sub_account_position(db.conn(), "alphabreakout", "BTCUSDT").unwrap();
    let bo_after = get_sub_account_positions(db.conn(), "alphabreakout").unwrap();
    assert_eq!(bo_after.len(), 0);
    // Carry position untouched
    let carry_after = get_sub_account_positions(db.conn(), "alphacarry").unwrap();
    assert_eq!(carry_after.len(), 1);

    drop(db);
    let _ = fs::remove_file(test_db);
    let _ = fs::remove_file(format!("{test_db}-wal"));
    let _ = fs::remove_file(format!("{test_db}-shm"));
}

#[test]
fn test_sub_account_isolated_circuit_breaker() {
    let _ = fs::create_dir_all("data");
    let test_db = "data/test_sub_accounts_cb.db";
    let _ = fs::remove_file(test_db);

    let db = DbManager::open(test_db).unwrap();
    let now_ms = 1_700_000_000_000;

    // alphabreakout takes a 7% loss ($100 -> $93)
    let (hwm, dd) =
        update_sub_account_equity(db.conn(), "alphabreakout", dec!(93.0), now_ms).unwrap();
    assert_eq!(hwm, dec!(100.0));
    assert_eq!(dd, dec!(7.0));

    let bo_rec = get_sub_account(db.conn(), "alphabreakout")
        .unwrap()
        .unwrap();
    assert!(bo_rec.is_halted);
    assert!(bo_rec.halt_reason.unwrap().contains("circuit breaker"));

    // Verify other sub-accounts are completely unaffected
    let carry_rec = get_sub_account(db.conn(), "alphacarry").unwrap().unwrap();
    assert!(!carry_rec.is_halted);
    assert_eq!(carry_rec.current_equity, dec!(100.0));

    let macro_rec = get_sub_account(db.conn(), "alphamacro").unwrap().unwrap();
    assert!(!macro_rec.is_halted);
    assert_eq!(macro_rec.current_equity, dec!(100.0));

    // Verify portfolio summary reflects the sum
    let summary = get_portfolio_summary(db.conn()).unwrap();
    assert_eq!(summary.total_equity, dec!(293.0));
    assert_eq!(summary.total_hwm, dec!(300.0));

    drop(db);
    let _ = fs::remove_file(test_db);
    let _ = fs::remove_file(format!("{test_db}-wal"));
    let _ = fs::remove_file(format!("{test_db}-shm"));
}

#[test]
fn test_sub_account_orders_and_telegram_telemetry() {
    let _ = fs::create_dir_all("data");
    let test_db = "data/test_sub_accounts_tg.db";
    let _ = fs::remove_file(test_db);

    let db = DbManager::open(test_db).unwrap();
    let now_ms = 1_700_000_000_000;

    let intent = OrderIntent::new(
        Venue::Okx,
        "BTCUSDT",
        Side::Buy,
        OrderType::LimitMaker,
        Some(dec!(84000.0)),
        dec!(0.002),
        Some(dec!(82000.0)),
        None,
        "sub-ord-1",
    )
    .unwrap();
    insert_sub_account_order(db.conn(), "alphabreakout", &intent, "FILLED", now_ms).unwrap();

    let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
    let status_msg = dispatch_command("/status", test_db, "paper", &symbols);
    assert!(status_msg.contains("Sub-Accounts (OKX):"));
    assert!(status_msg.contains("alphabreakout"));
    assert!(status_msg.contains("alphacarry"));
    assert!(status_msg.contains("alphamacro"));

    let pnl_msg = dispatch_command("/pnl", test_db, "paper", &symbols);
    assert!(pnl_msg.contains("Sub-Account Balances:"));
    assert!(pnl_msg.contains("alphabreakout"));
    assert!(pnl_msg.contains("$100.0"));

    // Verify @bot_name command routing
    let status_bot = dispatch_command("/status@alpha_trading_bot", test_db, "paper", &symbols);
    assert!(status_bot.contains("Sub-Accounts (OKX):"));

    drop(db);
    let _ = fs::remove_file(test_db);
    let _ = fs::remove_file(format!("{test_db}-wal"));
    let _ = fs::remove_file(format!("{test_db}-shm"));
}

#[test]
fn test_sub_account_order_status_update_and_flatten_all_subs() {
    let _ = fs::create_dir_all("data");
    let test_db = "data/test_sub_accounts_flatten.db";
    let _ = fs::remove_file(test_db);

    let db = DbManager::open(test_db).unwrap();
    let now_ms = 1_700_000_000_000;

    let intent = OrderIntent::new(
        Venue::Okx,
        "BTCUSDT",
        Side::Buy,
        OrderType::LimitMaker,
        Some(dec!(84000.0)),
        dec!(0.002),
        Some(dec!(82000.0)),
        None,
        "sub-ord-pending-1",
    )
    .unwrap();
    insert_sub_account_order(db.conn(), "alphabreakout", &intent, "WORKING", now_ms).unwrap();

    let orders = fetch_sub_account_orders(db.conn(), "alphabreakout", 5).unwrap();
    assert_eq!(orders.len(), 1);
    assert_eq!(orders[0].status, "WORKING");

    // Transition to FILLED
    update_sub_account_order_status(db.conn(), "sub-ord-pending-1", "FILLED", now_ms + 1000)
        .unwrap();
    let orders_after = fetch_sub_account_orders(db.conn(), "alphabreakout", 5).unwrap();
    assert_eq!(orders_after[0].status, "FILLED");

    // Test operator flatten via telegram halts all sub-accounts
    let symbols = vec!["BTCUSDT".to_string(), "ETHUSDT".to_string()];
    let flatten_resp = dispatch_command("/flatten", test_db, "paper", &symbols);
    assert!(flatten_resp.contains("FLATTEN ENGAGED"));

    let subs = list_sub_accounts(db.conn()).unwrap();
    for s in &subs {
        assert!(
            s.is_halted,
            "Sub-account {} should be halted on flatten",
            s.name
        );
        assert_eq!(s.halt_reason.as_deref(), Some("operator telegram flatten"));
    }

    let status_after = dispatch_command("/status", test_db, "paper", &symbols);
    assert!(status_after.contains("HALTED"));
    assert!(status_after.contains("alphabreakout"));
    assert!(status_after.contains("HALTED"));

    drop(db);
    let _ = fs::remove_file(test_db);
    let _ = fs::remove_file(format!("{test_db}-wal"));
    let _ = fs::remove_file(format!("{test_db}-shm"));
}
