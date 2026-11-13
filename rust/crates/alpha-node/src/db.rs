use crate::error::NodeError;
use alpha_types::OrderIntent;
use rusqlite::{params, Connection};
use rust_decimal::prelude::*;
use rust_decimal::Decimal;
use std::path::Path;

const MIGRATION_SQL: &str = include_str!("../../../../migrations/0001_state.sql");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PositionRecord {
    pub symbol: String,
    pub venue: String,
    pub qty: Decimal,
    pub entry_price: Decimal,
    pub current_stop_price: Decimal,
    pub highest_price: Decimal,
    pub breakeven_active: bool,
    pub bars_held: u32,
}

pub struct DbManager {
    conn: Connection,
}

impl DbManager {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, NodeError> {
        let conn = Connection::open(path)?;
        let mgr = Self { conn };
        mgr.init_migrations()?;
        Ok(mgr)
    }

    pub fn open_in_memory() -> Result<Self, NodeError> {
        let conn = Connection::open_in_memory()?;
        let mgr = Self { conn };
        mgr.init_migrations()?;
        Ok(mgr)
    }

    pub fn init_migrations(&self) -> Result<(), NodeError> {
        self.conn.execute_batch(MIGRATION_SQL)?;
        Ok(())
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn insert_order(
        &self,
        intent: &OrderIntent,
        status: &str,
        now_ms: i64,
    ) -> Result<(), NodeError> {
        let sql =
            "INSERT INTO orders (client_order_id, venue, symbol, side, order_type, price, qty, \
                   stop_loss_price, take_profit_price, status, created_at_ms, updated_at_ms) \
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)";
        let p_str = intent.price.map(|p| p.to_string());
        let sl_str = intent.stop_loss_price.map(|p| p.to_string());
        let tp_str = intent.take_profit_price.map(|p| p.to_string());
        self.conn.execute(
            sql,
            params![
                intent.client_order_id,
                format!("{:?}", intent.venue),
                intent.symbol,
                format!("{:?}", intent.side),
                format!("{:?}", intent.order_type),
                p_str,
                intent.qty.to_string(),
                sl_str,
                tp_str,
                status,
                now_ms,
                now_ms
            ],
        )?;
        Ok(())
    }

    pub fn update_order_status(
        &self,
        client_order_id: &str,
        status: &str,
        now_ms: i64,
    ) -> Result<(), NodeError> {
        let mut stmt = self.conn.prepare_cached(
            "UPDATE orders SET status = ?1, updated_at_ms = ?2 WHERE client_order_id = ?3",
        )?;
        stmt.execute(params![status, now_ms, client_order_id])?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn upsert_position(
        &self,
        symbol: &str,
        venue: &str,
        qty: Decimal,
        entry_price: Decimal,
        stop_price: Decimal,
        highest_price: Decimal,
        breakeven: bool,
        bars_held: u32,
        now_ms: i64,
    ) -> Result<(), NodeError> {
        let sql = "INSERT INTO positions (symbol, venue, qty, entry_price, current_stop_price, \
                   highest_price, breakeven_active, bars_held, updated_at_ms) \
                   VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
                   ON CONFLICT(symbol) DO UPDATE SET qty = excluded.qty, \
                   current_stop_price = excluded.current_stop_price, highest_price = excluded.highest_price, \
                   breakeven_active = excluded.breakeven_active, bars_held = excluded.bars_held, \
                   updated_at_ms = excluded.updated_at_ms";
        self.conn.execute(
            sql,
            params![
                symbol,
                venue,
                qty.to_string(),
                entry_price.to_string(),
                stop_price.to_string(),
                highest_price.to_string(),
                if breakeven { 1 } else { 0 },
                bars_held,
                now_ms,
            ],
        )?;
        Ok(())
    }

    pub fn get_position(&self, symbol: &str) -> Result<Option<PositionRecord>, NodeError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT symbol, venue, qty, entry_price, current_stop_price, highest_price,
                    breakeven_active, bars_held
             FROM positions WHERE symbol = ?1",
        )?;

        let mut rows = stmt.query(params![symbol])?;
        if let Some(row) = rows.next()? {
            let qty_str: String = row.get(2)?;
            let entry_str: String = row.get(3)?;
            let stop_str: String = row.get(4)?;
            let high_str: String = row.get(5)?;
            let be_val: i32 = row.get(6)?;

            Ok(Some(PositionRecord {
                symbol: row.get(0)?,
                venue: row.get(1)?,
                qty: Decimal::from_str(&qty_str)?,
                entry_price: Decimal::from_str(&entry_str)?,
                current_stop_price: Decimal::from_str(&stop_str)?,
                highest_price: Decimal::from_str(&high_str)?,
                breakeven_active: be_val == 1,
                bars_held: row.get(7)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn delete_position(&self, symbol: &str) -> Result<(), NodeError> {
        let mut stmt = self
            .conn
            .prepare_cached("DELETE FROM positions WHERE symbol = ?1")?;
        stmt.execute(params![symbol])?;
        Ok(())
    }

    pub fn get_halt_state(&self) -> Result<(bool, Option<String>), NodeError> {
        let mut stmt = self
            .conn
            .prepare_cached("SELECT is_halted, halt_reason FROM halt_state WHERE id = 1")?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let halted: i32 = row.get(0)?;
            Ok((halted == 1, row.get(1)?))
        } else {
            Ok((false, None))
        }
    }

    pub fn set_halt_state(
        &self,
        is_halted: bool,
        reason: Option<&str>,
        now_ms: i64,
    ) -> Result<(), NodeError> {
        let mut stmt = self.conn.prepare_cached(
            "UPDATE halt_state SET is_halted = ?1, halt_reason = ?2, halted_at_ms = ?3 WHERE id = 1",
        )?;
        stmt.execute(params![if is_halted { 1 } else { 0 }, reason, now_ms])?;
        Ok(())
    }

    pub fn get_equity_hwm(&self) -> Result<(Decimal, Decimal, Decimal), NodeError> {
        let mut stmt = self.conn.prepare_cached(
            "SELECT high_water_mark, current_equity, max_drawdown_pct FROM equity_hwm WHERE id = 1",
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            let hwm: String = row.get(0)?;
            let eq: String = row.get(1)?;
            let dd: String = row.get(2)?;
            Ok((
                Decimal::from_str(&hwm)?,
                Decimal::from_str(&eq)?,
                Decimal::from_str(&dd)?,
            ))
        } else {
            Ok((Decimal::from(100), Decimal::from(100), Decimal::ZERO))
        }
    }

    pub fn update_equity_hwm(
        &self,
        current_equity: Decimal,
        now_ms: i64,
    ) -> Result<(Decimal, Decimal), NodeError> {
        let (hwm, _, _) = self.get_equity_hwm()?;
        let hwm = hwm.max(current_equity);
        let drawdown_pct = if hwm > Decimal::ZERO {
            ((hwm - current_equity) / hwm) * Decimal::from(100)
        } else {
            Decimal::ZERO
        };

        let mut update_stmt = self.conn.prepare_cached(
            "UPDATE equity_hwm SET high_water_mark = ?1, current_equity = ?2, max_drawdown_pct = ?3, updated_at_ms = ?4 WHERE id = 1",
        )?;
        update_stmt.execute(params![
            hwm.to_string(),
            current_equity.to_string(),
            drawdown_pct.to_string(),
            now_ms
        ])?;

        if drawdown_pct >= Decimal::from(6) {
            self.set_halt_state(
                true,
                Some("drawdown exceeded 6.0% circuit breaker limit"),
                now_ms,
            )?;
        }

        Ok((hwm, drawdown_pct))
    }
}
