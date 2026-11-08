use alpha_core::exits::ExitConfig;
use alpha_node::simulator::{PaperFill, PaperSimulator};
use alpha_types::{Bar, MarketTrade, OrderIntent, OrderType, Side, Venue};
use rust_decimal_macros::dec;

fn make_test_order() -> OrderIntent {
    OrderIntent::new(
        Venue::Binance,
        "BTCUSDT",
        Side::Buy,
        OrderType::LimitMaker,
        Some(dec!(60000.0)),
        dec!(0.001),
        Some(dec!(58500.0)),
        Some(dec!(63000.0)),
        "test-ord-1",
    )
    .unwrap()
}

#[test]
fn test_unreached_order_does_not_fill() {
    let mut sim = PaperSimulator::new(dec!(100.0), 0, 10, 5);
    sim.submit_order(make_test_order()).unwrap();

    let trade = MarketTrade::new("BTCUSDT", 1, dec!(60010.0), dec!(0.1), false, 1000).unwrap();
    let fill = sim.process_trade(&trade).unwrap();
    assert!(fill.is_none());
    assert!(sim.working_order.is_some());
    assert!(sim.active_position.is_none());
}

#[test]
fn test_maker_entry_and_stop_loss_trigger() {
    let mut sim = PaperSimulator::new(dec!(100.0), 0, 10, 5);
    sim.submit_order(make_test_order()).unwrap();

    // 1. Trade hits 60000 -> fills entry at 60000
    let entry_trade = MarketTrade::new("BTCUSDT", 1, dec!(60000.0), dec!(0.1), true, 1000).unwrap();
    let fill = sim.process_trade(&entry_trade).unwrap();
    assert!(matches!(fill, Some(PaperFill::Entry { .. })));
    assert_eq!(sim.cash_balance, dec!(40.0)); // 100 - (0.001 * 60000)
    assert!(sim.active_position.is_some());

    // 2. Price dumps below stop of 58500
    let dump_trade = MarketTrade::new("BTCUSDT", 2, dec!(58400.0), dec!(0.1), false, 2000).unwrap();
    let stop_fill = sim.process_trade(&dump_trade).unwrap();
    assert!(matches!(stop_fill, Some(PaperFill::StopLoss { .. })));
    assert!(sim.active_position.is_none());
    assert!(sim.cash_balance > dec!(98.0) && sim.cash_balance < dec!(99.0));
}

#[test]
fn test_take_profit_fill() {
    let mut sim = PaperSimulator::new(dec!(100.0), 0, 10, 5);
    sim.submit_order(make_test_order()).unwrap();

    let entry_trade = MarketTrade::new("BTCUSDT", 1, dec!(59990.0), dec!(0.1), true, 1000).unwrap();
    sim.process_trade(&entry_trade).unwrap();

    // Hits TP of 63000
    let pump_trade = MarketTrade::new("BTCUSDT", 2, dec!(63100.0), dec!(0.1), true, 2000).unwrap();
    let tp_fill = sim.process_trade(&pump_trade).unwrap();
    assert!(matches!(tp_fill, Some(PaperFill::TakeProfit { .. })));
    assert!(sim.active_position.is_none());
    assert!(sim.cash_balance > dec!(102.0));
}

#[test]
fn test_process_bar_trailing_stop_exit() {
    let mut sim = PaperSimulator::new(dec!(100.0), 0, 10, 5);
    sim.submit_order(make_test_order()).unwrap();

    let entry_trade = MarketTrade::new("BTCUSDT", 1, dec!(60000.0), dec!(0.1), true, 1000).unwrap();
    sim.process_trade(&entry_trade).unwrap();

    let exit_cfg = ExitConfig::default();
    // Bar with low dropping below stop
    let dump_bar = Bar::new(
        1000,
        2000,
        dec!(59000.0),
        dec!(59500.0),
        dec!(58000.0), // Low < 58500 initial stop
        dec!(58200.0),
        dec!(10.0),
        dec!(4.0),
        dec!(585000.0),
        100,
    )
    .unwrap();

    let bar_fill = sim
        .process_bar("BTCUSDT", &dump_bar, dec!(500.0), &exit_cfg)
        .unwrap();
    assert!(matches!(bar_fill, Some(PaperFill::BarExit { .. })));
    assert!(sim.active_position.is_none());
}
