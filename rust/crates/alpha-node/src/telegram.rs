use reqwest::Client;
use rust_decimal::Decimal;
use serde_json::json;
use std::env;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct TelegramNotifier {
    bot_token: Option<String>,
    chat_id: Option<String>,
    client: Client,
}

pub fn load_env_var(key1: &str, key2: &str) -> Option<String> {
    if let Ok(val) = env::var(key1).or_else(|_| env::var(key2)) {
        if !val.trim().is_empty() {
            return Some(val);
        }
    }
    for path in &[".env", "/opt/alpha/.env"] {
        if let Ok(content) = std::fs::read_to_string(path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with('#') || !trimmed.contains('=') {
                    continue;
                }
                let mut parts = trimmed.splitn(2, '=');
                let k = parts.next().unwrap_or("").trim();
                let v = parts.next().unwrap_or("").trim().trim_matches('"');
                if (k == key1 || k == key2) && !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

impl TelegramNotifier {
    pub fn new(bot_token: Option<String>, chat_id: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self {
            bot_token,
            chat_id,
            client,
        }
    }

    pub fn from_env() -> Self {
        let bot_token = load_env_var("TELEGRAM_BOT_TOKEN", "TG_TOKEN");
        let chat_id = load_env_var("TELEGRAM_CHAT_ID", "TG_CHAT_ID");
        Self::new(bot_token, chat_id)
    }

    pub fn token(&self) -> Option<&str> {
        self.bot_token.as_deref()
    }

    pub fn chat_id(&self) -> Option<&str> {
        self.chat_id.as_deref()
    }

    pub fn is_configured(&self) -> bool {
        self.bot_token.is_some() && self.chat_id.is_some()
    }

    pub fn send_alert(&self, text: String) {
        let (token, chat) = match (&self.bot_token, &self.chat_id) {
            (Some(t), Some(c)) => (t.clone(), c.clone()),
            _ => return,
        };

        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            let client = self.client.clone();
            handle.spawn(async move {
                let url = format!("https://api.telegram.org/bot{token}/sendMessage");
                let payload = json!({
                    "chat_id": chat,
                    "text": text,
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
                if let Err(e) = client.post(&url).json(&payload).send().await {
                    eprintln!("[telegram warning] alert failed: {e}");
                }
            });
        }
    }

    pub fn notify_startup(&self, mode: &str, symbols: &[String], equity: Decimal) {
        let syms = symbols.join(", ");
        let msg = format!(
            "<b>Alpha Node v0.1.0 Started</b>\n\
             Mode: <code>{mode}</code>\n\
             Symbols: <code>{syms}</code>\n\
             Initial Equity: <b>${equity}</b>\n\
             Status: Buffers pre-warmed & WebSocket connected"
        );
        self.send_alert(msg);
    }

    pub fn notify_signal_approved(
        &self,
        symbol: &str,
        entry: Decimal,
        stop: Decimal,
        stop_pct: Decimal,
        qty: Decimal,
    ) {
        let msg = format!(
            "<b>Signal APPROVED: {symbol}</b>\n\
             Action: BUY BREAKOUT\n\
             Entry Limit: <b>${entry}</b>\n\
             Initial Stop: <b>${stop}</b> (-{stop_pct}%)\n\
             Size: <b>{qty}</b> {symbol}"
        );
        self.send_alert(msg);
    }

    pub fn notify_order_submitted(
        &self,
        symbol: &str,
        side: &str,
        qty: Decimal,
        price: Option<Decimal>,
        order_type: &str,
    ) {
        let px_str = price.map_or("MARKET".to_string(), |p| format!("${p}"));
        let msg = format!(
            "<b>Order Submitted</b>\n\
             Symbol: <code>{symbol}</code>\n\
             Side: <b>{side}</b> | Type: {order_type}\n\
             Price: {px_str} | Qty: {qty}"
        );
        self.send_alert(msg);
    }

    pub fn notify_fill(
        &self,
        symbol: &str,
        side: &str,
        qty: Decimal,
        price: Decimal,
        fee: Decimal,
        cash: Decimal,
    ) {
        let msg = format!(
            "<b>Paper Fill Executed</b>\n\
             Symbol: <code>{symbol}</code>\n\
             Side: <b>{side}</b> | Fill Price: <b>${price}</b>\n\
             Quantity: {qty} | Fee: ${fee}\n\
             Remaining Cash: <b>${cash}</b>"
        );
        self.send_alert(msg);
    }

    pub fn notify_position_closed(
        &self,
        symbol: &str,
        exit_price: Decimal,
        pnl: Decimal,
        pnl_r: Decimal,
        reason: &str,
    ) {
        let outcome = if pnl >= Decimal::ZERO {
            "PROFIT"
        } else {
            "LOSS"
        };
        let msg = format!(
            "<b>Position Closed ({outcome})</b>\n\
             Symbol: <code>{symbol}</code>\n\
             Exit Price: <b>${exit_price}</b>\n\
             Net PnL: <b>${pnl}</b> ({pnl_r}R)\n\
             Reason: <i>{reason}</i>"
        );
        self.send_alert(msg);
    }

    pub fn notify_circuit_breaker(&self, reason: &str, dd_pct: Decimal) {
        let msg = format!(
            "<b>CIRCUIT BREAKER TRIPPED</b>\n\
             Current Drawdown: <b>{dd_pct}%</b> (Limit: 6.0%)\n\
             Reason: <i>{reason}</i>\n\
             Action: Trading HALTED immediately."
        );
        self.send_alert(msg);
    }

    pub fn notify_adl_desync(&self, symbol: &str, spot_qty: Decimal, perp_qty: Decimal) {
        let msg = format!(
            "<b>EMERGENCY: ADL DESYNC DETECTED</b>\n\
             Symbol: <code>{symbol}</code>\n\
             Spot Qty: <b>{spot_qty}</b> | Perp Qty: <b>{perp_qty}</b>\n\
             Action: Emergency Market Sell Spot & Trading HALTED immediately."
        );
        self.send_alert(msg);
    }
}
