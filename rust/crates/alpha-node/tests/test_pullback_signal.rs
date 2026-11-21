use alpha_core::pullback::PullbackConfig;
use alpha_core::risk::RiskPolicy;
use alpha_core::sizing::FeeBudget;
use alpha_node::buffer::BarBuffer;
use alpha_node::pullback_signal::evaluate_pullback_bar;
use alpha_node::signal::{default_filter_for_symbol, LiveSignalVerdict};
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
fn test_pullback_insufficient_bars_verdict() {
    let buffer = BarBuffer::new(250).expect("Buffer creates");
    let cfg = PullbackConfig::default();
    let fee_budget = FeeBudget::new(dec!(0.0008), dec!(0.0010), dec!(0.0003)).expect("Valid fees");
    let filter = default_filter_for_symbol("BTCUSDT");
    let risk_policy = RiskPolicy::new(dec!(0.50), dec!(0.06)).expect("Valid risk");

    let verdict = evaluate_pullback_bar(
        "BTCUSDT",
        &buffer,
        dec!(100.0),
        dec!(100.0),
        dec!(0.010),
        &cfg,
        &fee_budget,
        &filter,
        &risk_policy,
    )
    .expect("Evaluation succeeds");

    assert_eq!(
        verdict,
        LiveSignalVerdict::InsufficientBars {
            symbol: "BTCUSDT".to_string(),
            required: 205,
            provided: 0,
        }
    );
}

#[test]
fn test_pullback_abstain_on_flat_market() {
    let bars = build_bars(210, dec!(60000.0), dec!(0.0));
    let buffer = BarBuffer::from_bars(250, bars).expect("Buffer creates");
    let cfg = PullbackConfig::default();
    let fee_budget = FeeBudget::new(dec!(0.0008), dec!(0.0010), dec!(0.0003)).expect("Valid fees");
    let filter = default_filter_for_symbol("BTCUSDT");
    let risk_policy = RiskPolicy::new(dec!(0.50), dec!(0.06)).expect("Valid risk");

    let verdict = evaluate_pullback_bar(
        "BTCUSDT",
        &buffer,
        dec!(100.0),
        dec!(100.0),
        dec!(0.010),
        &cfg,
        &fee_budget,
        &filter,
        &risk_policy,
    )
    .expect("Evaluation succeeds");

    assert_eq!(
        verdict,
        LiveSignalVerdict::AbstainSignal {
            symbol: "BTCUSDT".to_string(),
            reason: "PullbackFilterNotTriggered".to_string(),
        }
    );
}
