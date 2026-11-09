use crate::buffer::BarBuffer;
use crate::error::NodeError;
use alpha_core::exits::{calculate_initial_stop, ExitConfig};
use alpha_core::h1::{evaluate_h1_signal, H1Config, H1Verdict};
use alpha_core::indicators::average_true_range;
use alpha_core::risk::{RiskPolicy, RiskVerdict};
use alpha_core::sizing::{calculate_position_size, ExchangeFilter, FeeBudget, PositionSize};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LiveSignalVerdict {
    Approved {
        symbol: String,
        size: PositionSize,
        initial_stop: Decimal,
        atr: Decimal,
    },
    AbstainSignal {
        symbol: String,
        reason: String,
    },
    AbstainRisk {
        symbol: String,
        reason: String,
    },
    InsufficientBars {
        symbol: String,
        required: usize,
        provided: usize,
    },
}

#[must_use]
pub fn default_filter_for_symbol(symbol: &str) -> ExchangeFilter {
    match symbol.to_uppercase().as_str() {
        "BTCUSDT" | "BTCFDUSD" | "BTCUSDC" => {
            ExchangeFilter::new(dec!(0.10), dec!(0.00001), dec!(0.00001), dec!(5.0))
                .unwrap_or_else(|_| fallback_filter())
        }
        "ETHUSDT" | "ETHFDUSD" | "ETHUSDC" => {
            ExchangeFilter::new(dec!(0.01), dec!(0.0001), dec!(0.0001), dec!(5.0))
                .unwrap_or_else(|_| fallback_filter())
        }
        _ => fallback_filter(),
    }
}

fn fallback_filter() -> ExchangeFilter {
    ExchangeFilter {
        tick_size: dec!(0.01),
        step_size: dec!(0.0001),
        min_qty: dec!(0.0001),
        min_notional: dec!(5.0),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn evaluate_symbol_bar(
    symbol: &str,
    buffer: &BarBuffer,
    equity: Decimal,
    equity_hwm: Decimal,
    risk_fraction: Decimal,
    h1_config: &H1Config,
    exit_config: &ExitConfig,
    fee_budget: &FeeBudget,
    exchange_filter: &ExchangeFilter,
    risk_policy: &RiskPolicy,
) -> Result<LiveSignalVerdict, NodeError> {
    let req_bars = h1_config
        .er_period
        .max(h1_config.donchian_period)
        .max(h1_config.volume_sma_period)
        + 1;

    if !buffer.is_ready(req_bars) {
        return Ok(LiveSignalVerdict::InsufficientBars {
            symbol: symbol.to_string(),
            required: req_bars,
            provided: buffer.len(),
        });
    }

    let h1_result = evaluate_h1_signal(buffer.bars(), h1_config)?;
    if !h1_result.is_signal() {
        let reason = match h1_result {
            H1Verdict::AbstainChop { er, threshold } => {
                format!("Chop filter engaged (ER {:.3} < {:.3})", er, threshold)
            }
            H1Verdict::AbstainNoBreakout { close, upper_band } => {
                format!("No breakout (close {} <= upper {})", close, upper_band)
            }
            H1Verdict::AbstainLowVolume {
                volume,
                required_volume,
            } => {
                format!("Volume insufficient ({} <= {})", volume, required_volume)
            }
            H1Verdict::AbstainLowTakerRatio { ratio, threshold } => {
                format!("Taker buy ratio low ({} < {})", ratio, threshold)
            }
            H1Verdict::Signal { .. } => String::new(),
        };
        return Ok(LiveSignalVerdict::AbstainSignal {
            symbol: symbol.to_string(),
            reason,
        });
    }

    let atr = average_true_range(buffer.bars(), 14)?;
    let latest = buffer.latest().ok_or_else(|| NodeError::BufferError {
        reason: "empty buffer on signal evaluation".to_string(),
    })?;

    let initial_stop = calculate_initial_stop(latest.close, atr, exit_config)?;
    let pos_size = calculate_position_size(
        equity,
        risk_fraction,
        latest.close,
        initial_stop,
        fee_budget,
        exchange_filter,
        exit_config.min_stop_distance_pct,
    )?;

    match risk_policy.evaluate_entry(equity, equity_hwm, &pos_size, exchange_filter) {
        RiskVerdict::Approved { size } => Ok(LiveSignalVerdict::Approved {
            symbol: symbol.to_string(),
            size,
            initial_stop,
            atr,
        }),
        RiskVerdict::AbstainMinNotional {
            notional,
            min_notional,
        } => Ok(LiveSignalVerdict::AbstainRisk {
            symbol: symbol.to_string(),
            reason: format!("Notional {} < minNotional {}", notional, min_notional),
        }),
        RiskVerdict::AbstainMinQuantity { qty, min_qty } => Ok(LiveSignalVerdict::AbstainRisk {
            symbol: symbol.to_string(),
            reason: format!("Qty {} < minQty {}", qty, min_qty),
        }),
        RiskVerdict::AbstainExceedsMaxEquityFraction {
            fraction,
            max_fraction,
        } => Ok(LiveSignalVerdict::AbstainRisk {
            symbol: symbol.to_string(),
            reason: format!("Equity fraction {} > max {}", fraction, max_fraction),
        }),
        RiskVerdict::AbstainDrawdownBreaker {
            current_drawdown,
            max_drawdown,
        } => Ok(LiveSignalVerdict::AbstainRisk {
            symbol: symbol.to_string(),
            reason: format!(
                "Drawdown {:.2}% >= breaker limit {:.2}%",
                current_drawdown, max_drawdown
            ),
        }),
    }
}
