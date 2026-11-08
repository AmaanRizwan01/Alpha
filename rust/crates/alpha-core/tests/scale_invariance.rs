use alpha_core::{calculate_position_size, ExchangeFilter, FeeBudget};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[test]
fn test_scale_invariance_across_capital_tiers() {
    let filter = ExchangeFilter::new(dec!(0.10), dec!(0.00001), dec!(0.00001), dec!(5.0)).unwrap();
    let fees = FeeBudget::new(dec!(0.0008), dec!(0.0010), dec!(0.0005)).unwrap();
    let min_stop_pct = dec!(0.022);
    let risk_pct = dec!(0.010); // 1.00% risk per trade

    // Test across a grid of BTC prices ($20,000 to $120,000)
    let btc_prices = [
        dec!(25000.0),
        dec!(45000.0),
        dec!(65000.0),
        dec!(85000.0),
        dec!(100000.0),
    ];
    // Test across stop distances (2.5% to 5.0%)
    let stop_fractions = [dec!(0.025), dec!(0.030), dec!(0.035), dec!(0.045)];

    let tiers = [
        dec!(100.0),   // Stage 1 Incubation
        dec!(5000.0),  // Stage 2A Scale-up
        dec!(10000.0), // Stage 2B Full Target
        dec!(50000.0), // High Scale
    ];

    for &price in &btc_prices {
        for &stop_frac in &stop_fractions {
            let stop_price = price * (Decimal::ONE - stop_frac);

            let mut notional_fractions = Vec::new();
            let mut risk_fractions = Vec::new();

            for &equity in &tiers {
                let size = calculate_position_size(
                    equity,
                    risk_pct,
                    price,
                    stop_price,
                    &fees,
                    &filter,
                    min_stop_pct,
                )
                .unwrap();

                // 1. Hard Law: Realized dollar risk must never exceed max budget
                let max_risk_budget = equity * risk_pct;
                assert!(
                    size.risk_quote <= max_risk_budget,
                    "Risk exceeded at equity {}: {} > {}",
                    equity,
                    size.risk_quote,
                    max_risk_budget
                );

                // Record notional fraction of equity
                let notional_frac = size.notional / equity;
                let actual_risk_frac = size.risk_quote / equity;
                notional_fractions.push(notional_frac);
                risk_fractions.push(actual_risk_frac);
            }

            // 2. Scale-Invariance Law:
            // Compare $100 against $10,000 baseline
            let frac_100 = notional_fractions[0];
            let frac_10000 = notional_fractions[2];
            let diff = (frac_100 - frac_10000).abs();

            // Lot quantization error on $100 is at most ~1.5% of notional
            assert!(
                diff < dec!(0.018),
                "Divergence too large at price {}: $100 frac={}, $10k frac={}, diff={}",
                price,
                frac_100,
                frac_10000,
                diff
            );
        }
    }
}
