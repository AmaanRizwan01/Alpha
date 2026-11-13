use crate::error::NodeError;
use crate::signal::default_filter_for_symbol;
use alpha_core::carry::{funding_rate_to_apr, FundingCarryConfig};
use alpha_core::carry_sizing::{calculate_carry_sizing, CarrySizing, CarrySizingVerdict};
use rust_decimal::Decimal;

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
    let apr = funding_rate_to_apr(smoothed_rate);

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
            if time_ms <= 10 * 60 * 1000 && pred < cfg.predictive_pre_exit_rate {
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

    let filter = default_filter_for_symbol(symbol);
    match calculate_carry_sizing(capital, spot_price, perp_price, &filter, &filter)? {
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

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
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
            dec!(65000.0),
            false,
            dec!(0.0),
            0,
        )
        .unwrap();

        match verdict {
            LiveCarryVerdict::Approved { sizing, apr, .. } => {
                assert!(apr > cfg.entry_apr);
                assert!(sizing.spot_notional >= dec!(5.0));
            }
            _ => panic!("Expected approved carry entry"),
        }
    }

    #[test]
    fn test_live_carry_predictive_pre_exit() {
        let cfg = FundingCarryConfig::default();
        let verdict = evaluate_live_carry_step(
            "BTCUSDT",
            &cfg,
            dec!(0.0001),
            dec!(0.0001),
            Some(dec!(-0.00010)), // Predicted negative funding
            Some(5 * 60 * 1000),  // 5 mins to settle <= 10 mins
            dec!(100.0),
            dec!(65000.0),
            dec!(65000.0),
            true, // In position
            dec!(0.001),
            0,
        )
        .unwrap();

        assert!(matches!(
            verdict,
            LiveCarryVerdict::PredictivePreExit { .. }
        ));
    }
}
