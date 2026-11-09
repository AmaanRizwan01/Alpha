use crate::engine::TradeEngine;
use crate::error::NodeError;
use crate::signal::{default_filter_for_symbol, evaluate_symbol_bar, LiveSignalVerdict};
use alpha_core::sizing::PositionSize;
use alpha_types::{OrderIntent, OrderType, Side, Venue};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub fn evaluate_and_dispatch(
    engine: &mut TradeEngine,
    symbol: &str,
    now_ms: i64,
) -> Result<(), NodeError> {
    let (hwm, _, _) = engine.db.get_equity_hwm()?;
    let filter = default_filter_for_symbol(symbol);
    let buf = engine
        .bar_buffers
        .get(symbol)
        .ok_or_else(|| NodeError::BufferError {
            reason: format!("missing buffer for {}", symbol),
        })?;

    let verdict = evaluate_symbol_bar(
        symbol,
        buf,
        engine.simulator.cash_balance,
        hwm,
        engine.config.risk.max_risk_per_trade,
        &engine.h1_config,
        &engine.exit_config,
        &engine.fee_budget,
        &filter,
        &engine.risk_policy,
    )?;

    if let LiveSignalVerdict::Approved {
        symbol,
        size,
        initial_stop,
        atr,
    } = verdict
    {
        let stop_pct = if size.rounded_entry > Decimal::ZERO {
            ((size.rounded_entry - initial_stop) / size.rounded_entry) * dec!(100.0)
        } else {
            dec!(2.2)
        };
        println!(
            "[signal-h1] APPROVED for {}: qty={} px={} stop={} atr={}",
            symbol, size.qty, size.rounded_entry, initial_stop, atr
        );
        engine.telegram.notify_signal_approved(
            &symbol,
            size.rounded_entry,
            initial_stop,
            stop_pct,
            size.qty,
        );
        submit_signal_order(engine, &symbol, &size, initial_stop, now_ms)?;
    }
    Ok(())
}

fn submit_signal_order(
    engine: &mut TradeEngine,
    symbol: &str,
    size: &PositionSize,
    stop: Decimal,
    now_ms: i64,
) -> Result<(), NodeError> {
    let client_id = format!("paper-h1-{}-{}", symbol, now_ms);
    let intent = OrderIntent::new(
        Venue::Binance,
        symbol,
        Side::Buy,
        OrderType::LimitMaker,
        Some(size.rounded_entry),
        size.qty,
        Some(stop),
        None,
        &client_id,
    )?;
    engine.db.insert_order(&intent, "WORKING", now_ms)?;
    engine.telegram.notify_order_submitted(
        symbol,
        "BUY",
        size.qty,
        Some(size.rounded_entry),
        "LimitMaker",
    );
    engine.simulator.submit_order(intent)
}
