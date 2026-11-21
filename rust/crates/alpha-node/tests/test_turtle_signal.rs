use alpha_core::risk::RiskPolicy;
use alpha_core::sizing::FeeBudget;
use alpha_core::turtle::TurtleConfig;
use alpha_node::buffer::BarBuffer;
use alpha_node::signal::{default_filter_for_symbol, LiveSignalVerdict};
use alpha_node::turtle_signal::evaluate_turtle_bar;
use alpha_types::Bar;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn build_bars(n: usize, start_px: Decimal, step: Decimal) -> Vec<Bar> {
    (0..n)
        .map(|i| {
            let px = start_px + (step * Decimal::from(i));
            let t = (i as i64) * 86_400_000;
            Bar::new(
                t,
                t + 86_400_000,
                px,
                px + dec!(50.0),
                px - dec!(20.0),
                px,
                dec!(1000.0),
                dec!(600.0),
                dec!(50_000_000.0),
                500,
            )
            .expect("Valid bar")
        })
        .collect()
}

#[test]
fn test_turtle_insufficient_bars_verdict() {
    let buffer = BarBuffer::new(50).expect("Buffer creates");
    let turtle_cfg = TurtleConfig::default();
    let fee_budget = FeeBudget::new(dec!(0.0008), dec!(0.0010), dec!(0.0003)).expect("Valid fees");
    let filter = default_filter_for_symbol("BTCUSDT");
    let risk_policy = RiskPolicy::new(dec!(0.50), dec!(0.06)).expect("Valid risk");

    let verdict = evaluate_turtle_bar(
        "BTCUSDT",
        &buffer,
        dec!(100.0),
        dec!(100.0),
        dec!(0.010),
        &turtle_cfg,
        &fee_budget,
        &filter,
        &risk_policy,
    )
    .expect("Evaluation succeeds");

    assert_eq!(
        verdict,
        LiveSignalVerdict::InsufficientBars {
            symbol: "BTCUSDT".to_string(),
            required: 22,
            provided: 0,
        }
    );
}

#[test]
fn test_turtle_abstain_on_flat_market() {
    let bars = build_bars(30, dec!(60000.0), dec!(0.0));
    let buffer = BarBuffer::from_bars(50, bars).expect("Buffer creates");
    let turtle_cfg = TurtleConfig::default();
    let fee_budget = FeeBudget::new(dec!(0.0008), dec!(0.0010), dec!(0.0003)).expect("Valid fees");
    let filter = default_filter_for_symbol("BTCUSDT");
    let risk_policy = RiskPolicy::new(dec!(0.50), dec!(0.06)).expect("Valid risk");

    let verdict = evaluate_turtle_bar(
        "BTCUSDT",
        &buffer,
        dec!(100.0),
        dec!(100.0),
        dec!(0.010),
        &turtle_cfg,
        &fee_budget,
        &filter,
        &risk_policy,
    )
    .expect("Evaluation succeeds");

    assert_eq!(
        verdict,
        LiveSignalVerdict::AbstainSignal {
            symbol: "BTCUSDT".to_string(),
            reason: "PriceBelowEntryChannel".to_string(),
        }
    );
}

#[test]
fn test_turtle_approved_breakout_signal() {
    let mut bars = build_bars(22, dec!(60000.0), dec!(0.0));
    let breakout_t = 22 * 86_400_000;
    // Bar 23 breaks out above 60050 high to 61500
    bars.push(
        Bar::new(
            breakout_t,
            breakout_t + 86_400_000,
            dec!(60100.0),
            dec!(61500.0),
            dec!(60000.0),
            dec!(61200.0),
            dec!(5000.0),
            dec!(3500.0),
            dec!(300_000_000.0),
            2000,
        )
        .expect("Valid bar"),
    );

    let buffer = BarBuffer::from_bars(50, bars).expect("Buffer creates");
    let turtle_cfg = TurtleConfig::default();
    let fee_budget = FeeBudget::new(dec!(0.0008), dec!(0.0010), dec!(0.0003)).expect("Valid fees");
    let filter = default_filter_for_symbol("BTCUSDT");
    let risk_policy = RiskPolicy::new(dec!(0.50), dec!(0.06)).expect("Valid risk");

    let verdict = evaluate_turtle_bar(
        "BTCUSDT",
        &buffer,
        dec!(1000.0),
        dec!(1000.0),
        dec!(0.010),
        &turtle_cfg,
        &fee_budget,
        &filter,
        &risk_policy,
    )
    .expect("Evaluation succeeds");

    match verdict {
        LiveSignalVerdict::Approved {
            symbol,
            size,
            initial_stop,
            atr,
        } => {
            assert_eq!(symbol, "BTCUSDT");
            assert!(size.qty > Decimal::ZERO);
            assert!(initial_stop < size.rounded_entry);
            assert!(atr > Decimal::ZERO);
        }
        _ => panic!("Expected LiveSignalVerdict::Approved, got {:?}", verdict),
    }
}
