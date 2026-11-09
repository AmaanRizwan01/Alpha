use crate::buffer::BarBuffer;
use crate::config::AppConfig;
use crate::db::DbManager;
use crate::error::NodeError;
use crate::fill_recorder::record_paper_fill;
use crate::reconcile::{reconcile_cold_boot, ReconcileOutcome};
use crate::simulator::PaperSimulator;
use crate::telegram::TelegramNotifier;
use crate::ws::{build_stream_url, connect_binance_stream, parse_ws_frame, LiveMarketEvent};
use alpha_core::exits::ExitConfig;
use alpha_core::h1::H1Config;
use alpha_core::indicators::average_true_range;
use alpha_core::risk::RiskPolicy;
use alpha_core::sizing::FeeBudget;
use alpha_types::{Bar, MarketTrade};
use futures_util::StreamExt;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use std::collections::HashMap;
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
    pub bar_buffers: HashMap<String, BarBuffer>,
    pub h1_config: H1Config,
    pub fee_budget: FeeBudget,
    pub risk_policy: RiskPolicy,
    pub telegram: TelegramNotifier,
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
            if let ReconcileOutcome::Halted { reason } =
                reconcile_cold_boot(&db, sym, None, false, now_ms)?
            {
                return Err(NodeError::StateError { reason });
            }
        }

        let mut bar_buffers = HashMap::new();
        for sym in &config.universe.symbols {
            bar_buffers.insert(sym.clone(), BarBuffer::new(100)?);
        }

        let fee_budget = FeeBudget::new(
            config.execution.maker_fee_bps / dec!(10000.0),
            config.execution.taker_fee_bps / dec!(10000.0),
            config.execution.slippage_allowance_bps / dec!(10000.0),
        )?;
        let risk_policy = RiskPolicy::new(dec!(0.50), config.risk.circuit_breaker_drawdown)?;
        let exit_config = ExitConfig {
            min_stop_distance_pct: config.risk.min_stop_distance,
            ..Default::default()
        };
        let (_, current_eq, _) = db.get_equity_hwm()?;
        Ok(Self {
            db,
            config,
            simulator: PaperSimulator::new(current_eq, 0, 10, 5),
            exit_config,
            bar_buffers,
            h1_config: H1Config::default(),
            fee_budget,
            risk_policy,
            telegram: TelegramNotifier::from_env(),
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
                    self.on_bar_closed(&symbol, bar, now_ms)?;
                }
                Ok(())
            }
        }
    }

    fn on_trade(&mut self, trade: &MarketTrade, now_ms: i64) -> Result<(), NodeError> {
        if let Some(fill) = self.simulator.process_trade(trade)? {
            record_paper_fill(&self.db, &mut self.simulator, &fill, &self.telegram, now_ms)?;
        }
        Ok(())
    }

    fn on_bar_closed(&mut self, symbol: &str, bar: Bar, now_ms: i64) -> Result<(), NodeError> {
        let atr = self.update_buffer_and_calc_atr(symbol, bar.clone())?;
        if let Some(fill) = self
            .simulator
            .process_bar(symbol, &bar, atr, &self.exit_config)?
        {
            record_paper_fill(&self.db, &mut self.simulator, &fill, &self.telegram, now_ms)?;
        }
        if self.simulator.active_position.is_some() || self.simulator.working_order.is_some() {
            return Ok(());
        }
        crate::signal_dispatch::evaluate_and_dispatch(self, symbol, now_ms)
    }

    fn update_buffer_and_calc_atr(&mut self, symbol: &str, bar: Bar) -> Result<Decimal, NodeError> {
        let buf = self
            .bar_buffers
            .entry(symbol.to_string())
            .or_insert(BarBuffer::new(100)?);
        buf.push(bar.clone())?;
        let atr = if buf.len() >= 14 {
            average_true_range(buf.bars(), 14).unwrap_or_else(|_| bar.hl_range().max(dec!(100.0)))
        } else {
            bar.hl_range().max(dec!(100.0))
        };
        Ok(atr)
    }

    pub async fn run(&mut self, max_frames: Option<usize>) -> Result<(), NodeError> {
        crate::bootstrap::bootstrap_symbol_buffers(
            &self.config.universe.symbols,
            &self.config.universe.timeframe,
            &mut self.bar_buffers,
        )
        .await;
        let stream_url = build_stream_url(&self.config.universe.symbols, &["1h", "4h"]);
        println!("[ws] Connecting to Binance stream: {}", stream_url);
        let mut ws = connect_binance_stream(&stream_url).await?;
        println!("[ws] Connected. Live market feed active.");

        let (_, eq, _) = self.db.get_equity_hwm()?;
        self.telegram
            .notify_startup(&self.config.system.mode, &self.config.universe.symbols, eq);

        let mut count = 0;
        let mut last_tick = std::time::Instant::now();
        while let Some(msg_result) = ws.next().await {
            if let Message::Text(text) = msg_result? {
                let now_ms = current_time_ms();
                if let Some(event) = parse_ws_frame(&text, now_ms)? {
                    self.handle_market_event(event, now_ms)?;
                }
                count += 1;
                if let Some(max) = max_frames {
                    if count >= max {
                        break;
                    }
                }
            }
            if last_tick.elapsed().as_secs() >= self.config.monitoring.heartbeat_interval_secs {
                let (halted, reason) = self.db.get_halt_state()?;
                if halted {
                    println!(
                        "[engine] Halt engaged ({}). Exiting loop.",
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
