use alpha_core::pullback::{
    evaluate_pullback_bar_exit, PullbackConfig, PullbackExitReason, PullbackExitState,
    PullbackVerdict,
};
use alpha_core::pullback_sim::simulate_pullback_series;
use alpha_types::Bar;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn make_test_bar(t: i64, o: Decimal, h: Decimal, l: Decimal, c: Decimal) -> Bar {
    Bar::new(
        t,
        t + 86_400_000,
        o,
        h,
        l,
        c,
        dec!(1000),
        dec!(500),
        dec!(50_000),
        100,
    )
    .expect("Valid bar")
}

#[test]
fn test_pullback_config_validation() {
    let cfg = PullbackConfig::default();
    assert_eq!(cfg.trend_sma_period, 200);
    assert_eq!(cfg.rsi_period, 3);
    assert_eq!(cfg.max_hold_bars, 40);

    let err = PullbackConfig::new(
        0,
        3,
        dec!(30),
        14,
        dec!(2),
        dec!(0.02),
        dec!(1),
        dec!(0.15),
        10,
        40,
    );
    assert!(err.is_err());
}

#[test]
fn test_pullback_exit_stop_loss() {
    let cfg = PullbackConfig::default();
    let bar = make_test_bar(1000, dec!(100), dec!(102), dec!(94), dec!(96));
    let state = PullbackExitState {
        entry_price: dec!(100),
        stop_price: dec!(95),
        highest_price: dec!(100),
        stop_distance: dec!(0.05),
        ratcheted: false,
        bars_held: 2,
        ema_val: dec!(98),
        next_open: dec!(96),
    };
    let verdict = evaluate_pullback_bar_exit(&bar, &state, &cfg);
    match verdict {
        PullbackVerdict::ExitLong { exit_price, reason } => {
            assert_eq!(reason, PullbackExitReason::StopLoss);
            assert_eq!(exit_price, dec!(95));
        }
        _ => panic!("Expected StopLoss exit"),
    }
}

#[test]
fn test_pullback_exit_stagnation() {
    let cfg = PullbackConfig::default();
    let bar = make_test_bar(1000, dec!(100), dec!(103), dec!(98), dec!(101));
    let state = PullbackExitState {
        entry_price: dec!(100),
        stop_price: dec!(95),
        highest_price: dec!(103),
        stop_distance: dec!(0.05),
        ratcheted: false,
        bars_held: 40,
        ema_val: dec!(99),
        next_open: dec!(102),
    };
    let verdict = evaluate_pullback_bar_exit(&bar, &state, &cfg);
    match verdict {
        PullbackVerdict::ExitLong { exit_price, reason } => {
            assert_eq!(reason, PullbackExitReason::Stagnation);
            assert_eq!(exit_price, dec!(102));
        }
        _ => panic!("Expected Stagnation exit"),
    }
}

#[test]
fn test_pullback_ratchet_activation() {
    let cfg = PullbackConfig::default();
    // Entry at 100, stop distance 5% (stop at 95). Risk dollar = 5.0.
    // +1.0R excursion requires price to reach 105. High = 106 triggers ratchet.
    let bar = make_test_bar(1000, dec!(100), dec!(106), dec!(99), dec!(105));
    let state = PullbackExitState {
        entry_price: dec!(100),
        stop_price: dec!(95),
        highest_price: dec!(100),
        stop_distance: dec!(0.05),
        ratcheted: false,
        bars_held: 3,
        ema_val: dec!(98),
        next_open: dec!(105),
    };
    let verdict = evaluate_pullback_bar_exit(&bar, &state, &cfg);
    match verdict {
        PullbackVerdict::Hold {
            stop_price,
            ratcheted,
            ..
        } => {
            assert!(ratcheted);
            // Stop locked at entry * (1 + 0.15 * 0.05) = 100 * 1.0075 = 100.75
            assert_eq!(stop_price, dec!(100.75));
        }
        _ => panic!("Expected Hold with ratcheted stop"),
    }
}

#[test]
fn test_pullback_series_empty_when_insufficient_bars() {
    let cfg = PullbackConfig::default();
    let bars: Vec<Bar> = (0..50)
        .map(|i| make_test_bar(i * 86_400_000, dec!(100), dec!(105), dec!(95), dec!(102)))
        .collect();
    let trades = simulate_pullback_series(&bars, &cfg, dec!(0.0015)).expect("Simulation runs");
    assert!(trades.is_empty());
}
