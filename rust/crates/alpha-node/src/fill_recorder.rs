use crate::db::{DbManager, PositionRecord};
use crate::error::NodeError;
use crate::simulator::{PaperFill, PaperSimulator};
use crate::telegram::TelegramNotifier;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

struct FillCtx<'a> {
    db: &'a DbManager,
    sim: &'a mut PaperSimulator,
    tg: &'a TelegramNotifier,
    now: i64,
}

fn handle_entry(
    ctx: &mut FillCtx<'_>,
    oid: &str,
    sym: &str,
    px: Decimal,
    qty: Decimal,
    fee: Decimal,
) -> Result<(), NodeError> {
    ctx.db.update_order_status(oid, "FILLED", ctx.now)?;
    let _ =
        crate::sub_account::update_sub_account_order_status(ctx.db.conn(), oid, "FILLED", ctx.now);
    let stop = px * dec!(0.975);
    let pos = PositionRecord {
        symbol: sym.to_string(),
        venue: "Binance".to_string(),
        qty,
        entry_price: px,
        current_stop_price: stop,
        highest_price: px,
        breakeven_active: false,
        bars_held: 0,
    };
    ctx.db
        .upsert_position(sym, "Binance", qty, px, stop, px, false, 0, ctx.now)?;
    let _ = crate::sub_account::upsert_sub_account_position(
        ctx.db.conn(),
        "alphabreakout",
        &pos,
        ctx.now,
    );
    println!("[paper-fill] ENTRY {sym} qty={qty} price={px}");
    ctx.tg
        .notify_fill(sym, "BUY", qty, px, fee, ctx.sim.cash_balance);
    Ok(())
}

fn handle_sl(
    ctx: &mut FillCtx<'_>,
    sym: &str,
    px: Decimal,
    qty: Decimal,
    pnl: Decimal,
) -> Result<(), NodeError> {
    ctx.db.delete_position(sym)?;
    let _ = crate::sub_account::delete_sub_account_position(ctx.db.conn(), "alphabreakout", sym);
    let (_hwm, dd) = ctx.db.update_equity_hwm(ctx.sim.cash_balance, ctx.now)?;
    let _ = crate::sub_account::update_sub_account_equity(
        ctx.db.conn(),
        "alphabreakout",
        ctx.sim.cash_balance,
        ctx.now,
    );
    println!(
        "[paper-fill] STOP_LOSS {sym} qty={qty} px={px} pnl={pnl} | eq={} (dd={dd:.2}%)",
        ctx.sim.cash_balance
    );
    ctx.tg
        .notify_position_closed(sym, px, pnl, dec!(-1.0), "STOP_LOSS");
    if dd >= Decimal::from(6) {
        println!("[circuit-breaker] Drawdown reached {dd:.2}% >= 6.0%. HALTED.");
        ctx.tg
            .notify_circuit_breaker("Drawdown reached >= 6.0%", dd);
    }
    Ok(())
}

fn handle_exit(
    ctx: &mut FillCtx<'_>,
    sym: &str,
    px: Decimal,
    qty: Decimal,
    pnl: Decimal,
) -> Result<(), NodeError> {
    ctx.db.delete_position(sym)?;
    let _ = crate::sub_account::delete_sub_account_position(ctx.db.conn(), "alphabreakout", sym);
    ctx.db.update_equity_hwm(ctx.sim.cash_balance, ctx.now)?;
    let _ = crate::sub_account::update_sub_account_equity(
        ctx.db.conn(),
        "alphabreakout",
        ctx.sim.cash_balance,
        ctx.now,
    );
    println!(
        "[paper-fill] EXIT {sym} qty={qty} px={px} pnl={pnl} | eq={}",
        ctx.sim.cash_balance
    );
    let pnl_r = if pnl > Decimal::ZERO {
        dec!(1.5)
    } else {
        dec!(0.0)
    };
    ctx.tg
        .notify_position_closed(sym, px, pnl, pnl_r, "TAKE_PROFIT/BAR_EXIT");
    Ok(())
}

pub fn record_paper_fill(
    db: &DbManager,
    simulator: &mut PaperSimulator,
    fill: &PaperFill,
    telegram: &TelegramNotifier,
    now_ms: i64,
) -> Result<(), NodeError> {
    let mut ctx = FillCtx {
        db,
        sim: simulator,
        tg: telegram,
        now: now_ms,
    };
    match fill {
        PaperFill::Entry {
            order_id,
            symbol,
            price,
            qty,
            fee,
            ..
        } => handle_entry(&mut ctx, order_id, symbol, *price, *qty, *fee),
        PaperFill::StopLoss {
            symbol,
            fill_price,
            qty,
            pnl,
            ..
        } => handle_sl(&mut ctx, symbol, *fill_price, *qty, *pnl),
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
        } => handle_exit(&mut ctx, symbol, *fill_price, *qty, *pnl),
    }
}
