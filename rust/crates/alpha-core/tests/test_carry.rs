use alpha_core::carry::{
    calc_funding_apr_series, calc_funding_apr_series_custom, check_adl_desync, funding_rate_to_apr,
    funding_rate_to_apr_custom, simulate_carry_series, update_funding_ema, CarryExitReason,
    FundingCarryConfig,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[test]
fn test_funding_apr_calculation() {
    let r1 = dec!(0.0001); // 0.01% per 8h
    let apr1 = funding_rate_to_apr(r1);
    assert_eq!(apr1, dec!(0.1095)); // 10.95% APR

    let ema_next = update_funding_ema(r1, dec!(0.0002), dec!(0.20));
    assert_eq!(ema_next, dec!(0.00012));

    let series = calc_funding_apr_series(&[r1, dec!(0.0002)], dec!(0.20));
    assert_eq!(series.len(), 2);
    assert_eq!(series[0], dec!(0.1095));
    assert_eq!(series[1], funding_rate_to_apr(dec!(0.00012)));
}

#[test]
fn test_custom_cycles_per_year_for_4h_and_1h() {
    let r = dec!(0.0001);
    // 4h funding: 6 cycles/day * 365 = 2190
    let apr_4h = funding_rate_to_apr_custom(r, dec!(2190));
    assert_eq!(apr_4h, dec!(0.2190));

    // 1h funding: 24 cycles/day * 365 = 8760
    let apr_1h = funding_rate_to_apr_custom(r, dec!(8760));
    assert_eq!(apr_1h, dec!(0.8760));

    let series_4h = calc_funding_apr_series_custom(&[r], dec!(0.20), dec!(2190));
    assert_eq!(series_4h[0], dec!(0.2190));
}

#[test]
fn test_carry_fee_hurdle_lock_prevents_premature_decay_exit() {
    let cfg = FundingCarryConfig::default();
    let mut rates = Vec::new();
    let mut times = Vec::new();

    // 1. High initial rate to trigger entry
    rates.push(dec!(0.0005));
    times.push(1000);

    // 2. Rate decays to 0.00001 (APR ~1% < 2.2% exit threshold) for 4 epochs
    // Fee hurdle lock holds
    for i in 1..=4 {
        rates.push(dec!(0.00001));
        times.push(1000 + i * 28800 * 1000);
    }

    let r_unit = dec!(0.010);
    let trades = simulate_carry_series(&cfg, &rates, &times, r_unit).unwrap();

    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].exit_reason, CarryExitReason::EndOfSample);

    // 3. Clear fee hurdle:
    for i in 5..=25 {
        rates.push(dec!(0.0002));
        times.push(1000 + i * 28800 * 1000);
    }
    for i in 26..=40 {
        rates.push(dec!(0.000001));
        times.push(1000 + i * 28800 * 1000);
    }

    let trades_with_hurdle_cleared = simulate_carry_series(&cfg, &rates, &times, r_unit).unwrap();
    assert_eq!(trades_with_hurdle_cleared.len(), 1);
    assert_eq!(
        trades_with_hurdle_cleared[0].exit_reason,
        CarryExitReason::FundingDecayExit
    );
    assert!(trades_with_hurdle_cleared[0].net_return_pct > Decimal::ZERO);
}

#[test]
fn test_funding_inversion_stop_overrides_fee_hurdle_lock() {
    let cfg = FundingCarryConfig::default();
    let mut rates = Vec::new();
    let mut times = Vec::new();

    rates.push(dec!(0.0005));
    times.push(1000);

    // Severe negative funding rate
    rates.push(dec!(-0.00020));
    times.push(2000);

    let r_unit = dec!(0.010);
    let trades = simulate_carry_series(&cfg, &rates, &times, r_unit).unwrap();

    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].exit_reason, CarryExitReason::FundingInversionStop);
    assert_eq!(trades[0].exit_idx, 1);
}

#[test]
fn test_adl_desync_watchdog_rule() {
    let tolerance = dec!(0.99);

    // Perfectly matched
    assert!(!check_adl_desync(dec!(1.0), dec!(-1.0), tolerance));

    // Slight mismatch within tolerance
    assert!(!check_adl_desync(dec!(1.0), dec!(-0.995), tolerance));

    // Desynced: perp reduced to 0.985 < 0.99
    assert!(check_adl_desync(dec!(1.0), dec!(-0.985), tolerance));

    // Severe desync (ADL closed perp leg completely)
    assert!(check_adl_desync(dec!(1.0), dec!(0.0), tolerance));

    // Flat portfolio: zero spot quantity must never trigger ADL desync
    assert!(!check_adl_desync(dec!(0.0), dec!(0.0), tolerance));
    assert!(!check_adl_desync(dec!(0.0), dec!(-1.0), tolerance));

    // Negative spot quantity is non-positive, must return false
    assert!(!check_adl_desync(dec!(-1.0), dec!(0.0), tolerance));
}

#[test]
fn test_simulate_carry_input_validation() {
    let cfg = FundingCarryConfig::default();
    let rates = vec![dec!(0.0001), dec!(0.0002)];
    let times = vec![1000, 2000];

    // Division by zero risk prevented: r_unit <= 0 returns error
    let err_r = simulate_carry_series(&cfg, &rates, &times, dec!(0.0));
    assert!(err_r.is_err());

    let err_r_neg = simulate_carry_series(&cfg, &rates, &times, dec!(-0.01));
    assert!(err_r_neg.is_err());

    // Length mismatch prevented: returns error
    let mismatched_times = vec![1000];
    let err_len = simulate_carry_series(&cfg, &rates, &mismatched_times, dec!(0.01));
    assert!(err_len.is_err());

    // Empty series returns empty Ok
    let empty_res = simulate_carry_series(&cfg, &[], &[], dec!(0.01)).unwrap();
    assert!(empty_res.is_empty());
}
