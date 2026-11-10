use crate::db::DbManager;
use crate::status::fetch_recent_orders;
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct ListenerConfig {
    pub db_path: String,
    pub mode: String,
    pub symbols: Vec<String>,
    pub token: String,
    pub authorized_chat_id: String,
}

fn handle_status(db_path: &str, mode: &str, symbols: &[String]) -> String {
    let db = match DbManager::open(db_path) {
        Ok(d) => d,
        Err(e) => return format!("⚠️ Error opening database: {e}"),
    };
    let (is_halted, halt_reason) = db.get_halt_state().unwrap_or((false, None));
    let (hwm, eq, dd_pct) = db.get_equity_hwm().unwrap_or((
        rust_decimal_macros::dec!(100),
        rust_decimal_macros::dec!(100),
        rust_decimal_macros::dec!(0),
    ));
    let state_str = if is_halted {
        format!(
            "🛑 HALTED ({})",
            halt_reason.unwrap_or_else(|| "unknown".into())
        )
    } else {
        "🟢 RUNNING / HEALTHY".to_string()
    };
    let headroom = (rust_decimal_macros::dec!(6.0) - dd_pct).max(rust_decimal_macros::dec!(0));
    format!(
        "<b>📊 Project Alpha Node Status</b>\n\n\
         <b>State:</b> {state_str}\n\
         <b>Mode:</b> {mode}\n\
         <b>Symbols:</b> {}\n\
         <b>Current Equity:</b> ${eq}\n\
         <b>High-Water Mark:</b> ${hwm}\n\
         <b>Current Drawdown:</b> {dd_pct}% (Max: 6.0%)\n\
         <b>Drawdown Headroom:</b> {headroom}%",
        symbols.join(", ")
    )
}

fn handle_positions(db_path: &str) -> String {
    let db = match DbManager::open(db_path) {
        Ok(d) => d,
        Err(e) => return format!("⚠️ Error opening database: {e}"),
    };
    let mut out = String::from("<b>📈 Active Positions</b>\n\n");
    let mut found = false;
    for sym in &["BTCUSDT", "ETHUSDT"] {
        if let Ok(Some(pos)) = db.get_position(sym) {
            found = true;
            out.push_str(&format!(
                "<b>{}</b>: qty={}\n\
                 • Entry: ${}\n\
                 • Stop: ${}\n\
                 • Peak: ${}\n\
                 • Bars: {} (BE: {})\n\n",
                pos.symbol,
                pos.qty,
                pos.entry_price,
                pos.current_stop_price,
                pos.highest_price,
                pos.bars_held,
                pos.breakeven_active
            ));
        }
    }
    if !found {
        out.push_str("<i>None - 100% flat in cash</i>");
    }
    out
}

fn handle_pnl(db_path: &str) -> String {
    let db = match DbManager::open(db_path) {
        Ok(d) => d,
        Err(e) => return format!("⚠️ Error opening database: {e}"),
    };
    let (hwm, eq, dd_pct) = db.get_equity_hwm().unwrap_or((
        rust_decimal_macros::dec!(100),
        rust_decimal_macros::dec!(100),
        rust_decimal_macros::dec!(0),
    ));
    let headroom = (rust_decimal_macros::dec!(6.0) - dd_pct).max(rust_decimal_macros::dec!(0));
    format!(
        "<b>💰 PnL & Balance Overview</b>\n\n\
         <b>Total Equity:</b> ${eq}\n\
         <b>Equity HWM:</b> ${hwm}\n\
         <b>Max Drawdown:</b> {dd_pct}%\n\
         <b>Circuit Breaker Limit:</b> 6.0%\n\
         <b>Drawdown Headroom:</b> {headroom}%"
    )
}

fn handle_orders(db_path: &str) -> String {
    let orders = match fetch_recent_orders(db_path, 5) {
        Ok(o) => o,
        Err(e) => return format!("⚠️ Error reading orders: {e}"),
    };
    if orders.is_empty() {
        return "<b>📋 Recent Orders</b>\n\n<i>No orders recorded yet.</i>".to_string();
    }
    let mut out = String::from("<b>📋 Recent Orders (Last 5)</b>\n\n");
    for o in orders {
        let px = o.price.as_deref().unwrap_or("MKT");
        out.push_str(&format!(
            "• <b>{} {}</b> qty={} px=${} | <code>{}</code>\n",
            o.side, o.symbol, o.qty, px, o.status
        ));
    }
    out
}

