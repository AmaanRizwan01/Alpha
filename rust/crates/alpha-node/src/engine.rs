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
                interval,
                bar,
                is_closed,
            } => {
                if is_closed && interval == self.config.universe.timeframe {
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
        let atr = match self.update_buffer_and_calc_atr(symbol, bar.clone()) {
            Ok(a) => a,
            Err(e) => {
                eprintln!("[engine] skipping bar for {}: {}", symbol, e);
                return Ok(());
            }
        };
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

    fn check_heartbeat(&mut self, last_tick: &mut std::time::Instant) -> Result<bool, NodeError> {
        if last_tick.elapsed().as_secs() < self.config.monitoring.heartbeat_interval_secs {
            return Ok(false);
        }
        *last_tick = std::time::Instant::now();
        let (halted, reason) = self.db.get_halt_state()?;
        if halted {
            let r = reason.unwrap_or_default();
            println!("[engine] Halt engaged ({r}). Exiting loop.");
            return Ok(true);
        }
        let (hwm, eq, dd) = self.db.get_equity_hwm()?;
        println!(
            "[daemon-tick] equity={eq} HWM={hwm} max_dd={dd}% | sim_cash={}",
            self.simulator.cash_balance
        );
        Ok(false)
    }

    fn process_ws_text(&mut self, text: &str, now_ms: i64) {
        if let Ok(Some(event)) = parse_ws_frame(text, now_ms) {
            if let Err(e) = self.handle_market_event(event, now_ms) {
                eprintln!("[engine warning] Event handling error: {e}");
            }
        }
    }

    pub async fn run(&mut self, max_frames: Option<usize>) -> Result<(), NodeError> {
        crate::bootstrap::bootstrap_symbol_buffers(
            &self.config.universe.symbols,
            &self.config.universe.timeframe,
            &mut self.bar_buffers,
        )
        .await;
        let stream_url = build_stream_url(
            &self.config.universe.symbols,
            &[&self.config.universe.timeframe],
        );
        let (_, eq, _) = self.db.get_equity_hwm()?;
        self.telegram
            .notify_startup(&self.config.system.mode, &self.config.universe.symbols, eq);

        let mut count = 0;
        let mut last_tick = std::time::Instant::now();

        loop {
            if self.db.get_halt_state()?.0 {
                break;
            }
            println!("[ws] Connecting to Binance stream: {stream_url}");
            let mut ws = match connect_binance_stream(&stream_url).await {
                Ok(stream) => {
                    println!("[ws] Connected. Live market feed active.");
                    stream
                }
                Err(e) => {
                    eprintln!("[ws warning] Connection failed: {e}. Retrying in 3s...");
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    continue;
                }
            };

            while let Some(msg_result) = ws.next().await {
                match msg_result {
                    Ok(Message::Text(text)) => {
                        let now_ms = current_time_ms();
                        self.process_ws_text(&text, now_ms);
                        count += 1;
                        if max_frames.is_some_and(|max| count >= max) {
                            return Ok(());
                        }
                    }
                    Ok(Message::Ping(_)) => continue,
                    Ok(Message::Close(_)) => {
                        println!("[ws] Stream closed by remote. Reconnecting...");
                        break;
                    }
                    Ok(_) => continue,
                    Err(e) => {
                        eprintln!("[ws warning] Stream error: {e}. Reconnecting...");
                        break;
                    }
                }

                if self.check_heartbeat(&mut last_tick)? {
                    return Ok(());
                }
            }

            if max_frames.is_some() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
        Ok(())
    }
}
