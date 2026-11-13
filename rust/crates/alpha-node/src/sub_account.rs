use crate::db::PositionRecord;
use crate::error::NodeError;
use crate::status::OrderRow;
use alpha_types::OrderIntent;
use rusqlite::{params, Connection, Row};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubAccountRecord {
    pub name: String,
    pub strategy: String,
    pub high_water_mark: Decimal,
    pub current_equity: Decimal,
    pub max_drawdown_pct: Decimal,
    pub is_halted: bool,
    pub halt_reason: Option<String>,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortfolioSummary {
    pub total_equity: Decimal,
    pub total_hwm: Decimal,
    pub sub_accounts: Vec<SubAccountRecord>,
}

fn map_sub(r: &Row<'_>) -> rusqlite::Result<SubAccountRecord> {
    let (h, e, d): (String, String, String) = (r.get(2)?, r.get(3)?, r.get(4)?);
    let halted: i32 = r.get(5)?;
    Ok(SubAccountRecord {
        name: r.get(0)?,
        strategy: r.get(1)?,
        high_water_mark: Decimal::from_str(&h).unwrap_or(dec!(100.0)),
        current_equity: Decimal::from_str(&e).unwrap_or(dec!(100.0)),
        max_drawdown_pct: Decimal::from_str(&d).unwrap_or(Decimal::ZERO),
        is_halted: halted == 1,
        halt_reason: r.get(6)?,
        updated_at_ms: r.get(7)?,
    })
}

pub fn list_sub_accounts(conn: &Connection) -> Result<Vec<SubAccountRecord>, NodeError> {
    let mut stmt = conn.prepare_cached(
        "SELECT name, strategy, high_water_mark, current_equity, max_drawdown_pct, \
         is_halted, halt_reason, updated_at_ms FROM sub_accounts ORDER BY name ASC",
    )?;
    let rows = stmt.query_map([], map_sub)?;
    let out = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(out)
}

pub fn get_sub_account(
    conn: &Connection,
    name: &str,
) -> Result<Option<SubAccountRecord>, NodeError> {
    let mut stmt = conn.prepare_cached(
        "SELECT name, strategy, high_water_mark, current_equity, max_drawdown_pct, \
         is_halted, halt_reason, updated_at_ms FROM sub_accounts WHERE name = ?1",
    )?;
    let mut rows = stmt.query_map(params![name], map_sub)?;
    let out = rows.next().transpose()?;
    Ok(out)
}

#[rustfmt::skip]
pub fn update_sub_account_equity(
    conn: &Connection,
    name: &str,
    eq: Decimal,
    now_ms: i64,
) -> Result<(Decimal, Decimal), NodeError> {
    let rec = get_sub_account(conn, name)?.ok_or_else(|| NodeError::StateError {
        reason: format!("sub-account '{name}' not found"),
    })?;
    let hwm = rec.high_water_mark.max(eq);
    let dd = if hwm > Decimal::ZERO { ((hwm - eq) / hwm) * dec!(100.0) } else { Decimal::ZERO };
    let trip = i32::from(dd >= dec!(6.0));
    let sql = "UPDATE sub_accounts SET high_water_mark = ?1, current_equity = ?2, max_drawdown_pct = ?3, \
               is_halted = CASE WHEN ?4 = 1 THEN 1 ELSE is_halted END, \
               halt_reason = CASE WHEN ?4 = 1 THEN 'drawdown >= 6.0% circuit breaker' ELSE halt_reason END, \
               updated_at_ms = ?5 WHERE name = ?6";
    conn.execute(sql, params![hwm.to_string(), eq.to_string(), dd.to_string(), trip, now_ms, name])?;
    Ok((hwm, dd))
}

pub fn set_sub_account_halt(
    conn: &Connection,
    name: &str,
    halted: bool,
    reason: Option<&str>,
    now_ms: i64,
) -> Result<(), NodeError> {
    let sql = "UPDATE sub_accounts SET is_halted = ?1, halt_reason = ?2, updated_at_ms = ?3 WHERE name = ?4";
    conn.execute(sql, params![i32::from(halted), reason, now_ms, name])?;
    Ok(())
}

fn map_pos(r: &Row<'_>, off: usize) -> rusqlite::Result<PositionRecord> {
    let (q, e, s, h): (String, String, String, String) = (
        r.get(off + 2)?,
        r.get(off + 3)?,
        r.get(off + 4)?,
        r.get(off + 5)?,
    );
    let be: i32 = r.get(off + 6)?;
    Ok(PositionRecord {
        symbol: r.get(off)?,
        venue: r.get(off + 1)?,
        qty: Decimal::from_str(&q).unwrap_or(Decimal::ZERO),
        entry_price: Decimal::from_str(&e).unwrap_or(Decimal::ZERO),
        current_stop_price: Decimal::from_str(&s).unwrap_or(Decimal::ZERO),
        highest_price: Decimal::from_str(&h).unwrap_or(Decimal::ZERO),
        breakeven_active: be == 1,
        bars_held: r.get(off + 7)?,
    })
}

#[rustfmt::skip]
pub fn upsert_sub_account_position(
    conn: &Connection,
    sub: &str,
    p: &PositionRecord,
    now: i64,
) -> Result<(), NodeError> {
    let sql = "INSERT INTO sub_account_positions (sub_account, symbol, venue, qty, entry_price, \
               current_stop_price, highest_price, breakeven_active, bars_held, updated_at_ms) \
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10) \
               ON CONFLICT(sub_account, symbol) DO UPDATE SET qty = excluded.qty, \
               current_stop_price = excluded.current_stop_price, highest_price = excluded.highest_price, \
               breakeven_active = excluded.breakeven_active, bars_held = excluded.bars_held, \
               updated_at_ms = excluded.updated_at_ms";
    let (q, e) = (p.qty.to_string(), p.entry_price.to_string());
    let (s, h) = (p.current_stop_price.to_string(), p.highest_price.to_string());
    conn.execute(sql, params![sub, p.symbol, p.venue, q, e, s, h, i32::from(p.breakeven_active), p.bars_held, now])?;
    Ok(())
}

pub fn get_sub_account_positions(
    conn: &Connection,
    sub_account: &str,
) -> Result<Vec<PositionRecord>, NodeError> {
    let mut stmt = conn.prepare_cached(
        "SELECT symbol, venue, qty, entry_price, current_stop_price, highest_price, \
         breakeven_active, bars_held FROM sub_account_positions WHERE sub_account = ?1",
    )?;
    let rows = stmt.query_map(params![sub_account], |r| map_pos(r, 0))?;
    let out = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(out)
}

pub fn get_all_sub_account_positions(
    conn: &Connection,
) -> Result<Vec<(String, PositionRecord)>, NodeError> {
    let mut stmt = conn.prepare_cached(
        "SELECT sub_account, symbol, venue, qty, entry_price, current_stop_price, \
         highest_price, breakeven_active, bars_held FROM sub_account_positions \
         ORDER BY sub_account ASC, symbol ASC",
    )?;
    let rows = stmt.query_map([], |r| Ok((r.get(0)?, map_pos(r, 1)?)))?;
    let out = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(out)
}

pub fn delete_sub_account_position(
    conn: &Connection,
    sub: &str,
    sym: &str,
) -> Result<(), NodeError> {
    conn.execute(
        "DELETE FROM sub_account_positions WHERE sub_account = ?1 AND symbol = ?2",
        params![sub, sym],
    )?;
    Ok(())
}

#[rustfmt::skip]
pub fn insert_sub_account_order(
    conn: &Connection,
    sub: &str,
    it: &OrderIntent,
    status: &str,
    now: i64,
) -> Result<(), NodeError> {
    let sql = "INSERT INTO sub_account_orders (client_order_id, sub_account, venue, symbol, side, \
               order_type, price, qty, stop_loss_price, take_profit_price, status, created_at_ms, updated_at_ms) \
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)";
    let p = it.price.map(|v| v.to_string());
    let sl = it.stop_loss_price.map(|v| v.to_string());
    let tp = it.take_profit_price.map(|v| v.to_string());
    let v = format!("{:?}", it.venue);
    let s = format!("{:?}", it.side);
    let ot = format!("{:?}", it.order_type);
    let q = it.qty.to_string();
    conn.execute(sql, params![it.client_order_id, sub, v, it.symbol, s, ot, p, q, sl, tp, status, now, now])?;
    Ok(())
}

pub fn update_sub_account_order_status(
    conn: &Connection,
    client_order_id: &str,
    status: &str,
    now: i64,
) -> Result<(), NodeError> {
    let sql =
        "UPDATE sub_account_orders SET status = ?1, updated_at_ms = ?2 WHERE client_order_id = ?3";
    conn.execute(sql, params![status, now, client_order_id])?;
    Ok(())
}

fn map_order(r: &Row<'_>) -> rusqlite::Result<OrderRow> {
    Ok(OrderRow {
        client_order_id: r.get(0)?,
        symbol: r.get(1)?,
        side: r.get(2)?,
        order_type: r.get(3)?,
        price: r.get(4)?,
        qty: r.get(5)?,
        status: r.get(6)?,
        created_at_ms: r.get(7)?,
    })
}

pub fn fetch_sub_account_orders(
    conn: &Connection,
    sub: &str,
    limit: usize,
) -> Result<Vec<OrderRow>, NodeError> {
    let mut stmt = conn.prepare_cached(
        "SELECT client_order_id, symbol, side, order_type, price, qty, status, created_at_ms \
         FROM sub_account_orders WHERE sub_account = ?1 ORDER BY created_at_ms DESC LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![sub, limit], map_order)?;
    let out = rows.collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(out)
}

pub fn get_portfolio_summary(conn: &Connection) -> Result<PortfolioSummary, NodeError> {
    let subs = list_sub_accounts(conn)?;
    let eq = subs.iter().map(|s| s.current_equity).sum();
    let hwm = subs.iter().map(|s| s.high_water_mark).sum();
    Ok(PortfolioSummary {
        total_equity: eq,
        total_hwm: hwm,
        sub_accounts: subs,
    })
}
