use alpha_core::carry::FundingCarryConfig;
use alpha_node::carry_signal::{evaluate_live_carry_step, LiveCarryVerdict};
use rust_decimal_macros::dec;

#[test]
fn test_live_carry_approved_entry() {
    let cfg = FundingCarryConfig::default();
    let verdict = evaluate_live_carry_step(
        "BTCUSDT",
        &cfg,
        dec!(0.0002), // ~21.9% APR > 12%
        dec!(0.0002),
        None,
        None,
        dec!(100.0),
        dec!(65000.0),
        dec!(65010.0), // Slight contango basis > -0.05%
        false,
        dec!(0.0),
        0,
    )
    .unwrap();

    match verdict {
        LiveCarryVerdict::Approved { sizing, apr, .. } => {
            assert!(apr > cfg.entry_apr);
            assert!(sizing.spot_notional >= dec!(5.0));
            assert_eq!(sizing.spot_qty, sizing.perp_qty);
        }
        _ => panic!("Expected approved carry entry"),
    }
}

#[test]
fn test_live_carry_basis_guard_rejects_backwardation() {
    let cfg = FundingCarryConfig::default();
    // Severe backwardation: perp is 64500 vs spot 65000 (-0.77% < -0.05%)
    let verdict = evaluate_live_carry_step(
        "BTCUSDT",
        &cfg,
        dec!(0.0002),
        dec!(0.0002),
        None,
        None,
        dec!(100.0),
        dec!(65000.0),
        dec!(64500.0),
        false,
        dec!(0.0),
        0,
    )
    .unwrap();

    match verdict {
        LiveCarryVerdict::Abstain { reason, .. } => {
            assert!(reason.contains("Spot-Perp basis"));
        }
        _ => panic!("Expected abstain due to basis backwardation"),
    }
}

#[test]
fn test_live_carry_predictive_pre_exit() {
    let cfg = FundingCarryConfig::default();
    // Valid pre-exit within 10 minutes
    let verdict = evaluate_live_carry_step(
        "BTCUSDT",
        &cfg,
        dec!(0.0001),
        dec!(0.0001),
        Some(dec!(-0.00010)),
        Some(5 * 60 * 1000), // 5 mins to settle
        dec!(100.0),
        dec!(65000.0),
        dec!(65000.0),
        true,
        dec!(0.001),
        0,
    )
    .unwrap();

    assert!(matches!(
        verdict,
        LiveCarryVerdict::PredictivePreExit { .. }
    ));

    // Negative time_ms (settlement elapsed) must NOT trigger pre-exit
    let verdict_post = evaluate_live_carry_step(
        "BTCUSDT",
        &cfg,
        dec!(0.0001),
        dec!(0.0001),
        Some(dec!(-0.00010)),
        Some(-5000), // Negative
        dec!(100.0),
        dec!(65000.0),
        dec!(65000.0),
        true,
        dec!(0.001),
        0,
    )
    .unwrap();

    assert!(!matches!(
        verdict_post,
        LiveCarryVerdict::PredictivePreExit { .. }
    ));
}
