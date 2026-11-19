use alpha_core::exits::{calculate_initial_stop, ActiveTrade, ExitConfig, ExitReason, ExitVerdict};
use alpha_types::Bar;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn make_test_bar(high: Decimal, low: Decimal, close: Decimal) -> Bar {
    Bar::new(
        0,
        1,
        close,
        high,
        low,
        close,
        dec!(10),
        dec!(5),
        dec!(100),
        10,
    )
    .unwrap()
}

#[test]
fn initial_stop_enforces_minimum_distance() {
    let config = ExitConfig::default(); // min_stop_distance_pct = 0.022 (2.20%)
    let entry = dec!(100);
    let tiny_atr = dec!(0.5); // 2.0 * 0.5 = 1.0 (1.0% stop, too tight)
    let stop = calculate_initial_stop(entry, tiny_atr, &config).unwrap();
    // Clamped to 100 * (1 - 0.022) = 97.80
    assert_eq!(stop, dec!(97.80));
}

#[test]
fn stop_loss_hit_triggers_exit() {
    let config = ExitConfig::default();
    let mut trade = ActiveTrade::new(dec!(100), dec!(97.5)).unwrap();
    let bar = make_test_bar(dec!(101), dec!(97.0), dec!(97.2));
    let verdict = trade.update_bar(&bar, dec!(1.0), &config).unwrap();

    assert_eq!(
        verdict,
        ExitVerdict::Exit {
            reason: ExitReason::StopLossHit,
            exit_price: dec!(97.5),
            bars_held: 1,
        }
    );
}

#[test]
fn chandelier_trail_ratchets_monotonically() {
    let config = ExitConfig::default();
    let mut trade = ActiveTrade::new(dec!(100), dec!(97.0)).unwrap();

    // Bar 1 surges to high 110, ATR = 2.0 => trail = 110 - (2.5 * 2) = 105.0
    let b1 = make_test_bar(dec!(110), dec!(100), dec!(108));
    let v1 = trade.update_bar(&b1, dec!(2.0), &config).unwrap();
    assert_eq!(
        v1,
        ExitVerdict::Hold {
            current_stop: dec!(105.0),
            highest_high: dec!(110),
            bars_held: 1,
        }
    );

    // Bar 2 pulls back to high 107, ATR = 2.5 => trail would be 107 - 6.25 = 100.75
    // Monotonic ratchet preserves stop at 105.0!
    let b2 = make_test_bar(dec!(107), dec!(106), dec!(106.5));
    let v2 = trade.update_bar(&b2, dec!(2.5), &config).unwrap();
    assert_eq!(
        v2,
        ExitVerdict::Hold {
            current_stop: dec!(105.0),
            highest_high: dec!(110),
            bars_held: 2,
        }
    );
}

#[test]
fn e1_stagnation_cuts_stalled_breakout() {
    let config = ExitConfig {
        enable_e1_stagnation: true,
        stagnation_bars: 3,
        ..Default::default()
    };

    // Entry 100, Stop 98 => 1R = 2.0
    let mut trade = ActiveTrade::new(dec!(100), dec!(98)).unwrap();

    // 2 bars of stalling around 100.2 (MFE = +0.1R)
    let b1 = make_test_bar(dec!(100.3), dec!(99.8), dec!(100.1));
    trade.update_bar(&b1, dec!(1.0), &config).unwrap();
    let b2 = make_test_bar(dec!(100.4), dec!(99.9), dec!(100.0));
    trade.update_bar(&b2, dec!(1.0), &config).unwrap();

    // 3rd bar still stagnant at close 99.8 (-0.1R)
    let b3 = make_test_bar(dec!(100.2), dec!(99.5), dec!(99.8));
    let v3 = trade.update_bar(&b3, dec!(1.0), &config).unwrap();

    assert_eq!(
        v3,
        ExitVerdict::Exit {
            reason: ExitReason::StagnationTimeStop,
            exit_price: dec!(99.8),
            bars_held: 3,
        }
    );
}

#[test]
fn e2_breakeven_ratchets_after_positive_excursion() {
    let config = ExitConfig {
        enable_e2_breakeven: true,
        ..Default::default()
    };

    // Entry 100, Stop 96 => 1R = 4.0. Trigger threshold = 100 + (1.25 * 4) = 105.0
    let mut trade = ActiveTrade::new(dec!(100), dec!(96)).unwrap();

    // Bar reaches high of 106 (> 105.0), but ATR is huge (5.0) so Chandelier trail = 106 - 12.5 = 93.5
    // E2 raises stop to 100 * 1.002 = 100.20
    let bar = make_test_bar(dec!(106), dec!(101), dec!(103));
    let v = trade.update_bar(&bar, dec!(5.0), &config).unwrap();

    assert_eq!(
        v,
        ExitVerdict::Hold {
            current_stop: dec!(100.20),
            highest_high: dec!(106),
            bars_held: 1,
        }
    );
    assert!(trade.is_breakeven_active);
}