fn handle_flatten(db_path: &str) -> String {
    let db = match DbManager::open(db_path) {
        Ok(d) => d,
        Err(e) => return format!("⚠️ Error opening database: {e}"),
    };
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let now_ms = i64::try_from(now).unwrap_or(0);
    match db.set_halt_state(true, Some("operator telegram flatten"), now_ms) {
        Ok(_) => "🚨 <b>FLATTEN ENGAGED</b>\n\n\
                  Trading engine halted via Telegram.\n\
                  No new orders will be submitted."
            .to_string(),
        Err(e) => format!("⚠️ Failed to engage halt: {e}"),
    }
}

fn handle_help() -> String {
    "🤖 <b>Project Alpha Telegram Controls</b>\n\n\
     • <b>📊 Status</b> (/status) - Overall node health & equity\n\
     • <b>📈 Positions</b> (/positions) - Open positions & stops\n\
     • <b>💰 PnL</b> (/pnl) - Equity, HWM & drawdown\n\
     • <b>📋 Orders</b> (/orders) - Last 5 orders & statuses\n\
     • <b>🚨 Flatten</b> (/flatten) - Emergency halt kill switch\n\
     • <b>ℹ️ Help</b> (/help) - Show this guide\n\n\
     <i>Tap any button below to execute instantly.</i>"
        .to_string()
}

pub fn dispatch_command(text: &str, db_path: &str, mode: &str, symbols: &[String]) -> String {
    let clean = text.trim().to_lowercase();
    match clean.as_str() {
        "/status" | "📊 status" | "status" => handle_status(db_path, mode, symbols),
        "/positions" | "📈 positions" | "positions" => handle_positions(db_path),
        "/pnl" | "💰 pnl" | "pnl" => handle_pnl(db_path),
        "/orders" | "📋 orders" | "orders" => handle_orders(db_path),
        "/flatten" | "🚨 flatten" | "flatten" => handle_flatten(db_path),
        "/help" | "/start" | "ℹ️ help" | "help" => handle_help(),
        _ => "Unrecognized command. Tap a button below or type /help.".to_string(),
    }
}

async fn handle_single_update(client: &Client, cfg: &ListenerConfig, upd: &Value) {
    let msg = match upd.get("message") {
        Some(m) => m,
        None => return,
    };
    let chat_id = msg
        .get("chat")
        .and_then(|c| c.get("id"))
        .map(|id| id.to_string());
    if chat_id.as_deref() != Some(&cfg.authorized_chat_id) {
        return;
    }
    let text = match msg.get("text").and_then(Value::as_str) {
        Some(t) => t,
        None => return,
    };
    let reply = dispatch_command(text, &cfg.db_path, &cfg.mode, &cfg.symbols);
    let send_url = format!("https://api.telegram.org/bot{}/sendMessage", cfg.token);
    let payload = json!({
        "chat_id": cfg.authorized_chat_id,
        "text": reply,
        "parse_mode": "HTML",
        "reply_markup": {
            "keyboard": [
                [{"text": "📊 Status"}, {"text": "📈 Positions"}],
                [{"text": "💰 PnL"}, {"text": "📋 Orders"}],
                [{"text": "🚨 Flatten"}, {"text": "ℹ️ Help"}]
            ],
            "resize_keyboard": true,
            "is_persistent": true
        }
    });
    let _ = client.post(&send_url).json(&payload).send().await;
}

pub async fn run_telegram_listener(cfg: ListenerConfig) {
    let client = Client::builder()
        .timeout(Duration::from_secs(25))
        .build()
        .unwrap_or_else(|_| Client::new());
    let mut offset: i64 = 0;
    println!(
        "[telegram-bot] Listener started for chat_id={}",
        cfg.authorized_chat_id
    );

    loop {
        let url = format!(
            "https://api.telegram.org/bot{}/getUpdates?offset={offset}&timeout=15",
            cfg.token
        );
        match client.get(&url).send().await {
            Ok(resp) => {
                if let Ok(body) = resp.json::<Value>().await {
                    if let Some(updates) = body.get("result").and_then(Value::as_array) {
                        for upd in updates {
                            if let Some(uid) = upd.get("update_id").and_then(Value::as_i64) {
                                offset = uid + 1;
                            }
                            handle_single_update(&client, &cfg, upd).await;
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("[telegram-bot warning] polling error: {e}");
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
        }
    }
}
