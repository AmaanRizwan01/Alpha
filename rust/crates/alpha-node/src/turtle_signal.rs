use crate::buffer::BarBuffer;
use crate::error::NodeError;
use crate::signal::LiveSignalVerdict;
use alpha_core::risk::{RiskPolicy, RiskVerdict};
use alpha_core::sizing::{calculate_position_size, ExchangeFilter, FeeBudget};
use alpha_core::turtle::{evaluate_turtle_entry, TurtleConfig, TurtleVerdict};
use rust_decimal::Decimal;

fn map_risk_verdict(
    symbol: &str,
    stop_price: Decimal,
    atr: Decimal,
    verdict: RiskVerdict,
) -> LiveSignalVerdict {
    match verdict {
        RiskVerdict::Approved { size } => LiveSignalVerdict::Approved {
            symbol: symbol.to_string(),
            size,
            initial_stop: stop_price,
            atr,
        },
        RiskVerdict::AbstainMinNotional {
            notional,
            min_notional,
        } => LiveSignalVerdict::AbstainRisk {
            symbol: symbol.to_string(),
            reason: format!("Notional {} < minNotional {}", notional, min_notional),
        },
        RiskVerdict::AbstainMinQuantity { qty, min_qty } => LiveSignalVerdict::AbstainRisk {
            symbol: symbol.to_string(),
            reason: format!("Qty {} < minQty {}", qty, min_qty),
        },
        RiskVerdict::AbstainExceedsMaxEquityFraction {
            fraction,
            max_fraction,
        } => LiveSignalVerdict::AbstainRisk {
            symbol: symbol.to_string(),
            reason: format!("Equity fraction {} > max {}", fraction, max_fraction),
        },
        RiskVerdict::AbstainDrawdownBreaker {
            current_drawdown,
            max_drawdown,
        } => LiveSignalVerdict::AbstainRisk {
            symbol: symbol.to_string(),
            reason: format!(
                "Drawdown {:.2}% >= breaker limit {:.2}%",
                current_drawdown, max_drawdown
            ),
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub fn evaluate_turtle_bar(
    symbol: &str,
    buffer: &BarBuffer,
    equity: Decimal,
    equity_hwm: Decimal,
    risk_fraction: Decimal,
    turtle_config: &TurtleConfig,
    fee_budget: &FeeBudget,
    exchange_filter: &ExchangeFilter,
    risk_policy: &RiskPolicy,
) -> Result<LiveSignalVerdict, NodeError> {
    let req_bars = turtle_config.entry_period.max(turtle_config.atr_period + 1) + 1;
    if !buffer.is_ready(req_bars) {
        return Ok(LiveSignalVerdict::InsufficientBars {
            symbol: symbol.to_string(),
            required: req_bars,
            provided: buffer.len(),
        });
    }

    let verdict = evaluate_turtle_entry(buffer.bars(), turtle_config)?;
    match verdict {
        TurtleVerdict::EnterLong {
            entry_price,
            stop_price,
            atr,
            ..
        } => {
            let pos_size = calculate_position_size(
                equity,
                risk_fraction,
                entry_price,
                stop_price,
                fee_budget,
                exchange_filter,
                turtle_config.min_stop_distance,
            )?;
            let risk_res =
                risk_policy.evaluate_entry(equity, equity_hwm, &pos_size, exchange_filter);
            Ok(map_risk_verdict(symbol, stop_price, atr, risk_res))
        }
        TurtleVerdict::Abstain { reason } => Ok(LiveSignalVerdict::AbstainSignal {
            symbol: symbol.to_string(),
            reason: reason.to_string(),
        }),
        TurtleVerdict::ExitLong { .. } | TurtleVerdict::Hold { .. } => {
            Ok(LiveSignalVerdict::AbstainSignal {
                symbol: symbol.to_string(),
                reason: "Exit or Hold state".to_string(),
            })
        }
    }
}
