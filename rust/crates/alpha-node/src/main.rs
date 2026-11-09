#![forbid(unsafe_code)]
#![deny(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::todo,
    clippy::unimplemented,
    clippy::float_cmp,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

pub mod config;
pub mod db;
pub mod engine;
pub mod error;
pub mod reconcile;
pub mod simulator;
pub mod ws;

use config::AppConfig;
use db::DbManager;
use engine::TradeEngine;
use error::NodeError;
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

fn current_time_ms() -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    i64::try_from(now.as_millis()).unwrap_or(0)
}

fn handle_status(target_path: &str) -> Result<(), NodeError> {
    let db_path = if target_path.ends_with(".toml") {
        let cfg = AppConfig::load_from_file(target_path)?;
        cfg.system.database_path
    } else {
        target_path.to_string()
    };

    let db = DbManager::open(&db_path)?;
    let (is_halted, halt_reason) = db.get_halt_state()?;
    let (hwm, eq, dd_pct) = db.get_equity_hwm()?;

    println!("=== Alpha Node Status ===");
    println!("Database: {}", db_path);
    println!(
        "Engine State: {}",
        if is_halted {
            format!("HALTED ({})", halt_reason.unwrap_or_default())
        } else {
            "RUNNING / HEALTHY".to_string()
        }
    );
    println!("Current Equity: {}", eq);
    println!("Equity High-Water Mark: {}", hwm);
    println!("Current Max Drawdown: {}%", dd_pct);

    for sym in &["BTCUSDT", "ETHUSDT", "BTC-USDT", "ETH-USDT"] {
        if let Some(pos) = db.get_position(sym)? {
            println!(
                "Position [{}] on {}: qty={}, entry={}, stop={}",
                pos.symbol, pos.venue, pos.qty, pos.entry_price, pos.current_stop_price
            );
        }
    }
    Ok(())
}

fn handle_flatten(target_path: &str) -> Result<(), NodeError> {
    let db_path = if target_path.ends_with(".toml") {
        let cfg = AppConfig::load_from_file(target_path)?;
        cfg.system.database_path
    } else {
        target_path.to_string()
    };

    let db = DbManager::open(&db_path)?;
    let now_ms = current_time_ms();
    db.set_halt_state(true, Some("operator emergency flatten"), now_ms)?;
    println!("=== FLATTEN ENGAGED ===");
    println!("Trading engine halted. Submitting cancel-all and liquidation intents.");
    Ok(())
}

async fn handle_trade(target_path: &str) -> Result<(), NodeError> {
    let config_path = if target_path.ends_with(".toml") {
        target_path
    } else {
        "configs/paper.toml"
    };

    let cfg = AppConfig::load_from_file(config_path)?;
    println!(
        "Alpha trade daemon initializing on database: {} (mode={}, symbols={:?})",
        cfg.system.database_path, cfg.system.mode, cfg.universe.symbols
    );

    let mut engine = TradeEngine::new(cfg)?;
    engine.run(None).await
}

fn print_usage() {
    println!("Project Alpha Node v0.1.0");
    println!("Usage: alpha-node <command> [options]");
    println!();
    println!("Commands:");
    println!("  status [config_or_db]       Show engine state, HWM, and active positions");
    println!("  flatten [config_or_db]      Emergency halt and trigger position liquidation");
    println!("  trade [config.toml]         Start 24/7 trading daemon with live WebSocket feed");
    println!("  record [out_dir]            Start 24/7 top-of-book and trade recorder");
}

#[tokio::main]
async fn main() -> Result<(), NodeError> {
    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(String::as_str);
    let target_path = args
        .get(2)
        .map(String::as_str)
        .unwrap_or("configs/paper.toml");

    match command {
        Some("status") => handle_status(target_path)?,
        Some("flatten") => handle_flatten(target_path)?,
        Some("trade") => handle_trade(target_path).await?,
        Some("record") => {
            println!("Alpha recorder initialized targeting: {}", target_path);
        }
        _ => print_usage(),
    }

    Ok(())
}
