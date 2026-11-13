use alpha_core::carry::{
    calc_funding_apr_series, check_adl_desync, funding_rate_to_apr, simulate_carry_series,
    update_funding_ema, CarryExitReason, FundingCarryConfig,
};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[test]
fn test_funding_apr_calculation() {
    let r1 = dec!(0.0001); // 0.01% per 8h
    let apr1 = funding_rate_to_apr(r1);
    assert_eq!(apr1, dec!(0.1095)); // 10.95% APR

    let ema_next = update_funding_ema(r1, dec!(0.0002), dec!(0.20));
    // 0.2 * 0.0002 + 0.8 * 0.0001 = 0.00004 + 0.00008 = 0.00012
    assert_eq!(ema_next, dec!(0.00012));

    let series = calc_funding_apr_series(&[r1, dec!(0.0002)], dec!(0.20));
    assert_eq!(series.len(), 2);
    assert_eq!(series[0], dec!(0.1095));
    assert_eq!(series[1], funding_rate_to_apr(dec!(0.00012)));
}

#[test]
fn test_carry_fee_hurdle_lock_prevents_premature_decay_exit() {
    let cfg = FundingCarryConfig::default();
    // Entry threshold is 12% APR -> ~0.0001096 rate per 8h
    // Fee hurdle: round_trip_friction = 0.0032. perp_fraction = 2/3.
    // fee_pct = (2/3) * 0.0032 = 0.002133333333333333333333333333

    let mut rates = Vec::new();
    let mut times = Vec::new();

    // 1. High initial rate to trigger entry
    rates.push(dec!(0.0005)); // APR = 54.75% -> Entry at idx 0!
    times.push(1000);

    // 2. Rate decays to 0.00001 (APR = ~1% < 2.2% exit threshold) for 3 epochs
    // Gross accumulated so far: (2/3) * (0.0005 + 0.00001*3) = (2/3) * 0.00053 = 0.00035333
    // Fee hurdle is 0.00213333.
    // Gross (0.00035) < Fee hurdle (0.00213) -> Fee-Hurdle Lock MUST HOLD!
    for i in 1..=4 {
        rates.push(dec!(0.00001));
        times.push(1000 + i * 28800 * 1000);
    }

    let r_unit = dec!(0.010);
    let trades = simulate_carry_series(&cfg, &rates, &times, r_unit).unwrap();

    // Should NOT have exited with FundingDecayExit; should only exit at EndOfSample!
    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].exit_reason, CarryExitReason::EndOfSample);

    // 3. Now add enough positive funding epochs to exceed fee hurdle:
    // Need sum of rates >= 0.0032. Let's add 20 epochs of 0.0002 (sum = 0.0040)
    for i in 5..=25 {
        rates.push(dec!(0.0002));
        times.push(1000 + i * 28800 * 1000);
    }
    // Then 15 decay epochs of near-zero rate so EMA decays well below 2.2% APR:
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

    // Entry at high rate
    rates.push(dec!(0.0005));
    times.push(1000);

    // Severe negative funding rate (inversion stop is -0.00015)
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

    // Slight mismatch within tolerance: 0.995 >= 0.99
    assert!(!check_adl_desync(dec!(1.0), dec!(-0.995), tolerance));

    // Desynced: perp reduced to 0.985 < 0.99
    assert!(check_adl_desync(dec!(1.0), dec!(-0.985), tolerance));

    // Severe desync (ADL closed perp leg completely)
    assert!(check_adl_desync(dec!(1.0), dec!(0.0), tolerance));
}
