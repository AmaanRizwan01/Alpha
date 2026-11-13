use crate::config::AppConfig;
use crate::db::DbManager;
use crate::error::NodeError;
use crate::telegram::TelegramNotifier;
use rusqlite::params;
use rust_decimal::Decimal;

#[derive(Debug)]
pub struct OrderRow {
    pub client_order_id: String,
    pub symbol: String,
    pub side: String,
    pub order_type: String,
    pub price: Option<String>,
    pub qty: String,
    pub status: String,
    pub created_at_ms: i64,
}

pub fn fetch_recent_orders(db_path: &str, limit: usize) -> Result<Vec<OrderRow>, NodeError> {
    let conn = rusqlite::Connection::open(db_path)?;
    let mut stmt = conn.prepare_cached(
        "SELECT client_order_id, symbol, side, order_type, price, qty, status, created_at_ms
         FROM orders ORDER BY created_at_ms DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![limit], |row| {
        Ok(OrderRow {
            client_order_id: row.get(0)?,
            symbol: row.get(1)?,
            side: row.get(2)?,
            order_type: row.get(3)?,
            price: row.get(4)?,
            qty: row.get(5)?,
            status: row.get(6)?,
            created_at_ms: row.get(7)?,
        })
    })?;

    let mut result = Vec::new();
    for r in rows {
        result.push(r?);
    }
    Ok(result)
}

struct TelemetrySnapshot<'a> {
    db_path: &'a str,
    config_info: Option<&'a str>,
    is_halted: bool,
    halt_reason: Option<&'a str>,
    hwm: Decimal,
    eq: Decimal,
    dd_pct: Decimal,
    tg_configured: bool,
}

fn print_metrics(snap: &TelemetrySnapshot) {
    println!("============================================================");
    println!("                PROJECT ALPHA NODE TELEMETRY                ");
    println!("============================================================");
    println!("Database:     {}", snap.db_path);
    if let Some(info) = snap.config_info {
        println!("Config:       {}", info);
    }
    println!(
        "Engine State: {}",
        if snap.is_halted {
            format!("HALTED ({})", snap.halt_reason.unwrap_or_default())
        } else {
            "RUNNING / HEALTHY".to_string()
        }
    );
    println!(
        "Telegram:     {}",
        if snap.tg_configured {
            "CONNECTED"
        } else {
            "NOT CONFIGURED"
        }
    );
    println!("------------------------------------------------------------");
    println!("Current Equity:         ${}", snap.eq);
    println!("Equity High-Water Mark: ${}", snap.hwm);
    println!(
        "Current Max Drawdown:   {}% (Circuit Breaker: 6.0%)",
        snap.dd_pct
    );
    let headroom = (Decimal::from(6) - snap.dd_pct).max(Decimal::ZERO);
    println!("Drawdown Headroom:      {}%", headroom);
    println!("------------------------------------------------------------");
}

fn print_sub_accounts(db: &DbManager) -> Result<(), NodeError> {
    if let Ok(subs) = crate::sub_account::list_sub_accounts(db.conn()) {
        if !subs.is_empty() {
            println!("Sub-Accounts (OKX Isolation):");
            for s in subs {
                let st = if s.is_halted { "HALTED" } else { "RUNNING" };
                println!(
                    "  • {:<14} [{:<19}] eq=${:<7} HWM=${:<7} dd={}% | state={}",
                    s.name, s.strategy, s.current_equity, s.high_water_mark, s.max_drawdown_pct, st
                );
            }
            println!("------------------------------------------------------------");
        }
    }
    Ok(())
}

fn print_positions_and_orders(db: &DbManager, db_path: &str) -> Result<(), NodeError> {
    println!("Active Positions:");
    let mut any_pos = false;
    for sym in &["BTCUSDT", "ETHUSDT", "BTC-USDT", "ETH-USDT"] {
        if let Some(pos) = db.get_position(sym)? {
            any_pos = true;
            println!(
                "  [{}] qty={} entry=${} stop=${} peak=${} (held {} bars, be={})",
                pos.symbol,
                pos.qty,
                pos.entry_price,
                pos.current_stop_price,
                pos.highest_price,
                pos.bars_held,
                pos.breakeven_active
            );
        }
    }
    if !any_pos {
        println!("  [None - 100% flat in cash]");
    }
    println!("------------------------------------------------------------");

    println!("Recent Orders (Last 5):");
    let orders = fetch_recent_orders(db_path, 5).unwrap_or_default();
    if orders.is_empty() {
        println!("  [No order records found]");
    } else {
        for o in orders {
            let px = o.price.as_deref().unwrap_or("MKT");
            println!(
                "  {} | {:<4} {:<7} | qty={:<8} px={:<8} | status={:<7} | id={}",
                o.created_at_ms, o.side, o.symbol, o.qty, px, o.status, o.client_order_id
            );
        }
    }
    println!("============================================================");
    Ok(())
}

fn resolve_db_path(target_path: &str, raw_db_path: &str) -> String {
    let p = std::path::Path::new(raw_db_path);
    if p.exists() {
        return raw_db_path.to_string();
    }
    if let Some(parent) = std::path::Path::new(target_path).parent() {
        let c1 = parent.join(p);
        if c1.exists() {
            return c1.to_string_lossy().to_string();
        }
        if let Some(grandparent) = parent.parent() {
            let c2 = grandparent.join(p);
            if c2.exists() {
                return c2.to_string_lossy().to_string();
            }
        }
    }
    raw_db_path.to_string()
}

pub fn display_status(target_path: &str) -> Result<(), NodeError> {
    let (db_path, config_info) = if target_path.ends_with(".toml") {
        let cfg = AppConfig::load_from_file(target_path)?;
        let info = format!(
            "mode={}, symbols={:?}",
            cfg.system.mode, cfg.universe.symbols
        );
        let resolved = resolve_db_path(target_path, &cfg.system.database_path);
        (resolved, Some(info))
    } else {
        (target_path.to_string(), None)
    };

    let db = DbManager::open(&db_path)?;
    let (is_halted, halt_reason) = db.get_halt_state()?;
    let (hwm, eq, dd_pct) = db.get_equity_hwm()?;
    let tg = TelegramNotifier::from_env();

    let snap = TelemetrySnapshot {
        db_path: &db_path,
        config_info: config_info.as_deref(),
        is_halted,
        halt_reason: halt_reason.as_deref(),
        hwm,
        eq,
        dd_pct,
        tg_configured: tg.is_configured(),
    };
    print_metrics(&snap);
    print_sub_accounts(&db)?;
    print_positions_and_orders(&db, &db_path)
}
