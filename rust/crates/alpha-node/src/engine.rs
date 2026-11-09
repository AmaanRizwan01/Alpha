use crate::config::AppConfig;
use crate::db::DbManager;
use crate::error::NodeError;
use crate::reconcile::{reconcile_cold_boot, ReconcileOutcome};
use crate::simulator::{PaperFill, PaperSimulator};
use crate::ws::{build_stream_url, connect_binance_stream, parse_ws_frame, LiveMarketEvent};
use alpha_core::exits::ExitConfig;
use alpha_types::MarketTrade;
use futures_util::StreamExt;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio_tungstenite::tungstenite::Message;

fn current_time_ms() -> i64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    i64::try_from(now.as_millis()).unwrap_or(0)
}

pub struct TradeEngine {
    pub db: DbManager,
    pub config: AppConfig,
    pub simulator: PaperSimulator,
    pub exit_config: ExitConfig,
}

impl TradeEngine {
    pub fn new(config: AppConfig) -> Result<Self, NodeError> {
        let db = DbManager::open(&config.system.database_path)?;
        let (is_halted, reason) = db.get_halt_state()?;
        if is_halted {
            return Err(NodeError::StateError {
                reason: format!("Engine halted: {}", reason.unwrap_or_default()),
            });
        }

        let now_ms = current_time_ms();
        for sym in &config.universe.symbols {
            let outcome = reconcile_cold_boot(&db, sym, None, false, now_ms)?;
            if let ReconcileOutcome::Halted { reason } = outcome {
                return Err(NodeError::StateError { reason });
            }
        }

        let (_, current_eq, _) = db.get_equity_hwm()?;
        let sim = PaperSimulator::new(current_eq, 0, 10, 5);

        Ok(Self {
            db,
            config,
            simulator: sim,
            exit_config: ExitConfig::default(),
        })
    }

    pub fn handle_market_event(
        &mut self,
        event: LiveMarketEvent,
        now_ms: i64,
    ) -> Result<(), NodeError> {
        match event {
            LiveMarketEvent::BookTicker(_) => Ok(()),
            LiveMarketEvent::Trade(trade) => self.on_trade(&trade, now_ms),
            LiveMarketEvent::Kline {
                symbol,
                bar,
                is_closed,
            } => {
                if is_closed {
                    let atr = (bar.high - bar.low).max(dec!(100.0));
                    if let Some(fill) =
                        self.simulator
                            .process_bar(&symbol, &bar, atr, &self.exit_config)?
                    {
                        self.record_fill(&fill, now_ms)?;
                    }
                }
                Ok(())
            }
        }
    }

    fn on_trade(&mut self, trade: &MarketTrade, now_ms: i64) -> Result<(), NodeError> {
        if let Some(fill) = self.simulator.process_trade(trade)? {
            self.record_fill(&fill, now_ms)?;
        }
        Ok(())
    }

    fn record_fill(&mut self, fill: &PaperFill, now_ms: i64) -> Result<(), NodeError> {
        match fill {
            PaperFill::Entry {
                order_id,
                symbol,
                price,
                qty,
                ..
            } => {
                self.db.update_order_status(order_id, "FILLED", now_ms)?;
                let initial_stop = *price * dec!(0.975);
                self.db.upsert_position(
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
            }
            PaperFill::StopLoss {
                symbol,
                fill_price,
                qty,
                pnl,
                ..
            } => {
                self.db.delete_position(symbol)?;
                let (_hwm, dd) = self
                    .db
                    .update_equity_hwm(self.simulator.cash_balance, now_ms)?;
                println!(
                    "[paper-fill] STOP_LOSS {} qty={} px={} pnl={} | equity={} (dd={:.2}%)",
                    symbol, qty, fill_price, pnl, self.simulator.cash_balance, dd
                );
                if dd >= Decimal::from(6) {
                    println!(
                        "[circuit-breaker] Drawdown reached {:.2}% >= 6.0%. HALTED.",
                        dd
                    );
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
                self.db.delete_position(symbol)?;
                self.db
                    .update_equity_hwm(self.simulator.cash_balance, now_ms)?;
                println!(
                    "[paper-fill] EXIT {} qty={} px={} pnl={} | equity={}",
                    symbol, qty, fill_price, pnl, self.simulator.cash_balance
                );
            }
        }
        Ok(())
    }

    pub async fn run(&mut self, max_frames: Option<usize>) -> Result<(), NodeError> {
        let stream_url = build_stream_url(&self.config.universe.symbols, &["1h", "4h"]);
        println!("[ws] Connecting to Binance stream: {}", stream_url);

        let mut ws_stream = connect_binance_stream(&stream_url).await?;
        println!("[ws] Connected. Live market feed active.");

        let mut count = 0;
        let mut last_tick = std::time::Instant::now();

        while let Some(msg_result) = ws_stream.next().await {
            let msg = msg_result?;
            if let Message::Text(text) = msg {
                let now_ms = current_time_ms();
                if let Some(event) = parse_ws_frame(&text, now_ms)? {
                    self.handle_market_event(event, now_ms)?;
                }
                count += 1;
                if let Some(max) = max_frames {
                    if count >= max {
                        println!("[ws] Reached max test frames ({}). Stopping.", max);
                        break;
                    }
                }
            }

            if last_tick.elapsed().as_secs() >= self.config.monitoring.heartbeat_interval_secs {
                let (halted, reason) = self.db.get_halt_state()?;
                if halted {
                    println!(
                        "[engine] Halt state engaged ({}). Exiting loop.",
                        reason.unwrap_or_default()
                    );
                    break;
                }
                let (hwm, eq, dd) = self.db.get_equity_hwm()?;
                println!(
                    "[daemon-tick] equity={} HWM={} max_dd={}% | sim_cash={}",
                    eq, hwm, dd, self.simulator.cash_balance
                );
                last_tick = std::time::Instant::now();
            }
        }

        Ok(())
    }
}
