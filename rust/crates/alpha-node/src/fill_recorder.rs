use crate::db::DbManager;
use crate::error::NodeError;
use crate::simulator::{PaperFill, PaperSimulator};
use crate::telegram::TelegramNotifier;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

pub fn record_paper_fill(
    db: &DbManager,
    simulator: &mut PaperSimulator,
    fill: &PaperFill,
    telegram: &TelegramNotifier,
    now_ms: i64,
) -> Result<(), NodeError> {
    match fill {
        PaperFill::Entry {
            order_id,
            symbol,
            price,
            qty,
            fee,
            ..
        } => {
            db.update_order_status(order_id, "FILLED", now_ms)?;
            let initial_stop = *price * dec!(0.975);
            db.upsert_position(
                symbol,
                "Binance",
                *qty,
                *price,
                initial_stop,
                *price,
                false,
                0,
                now_ms,
            )?;
            println!("[paper-fill] ENTRY {} qty={} price={}", symbol, qty, price);
            telegram.notify_fill(symbol, "BUY", *qty, *price, *fee, simulator.cash_balance);
        }
        PaperFill::StopLoss {
            symbol,
            fill_price,
            qty,
            pnl,
            ..
        } => {
            db.delete_position(symbol)?;
            let (_hwm, dd) = db.update_equity_hwm(simulator.cash_balance, now_ms)?;
            println!(
                "[paper-fill] STOP_LOSS {} qty={} px={} pnl={} | equity={} (dd={:.2}%)",
                symbol, qty, fill_price, pnl, simulator.cash_balance, dd
            );
            telegram.notify_position_closed(symbol, *fill_price, *pnl, dec!(-1.0), "STOP_LOSS");
            if dd >= Decimal::from(6) {
                println!(
                    "[circuit-breaker] Drawdown reached {:.2}% >= 6.0%. HALTED.",
                    dd
                );
                telegram.notify_circuit_breaker("Drawdown reached >= 6.0%", dd);
            }
        }
        PaperFill::TakeProfit {
            symbol,
            fill_price,
            qty,
            pnl,
            ..
        }
        | PaperFill::BarExit {
            symbol,
            fill_price,
            qty,
            pnl,
            ..
        } => {
            db.delete_position(symbol)?;
            db.update_equity_hwm(simulator.cash_balance, now_ms)?;
            println!(
                "[paper-fill] EXIT {} qty={} px={} pnl={} | equity={}",
                symbol, qty, fill_price, pnl, simulator.cash_balance
            );
            let pnl_r = if *pnl > Decimal::ZERO {
                dec!(1.5)
            } else {
                dec!(0.0)
            };
            telegram.notify_position_closed(
                symbol,
                *fill_price,
                *pnl,
                pnl_r,
                "TAKE_PROFIT/BAR_EXIT",
            );
        }
    }
    Ok(())
}
