use alpha_core::turtle::{
    evaluate_turtle_entry, evaluate_turtle_exit, TurtleConfig, TurtleExitReason, TurtleVerdict,
};
use alpha_types::Bar;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn make_test_bar(open: Decimal, high: Decimal, low: Decimal, close: Decimal) -> Bar {
    Bar::new(
        0,
        60_000,
        open,
        high,
        low,
        close,
        dec!(100),
        dec!(60),
        dec!(1000),
        10,
    )
    .expect("Valid bar")
}

#[test]
fn test_turtle_entry_triggers_on_channel_breakout() {
    let config = TurtleConfig::default();
    let mut bars = Vec::new();

    // 21 bars in a [90, 100] range
    for _ in 0..21 {
        bars.push(make_test_bar(dec!(95), dec!(100), dec!(90), dec!(95)));
    }

    // Bar 22 breaks out to 105
    bars.push(make_test_bar(dec!(101), dec!(105), dec!(99), dec!(104)));

    let verdict = evaluate_turtle_entry(&bars, &config).expect("Evaluation succeeds");
    match verdict {
        TurtleVerdict::EnterLong {
            entry_price,
            stop_price,
            entry_level,
            ..
        } => {
            assert_eq!(entry_level, dec!(100));
            assert_eq!(entry_price, dec!(101));
            assert!(stop_price < entry_price);
        }
        _ => panic!("Expected EnterLong verdict"),
    }
}

#[test]
fn test_turtle_entry_abstains_when_inside_channel() {
    let config = TurtleConfig::default();
    let mut bars = Vec::new();

    for _ in 0..22 {
        bars.push(make_test_bar(dec!(95), dec!(100), dec!(90), dec!(95)));
    }

    let verdict = evaluate_turtle_entry(&bars, &config).expect("Evaluation succeeds");
    assert_eq!(
        verdict,
        TurtleVerdict::Abstain {
            reason: "PriceBelowEntryChannel"
        }
    );
}

#[test]
fn test_turtle_exit_triggers_on_stop_loss() {
    let config = TurtleConfig::default();
    let mut bars = Vec::new();

    for _ in 0..11 {
        bars.push(make_test_bar(dec!(100), dec!(105), dec!(95), dec!(102)));
    }

    let current_stop = dec!(96);
    let verdict = evaluate_turtle_exit(&bars, current_stop, &config).expect("Exit check succeeds");
    assert_eq!(
        verdict,
        TurtleVerdict::ExitLong {
            exit_price: dec!(96),
            reason: TurtleExitReason::StopLoss,
        }
    );
}

#[test]
fn test_turtle_exit_triggers_on_10day_low_break() {
    let config = TurtleConfig::default();
    let mut bars = Vec::new();

    // 10 bars with lows at 100
    for _ in 0..10 {
        bars.push(make_test_bar(dec!(105), dec!(110), dec!(100), dec!(108)));
    }

    // Bar 11 drops below 100
    bars.push(make_test_bar(dec!(99), dec!(102), dec!(97), dec!(98)));

    let current_stop = dec!(90); // Stop is far away
    let verdict = evaluate_turtle_exit(&bars, current_stop, &config).expect("Exit check succeeds");
    match verdict {
        TurtleVerdict::ExitLong { exit_price, reason } => {
            assert_eq!(reason, TurtleExitReason::TrailingChannel);
            assert_eq!(exit_price, dec!(99)); // min(exit_level=100, open=99)
        }
        _ => panic!("Expected ExitLong verdict"),
    }
}

#[test]
fn test_simulate_turtle_series_generates_trade() {
    use alpha_core::simulate_turtle_series;
    let config = TurtleConfig::default();
    let mut bars = Vec::new();

    // 21 bars range [90, 100]
    for _ in 0..21 {
        bars.push(make_test_bar(dec!(95), dec!(100), dec!(90), dec!(95)));
    }
    // Bar 22: breakout to 105
    bars.push(make_test_bar(dec!(101), dec!(105), dec!(99), dec!(104)));
    // Next 10 bars hold above 100
    for _ in 0..10 {
        bars.push(make_test_bar(dec!(103), dec!(106), dec!(101), dec!(104)));
    }
    // Bar 33: collapse below 101 to 95
    bars.push(make_test_bar(dec!(98), dec!(100), dec!(94), dec!(96)));

    let trades = simulate_turtle_series(&bars, &config, dec!(0.002)).expect("Simulation succeeds");
    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].entry_idx, 21);
    assert!(trades[0].net_return_pct < Decimal::ZERO);
}
