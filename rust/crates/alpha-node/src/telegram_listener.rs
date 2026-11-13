use crate::db::DbManager;
use crate::status::fetch_recent_orders;
use reqwest::Client;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
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

fn get_eq_stats(db: &DbManager) -> (Decimal, Decimal, Decimal, Decimal) {
    let (hwm, eq, dd) = db
        .get_equity_hwm()
        .unwrap_or((dec!(100), dec!(100), dec!(0)));
    (hwm, eq, dd, (dec!(6.0) - dd).max(dec!(0)))
}

fn handle_status(db_path: &str, mode: &str, symbols: &[String]) -> String {
    let db = match DbManager::open(db_path) {
        Ok(d) => d,
        Err(e) => return format!("⚠️ Error opening database: {e}"),
    };
    let (is_halted, halt_reason) = db.get_halt_state().unwrap_or((false, None));
    let (hwm, eq, dd_pct, headroom) = get_eq_stats(&db);
    let state_str = if is_halted {
        format!(
            "🛑 HALTED ({})",
            halt_reason.unwrap_or_else(|| "unknown".into())
        )
    } else {
        "🟢 RUNNING / HEALTHY".to_string()
    };
    let mut out = format!(
        "<b>📊 Project Alpha Node Status</b>\n\n\
         <b>State:</b> {state_str}\n<b>Mode:</b> {mode}\n<b>Symbols:</b> {}\n\
         <b>Current Equity:</b> ${eq}\n<b>High-Water Mark:</b> ${hwm}\n\
         <b>Current Drawdown:</b> {dd_pct}% (Max: 6.0%)\n<b>Drawdown Headroom:</b> {headroom}%\n",
        symbols.join(", ")
    );
    if let Ok(subs) = crate::sub_account::list_sub_accounts(db.conn()) {
        if !subs.is_empty() {
            out.push_str("\n<b>Sub-Accounts (OKX):</b>\n");
            for s in subs {
                let st = if s.is_halted { "🛑" } else { "🟢" };
                let h_tag = if s.is_halted { " - HALTED" } else { "" };
                out.push_str(&format!(
                    "• {st} <b>{}</b>: ${} (DD: {}%{h_tag})\n",
                    s.name, s.current_equity, s.max_drawdown_pct
                ));
            }
        }
    }
    out
}

fn handle_positions(db_path: &str) -> String {
    let db = match DbManager::open(db_path) {
        Ok(d) => d,
        Err(e) => return format!("⚠️ Error opening database: {e}"),
    };
    let mut out = String::from("<b>📈 Active Positions</b>\n\n");
    let mut found = false;
    for sym in &["BTCUSDT", "ETHUSDT"] {
        if let Ok(Some(p)) = db.get_position(sym) {
            found = true;
            out.push_str(&format!(
                "<b>{}</b>: qty={} px=${} stop=${}\n",
                p.symbol, p.qty, p.entry_price, p.current_stop_price
            ));
        }
    }
    if let Ok(subs) = crate::sub_account::get_all_sub_account_positions(db.conn()) {
        for (sub, p) in subs {
            found = true;
            out.push_str(&format!(
                "[{sub}] <b>{}</b>: qty={} px=${} stop=${}\n",
                p.symbol, p.qty, p.entry_price, p.current_stop_price
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
    let (hwm, eq, dd, hr) = get_eq_stats(&db);
    let mut out = format!(
        "<b>💰 PnL & Balance Overview</b>\n\n<b>Total Equity:</b> ${eq}\n<b>HWM:</b> ${hwm}\n<b>Max DD:</b> {dd}%\n<b>Headroom:</b> {hr}%\n"
    );
    if let Ok(subs) = crate::sub_account::list_sub_accounts(db.conn()) {
        if !subs.is_empty() {
            out.push_str("\n<b>Sub-Account Balances:</b>\n");
            for s in subs {
                out.push_str(&format!(
                    "• <b>{}</b>: ${} (HWM: ${}, DD: {}%)\n",
                    s.name, s.current_equity, s.high_water_mark, s.max_drawdown_pct
                ));
            }
        }
    }
    out
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
    for sub in &["alphabreakout", "alphacarry", "alphamacro"] {
        let _ = crate::sub_account::set_sub_account_halt(
            db.conn(),
            sub,
            true,
            Some("operator telegram flatten"),
            now_ms,
        );
    }
    match db.set_halt_state(true, Some("operator telegram flatten"), now_ms) {
        Ok(_) => {
            "🚨 <b>FLATTEN ENGAGED</b>\n\nEngine & sub-accounts halted via Telegram.".to_string()
        }
        Err(e) => format!("⚠️ Failed to engage halt: {e}"),
    }
}

fn handle_help() -> String {
    "🤖 <b>Alpha Telegram Controls</b>\n\n\
     • <b>📊 Status</b> | <b>📈 Positions</b>\n\
     • <b>💰 PnL</b> | <b>🚨 Flatten</b>\n\
     • <b>/orders</b> | <b>/help</b>"
        .to_string()
}

pub fn dispatch_command(text: &str, db_path: &str, mode: &str, symbols: &[String]) -> String {
    let clean = text.trim().to_lowercase();
    let cmd = clean.split('@').next().unwrap_or("").trim();
    match cmd {
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
    let kb = json!({
        "keyboard": [[{"text": "📊 Status"}, {"text": "📈 Positions"}], [{"text": "💰 PnL"}, {"text": "🚨 Flatten"}]],
        "resize_keyboard": true, "is_persistent": true
    });
    let payload = json!({"chat_id": cfg.authorized_chat_id, "text": reply, "parse_mode": "HTML", "reply_markup": kb});
    let _ = client.post(&send_url).json(&payload).send().await;
}

pub async fn run_telegram_listener(cfg: ListenerConfig) {
    let client = Client::builder()
        .timeout(Duration::from_secs(25))
        .build()
        .unwrap_or_default();
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
