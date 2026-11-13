use crate::error::CoreError;
use crate::sizing::ExchangeFilter;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CarrySizing {
    pub spot_qty: Decimal,
    pub perp_qty: Decimal,
    pub spot_notional: Decimal,
    pub perp_notional: Decimal,
    pub spot_capital: Decimal,
    pub margin_capital: Decimal,
    pub total_capital: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CarrySizingVerdict {
    Approved(CarrySizing),
    AbstainMinNotional {
        spot_notional: Decimal,
        perp_notional: Decimal,
        min_notional: Decimal,
    },
    AbstainMinQty {
        spot_qty: Decimal,
        perp_qty: Decimal,
        min_qty: Decimal,
    },
}

pub fn calculate_carry_sizing(
    capital: Decimal,
    spot_price: Decimal,
    perp_price: Decimal,
    spot_filter: &ExchangeFilter,
    perp_filter: &ExchangeFilter,
) -> Result<CarrySizingVerdict, CoreError> {
    if capital <= Decimal::ZERO || spot_price <= Decimal::ZERO || perp_price <= Decimal::ZERO {
        return Err(CoreError::InvalidParameter {
            name: "carry_sizing_inputs",
            reason: "capital and prices must be strictly positive",
        });
    }

    let spot_capital = (capital * dec!(2)) / dec!(3);
    let margin_capital = capital - spot_capital;
    let perp_notional_target = margin_capital * dec!(2);

    let raw_spot_qty = spot_capital / spot_price;
    let raw_perp_qty = perp_notional_target / perp_price;

    let raw_spot = spot_filter.round_qty_down(raw_spot_qty);
    let raw_perp = perp_filter.round_qty_down(raw_perp_qty);
    let candidate = raw_spot.min(raw_perp);
    let matched_qty = spot_filter.round_qty_down(perp_filter.round_qty_down(candidate));

    let spot_qty = matched_qty;
    let perp_qty = matched_qty;

    let spot_notional = spot_qty * spot_price;
    let perp_notional = perp_qty * perp_price;

    if spot_qty < spot_filter.min_qty || perp_qty < perp_filter.min_qty {
        return Ok(CarrySizingVerdict::AbstainMinQty {
            spot_qty,
            perp_qty,
            min_qty: spot_filter.min_qty.max(perp_filter.min_qty),
        });
    }

    if spot_notional < spot_filter.min_notional || perp_notional < perp_filter.min_notional {
        return Ok(CarrySizingVerdict::AbstainMinNotional {
            spot_notional,
            perp_notional,
            min_notional: spot_filter.min_notional.max(perp_filter.min_notional),
        });
    }

    Ok(CarrySizingVerdict::Approved(CarrySizing {
        spot_qty,
        perp_qty,
        spot_notional,
        perp_notional,
        spot_capital,
        margin_capital,
        total_capital: capital,
    }))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn test_carry_sizing_scale_invariance_100_vs_10000() {
        let spot_filter =
            ExchangeFilter::new(dec!(0.10), dec!(0.00001), dec!(0.00001), dec!(5.0)).unwrap();
        let perp_filter =
            ExchangeFilter::new(dec!(0.10), dec!(0.00001), dec!(0.00001), dec!(5.0)).unwrap();

        let price = dec!(65000.0);

        let v100 =
            calculate_carry_sizing(dec!(100.0), price, price, &spot_filter, &perp_filter).unwrap();
        let v10000 =
            calculate_carry_sizing(dec!(10000.0), price, price, &spot_filter, &perp_filter)
                .unwrap();

        let s100 = match v100 {
            CarrySizingVerdict::Approved(s) => s,
            _ => panic!("Expected approved for 100"),
        };
        let s10000 = match v10000 {
            CarrySizingVerdict::Approved(s) => s,
            _ => panic!("Expected approved for 10000"),
        };

        let ratio = s10000.spot_notional / s100.spot_notional;
        assert!(ratio >= dec!(99.0) && ratio <= dec!(101.0));
        assert!(s100.spot_notional >= dec!(5.0));
        assert_eq!(s100.spot_qty, s100.perp_qty);
        assert_eq!(s10000.spot_qty, s10000.perp_qty);
    }

    #[test]
    fn test_carry_sizing_delta_neutral_match_with_basis_and_different_filters() {
        // Spot step is 0.00001, Futures step is 0.001
        let spot_filter =
            ExchangeFilter::new(dec!(0.10), dec!(0.00001), dec!(0.00001), dec!(5.0)).unwrap();
        let perp_filter =
            ExchangeFilter::new(dec!(0.10), dec!(0.001), dec!(0.001), dec!(5.0)).unwrap();

        let spot_p = dec!(65000.0);
        let perp_p = dec!(65050.0);

        let v = calculate_carry_sizing(dec!(100.0), spot_p, perp_p, &spot_filter, &perp_filter)
            .unwrap();
        let s = match v {
            CarrySizingVerdict::Approved(s) => s,
            _ => panic!("Expected approved sizing"),
        };
        // Exactly equal coin quantities
        assert_eq!(s.spot_qty, s.perp_qty);
        assert_eq!(s.spot_qty, dec!(0.001));
    }

    #[test]
    fn test_carry_sizing_abstains_on_micro_capital() {
        let spot_filter =
            ExchangeFilter::new(dec!(0.10), dec!(0.00001), dec!(0.00001), dec!(5.0)).unwrap();
        let perp_filter =
            ExchangeFilter::new(dec!(0.10), dec!(0.00001), dec!(0.00001), dec!(5.0)).unwrap();

        let v = calculate_carry_sizing(
            dec!(2.0),
            dec!(65000.0),
            dec!(65000.0),
            &spot_filter,
            &perp_filter,
        )
        .unwrap();
        assert!(matches!(v, CarrySizingVerdict::AbstainMinNotional { .. }));
    }
}
