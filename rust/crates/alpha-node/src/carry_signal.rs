use crate::error::NodeError;
use crate::signal::default_filter_for_symbol;
use alpha_core::carry::{funding_rate_to_apr_custom, FundingCarryConfig};
use alpha_core::carry_sizing::{calculate_carry_sizing, CarrySizing, CarrySizingVerdict};
use alpha_core::sizing::ExchangeFilter;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveCarryVerdict {
    Approved {
        symbol: String,
        sizing: CarrySizing,
        apr: Decimal,
    },
    Abstain {
        symbol: String,
        reason: String,
    },
    DecayExit {
        symbol: String,
        reason: String,
    },
    InversionExit {
        symbol: String,
        reason: String,
    },
    PredictivePreExit {
        symbol: String,
        reason: String,
    },
}

#[must_use]
pub fn default_fapi_filter_for_symbol(symbol: &str) -> ExchangeFilter {
    match symbol.to_uppercase().as_str() {
        "BTCUSDT" => ExchangeFilter::new(dec!(0.10), dec!(0.001), dec!(0.001), dec!(5.0))
            .unwrap_or_else(|_| default_filter_for_symbol(symbol)),
        "ETHUSDT" => ExchangeFilter::new(dec!(0.01), dec!(0.001), dec!(0.001), dec!(5.0))
            .unwrap_or_else(|_| default_filter_for_symbol(symbol)),
        _ => default_filter_for_symbol(symbol),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn evaluate_live_carry_step(
    symbol: &str,
    cfg: &FundingCarryConfig,
    smoothed_rate: Decimal,
    current_rate: Decimal,
    predicted_rate: Option<Decimal>,
    time_to_settle_ms: Option<i64>,
    capital: Decimal,
    spot_price: Decimal,
    perp_price: Decimal,
    in_position: bool,
    accumulated_funding: Decimal,
    decay_consecutive: usize,
) -> Result<LiveCarryVerdict, NodeError> {
    let apr = funding_rate_to_apr_custom(smoothed_rate, cfg.cycles_per_year);

    if in_position {
        if current_rate < cfg.inversion_stop_rate {
            return Ok(LiveCarryVerdict::InversionExit {
                symbol: symbol.to_string(),
                reason: format!(
                    "Funding inverted: rate {} < {}",
                    current_rate, cfg.inversion_stop_rate
                ),
            });
        }

        if let (Some(pred), Some(time_ms)) = (predicted_rate, time_to_settle_ms) {
            if (0..=10 * 60 * 1000).contains(&time_ms) && pred < cfg.predictive_pre_exit_rate {
                return Ok(LiveCarryVerdict::PredictivePreExit {
                    symbol: symbol.to_string(),
                    reason: format!(
                        "Predictive negative funding: pred {} < {}",
                        pred, cfg.predictive_pre_exit_rate
                    ),
                });
            }
        }

        if apr <= cfg.exit_apr && decay_consecutive >= cfg.min_decay_consecutive {
            let fee_pct = cfg.perp_fraction * cfg.round_trip_friction;
            let gross_pct = cfg.perp_fraction * accumulated_funding;
            if gross_pct >= fee_pct * cfg.fee_hurdle_mult {
                return Ok(LiveCarryVerdict::DecayExit {
                    symbol: symbol.to_string(),
                    reason: format!(
                        "Funding decayed below {}% APR with fees covered",
                        cfg.exit_apr
                    ),
                });
            }
        }

        return Ok(LiveCarryVerdict::Abstain {
            symbol: symbol.to_string(),
            reason: "Holding active carry position".to_string(),
        });
    }

    if apr < cfg.entry_apr {
        return Ok(LiveCarryVerdict::Abstain {
            symbol: symbol.to_string(),
            reason: format!("APR {} < threshold {}", apr, cfg.entry_apr),
        });
    }

    let basis = (perp_price - spot_price) / spot_price;
    if basis < cfg.min_basis {
        return Ok(LiveCarryVerdict::Abstain {
            symbol: symbol.to_string(),
            reason: format!(
                "Spot-Perp basis {} < min_basis threshold {}",
                basis, cfg.min_basis
            ),
        });
    }

    let spot_filter = default_filter_for_symbol(symbol);
    let perp_filter = default_fapi_filter_for_symbol(symbol);
    match calculate_carry_sizing(capital, spot_price, perp_price, &spot_filter, &perp_filter)? {
        CarrySizingVerdict::Approved(sizing) => Ok(LiveCarryVerdict::Approved {
            symbol: symbol.to_string(),
            sizing,
            apr,
        }),
        CarrySizingVerdict::AbstainMinNotional {
            spot_notional,
            min_notional,
            ..
        } => Ok(LiveCarryVerdict::Abstain {
            symbol: symbol.to_string(),
            reason: format!("Notional {} < minNotional {}", spot_notional, min_notional),
        }),
        CarrySizingVerdict::AbstainMinQty {
            spot_qty, min_qty, ..
        } => Ok(LiveCarryVerdict::Abstain {
            symbol: symbol.to_string(),
            reason: format!("Qty {} < minQty {}", spot_qty, min_qty),
        }),
    }
}
