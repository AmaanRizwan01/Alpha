use alpha_core::exits::ExitConfig;
use alpha_core::h1::H1Config;
use alpha_core::risk::RiskPolicy;
use alpha_core::sizing::FeeBudget;
use alpha_node::buffer::BarBuffer;
use alpha_node::signal::{default_filter_for_symbol, evaluate_symbol_bar, LiveSignalVerdict};
use alpha_types::Bar;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

fn build_bars(n: usize, start_px: Decimal, step: Decimal) -> Vec<Bar> {
    (0..n)
        .map(|i| {
            let px = start_px + (step * Decimal::from(i));
            let t = i64::try_from(i).unwrap() * 3_600_000;
            Bar::new(
                t,
                t + 3_600_000,
                px,
                px + dec!(50.0),
                px - dec!(20.0),
                px,
                dec!(1000.0),
                dec!(600.0),
                dec!(50_000_000.0),
                500,
            )
            .unwrap()
        })
        .collect()
}

#[test]
fn test_insufficient_bars_verdict() {
    let buffer = BarBuffer::new(50).unwrap();
    let h1_cfg = H1Config::default();
    let exit_cfg = ExitConfig::default();
    let fee_budget = FeeBudget::new(dec!(0.0008), dec!(0.0010), dec!(0.0003)).unwrap();
    let filter = default_filter_for_symbol("BTCUSDT");
    let risk_policy = RiskPolicy::new(dec!(0.50), dec!(0.06)).unwrap();

    let verdict = evaluate_symbol_bar(
        "BTCUSDT",
        &buffer,
        dec!(100.0),
        dec!(100.0),
        dec!(0.010),
        &h1_cfg,
        &exit_cfg,
        &fee_budget,
        &filter,
        &risk_policy,
    )
    .unwrap();

    assert_eq!(
        verdict,
        LiveSignalVerdict::InsufficientBars {
            symbol: "BTCUSDT".to_string(),
            required: 25,
            provided: 0,
        }
    );
}

#[test]
fn test_abstain_on_flat_market() {
    let bars = build_bars(30, dec!(60000.0), dec!(0.0));
    let buffer = BarBuffer::from_bars(50, bars).unwrap();
    let h1_cfg = H1Config::default();
    let exit_cfg = ExitConfig::default();
    let fee_budget = FeeBudget::new(dec!(0.0008), dec!(0.0010), dec!(0.0003)).unwrap();
    let filter = default_filter_for_symbol("BTCUSDT");
    let risk_policy = RiskPolicy::new(dec!(0.50), dec!(0.06)).unwrap();

    let verdict = evaluate_symbol_bar(
        "BTCUSDT",
        &buffer,
        dec!(100.0),
        dec!(100.0),
        dec!(0.010),
        &h1_cfg,
        &exit_cfg,
        &fee_budget,
        &filter,
        &risk_policy,
    )
    .unwrap();

    assert!(matches!(verdict, LiveSignalVerdict::AbstainSignal { .. }));
    if let LiveSignalVerdict::AbstainSignal { symbol, reason } = verdict {
        assert_eq!(symbol, "BTCUSDT");
        assert!(reason.contains("Chop filter") || reason.contains("No breakout"));
    }
}

#[test]
fn test_approved_breakout_signal() {
    let mut bars = build_bars(24, dec!(60000.0), dec!(10.0));
    let breakout_t = 24 * 3_600_000;
    let breakout_bar = Bar::new(
        breakout_t,
        breakout_t + 3_600_000,
        dec!(60240.0),
        dec!(61500.0),
        dec!(60200.0),
        dec!(61400.0),
        dec!(3000.0),
        dec!(2200.0),
        dec!(180_000_000.0),
        1500,
    )
    .unwrap();
    bars.push(breakout_bar);

    let buffer = BarBuffer::from_bars(50, bars).unwrap();
    let h1_cfg = H1Config::default();
    let exit_cfg = ExitConfig::default();
    let fee_budget = FeeBudget::new(dec!(0.0008), dec!(0.0010), dec!(0.0003)).unwrap();
    let filter = default_filter_for_symbol("BTCUSDT");
    let risk_policy = RiskPolicy::new(dec!(0.50), dec!(0.06)).unwrap();

    let verdict = evaluate_symbol_bar(
        "BTCUSDT",
        &buffer,
        dec!(100.0),
        dec!(100.0),
        dec!(0.010),
        &h1_cfg,
        &exit_cfg,
        &fee_budget,
        &filter,
        &risk_policy,
    )
    .unwrap();

    assert!(matches!(verdict, LiveSignalVerdict::Approved { .. }));
    if let LiveSignalVerdict::Approved {
        symbol,
        size,
        initial_stop,
        atr,
    } = verdict
    {
        assert_eq!(symbol, "BTCUSDT");
        assert!(size.notional >= dec!(5.0));
        assert!(initial_stop < dec!(61400.0));
        assert!(atr > Decimal::ZERO);
    }
}
