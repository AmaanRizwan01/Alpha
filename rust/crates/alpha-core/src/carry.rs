use crate::error::CoreError;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

pub use crate::carry_sim::*;

pub const DEFAULT_CYCLES_PER_YEAR: Decimal = dec!(1095); // 3 funding cycles per day * 365 days

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FundingCarryConfig {
    pub entry_apr: Decimal,
    pub exit_apr: Decimal,
    pub ema_alpha: Decimal,
    pub inversion_stop_rate: Decimal,
    pub round_trip_friction: Decimal,
    pub fee_hurdle_mult: Decimal,
    pub min_decay_consecutive: usize,
    pub perp_fraction: Decimal,
    pub predictive_pre_exit_rate: Decimal,
    pub adl_tolerance_ratio: Decimal,
    pub cycles_per_year: Decimal,
    pub min_basis: Decimal,
}

impl Default for FundingCarryConfig {
    fn default() -> Self {
        Self {
            entry_apr: dec!(0.120),
            exit_apr: dec!(0.022),
            ema_alpha: dec!(0.20),
            inversion_stop_rate: dec!(-0.00015),
            round_trip_friction: dec!(0.0032),
            fee_hurdle_mult: dec!(1.0),
            min_decay_consecutive: 3,
            perp_fraction: dec!(2) / dec!(3),
            predictive_pre_exit_rate: dec!(-0.00005),
            adl_tolerance_ratio: dec!(0.99),
            cycles_per_year: DEFAULT_CYCLES_PER_YEAR,
            min_basis: dec!(-0.0005),
        }
    }
}

impl FundingCarryConfig {
    pub fn new(
        entry_apr: Decimal,
        exit_apr: Decimal,
        ema_alpha: Decimal,
        inversion_stop_rate: Decimal,
        round_trip_friction: Decimal,
        fee_hurdle_mult: Decimal,
    ) -> Result<Self, CoreError> {
        if entry_apr <= exit_apr {
            return Err(CoreError::InvalidParameter {
                name: "entry_apr",
                reason: "entry APR must exceed exit APR",
            });
        }
        if exit_apr <= Decimal::ZERO {
            return Err(CoreError::InvalidParameter {
                name: "exit_apr",
                reason: "exit APR must be strictly positive",
            });
        }
        if ema_alpha <= Decimal::ZERO || ema_alpha > Decimal::ONE {
            return Err(CoreError::InvalidParameter {
                name: "ema_alpha",
                reason: "ema_alpha must be in (0.0, 1.0]",
            });
        }
        if inversion_stop_rate > Decimal::ZERO {
            return Err(CoreError::InvalidParameter {
                name: "inversion_stop_rate",
                reason: "inversion_stop_rate must be non-positive",
            });
        }
        if round_trip_friction < Decimal::ZERO || fee_hurdle_mult < Decimal::ZERO {
            return Err(CoreError::InvalidParameter {
                name: "friction_or_hurdle",
                reason: "friction and fee hurdle must be non-negative",
            });
        }
        Ok(Self {
            entry_apr,
            exit_apr,
            ema_alpha,
            inversion_stop_rate,
            round_trip_friction,
            fee_hurdle_mult,
            ..Default::default()
        })
    }
}

#[must_use]
pub fn update_funding_ema(prev_ema: Decimal, current_rate: Decimal, alpha: Decimal) -> Decimal {
    alpha * current_rate + (Decimal::ONE - alpha) * prev_ema
}

#[must_use]
pub fn funding_rate_to_apr(smoothed_rate: Decimal) -> Decimal {
    funding_rate_to_apr_custom(smoothed_rate, DEFAULT_CYCLES_PER_YEAR)
}

#[must_use]
pub fn funding_rate_to_apr_custom(smoothed_rate: Decimal, cycles_per_year: Decimal) -> Decimal {
    smoothed_rate * cycles_per_year
}

#[must_use]
pub fn check_adl_desync(spot_qty: Decimal, perp_qty: Decimal, tolerance_ratio: Decimal) -> bool {
    if spot_qty <= Decimal::ZERO {
        return false;
    }
    let effective_tol = if tolerance_ratio > Decimal::ZERO && tolerance_ratio <= Decimal::ONE {
        tolerance_ratio
    } else {
        dec!(0.99)
    };
    perp_qty.abs() < (spot_qty * effective_tol)
}

pub fn calc_funding_apr_series(rates: &[Decimal], alpha: Decimal) -> Vec<Decimal> {
    calc_funding_apr_series_custom(rates, alpha, DEFAULT_CYCLES_PER_YEAR)
}

pub fn calc_funding_apr_series_custom(
    rates: &[Decimal],
    alpha: Decimal,
    cycles_per_year: Decimal,
) -> Vec<Decimal> {
    if rates.is_empty() {
        return Vec::new();
    }
    let mut apr = Vec::with_capacity(rates.len());
    let mut ema = match rates.first() {
        Some(&r) => r,
        None => Decimal::ZERO,
    };
    apr.push(funding_rate_to_apr_custom(ema, cycles_per_year));

    for &rate in rates.iter().skip(1) {
        ema = update_funding_ema(ema, rate, alpha);
        apr.push(funding_rate_to_apr_custom(ema, cycles_per_year));
    }
    apr
}
