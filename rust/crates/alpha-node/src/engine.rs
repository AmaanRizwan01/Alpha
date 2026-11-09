use crate::buffer::BarBuffer;
use crate::config::AppConfig;
use crate::db::DbManager;
use crate::error::NodeError;
use crate::fill_recorder::record_paper_fill;
use crate::reconcile::{reconcile_cold_boot, ReconcileOutcome};
use crate::signal::{default_filter_for_symbol, evaluate_symbol_bar, LiveSignalVerdict};
use crate::simulator::PaperSimulator;
use crate::ws::{build_stream_url, connect_binance_stream, parse_ws_frame, LiveMarketEvent};
use alpha_core::exits::ExitConfig;
use alpha_core::h1::H1Config;
use alpha_core::indicators::average_true_range;
use alpha_core::risk::RiskPolicy;
use alpha_core::sizing::{FeeBudget, PositionSize};
use alpha_types::{Bar, MarketTrade, OrderIntent, OrderType, Side, Venue};
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
            record_paper_fill(&self.db, &mut self.simulator, &fill, now_ms)?;
        }
        Ok(())
    }

    fn on_bar_closed(&mut self, symbol: &str, bar: Bar, now_ms: i64) -> Result<(), NodeError> {
        let atr = {
            let buffer = self
                .bar_buffers
                .entry(symbol.to_string())
                .or_insert(BarBuffer::new(100)?);
            buffer.push(bar.clone())?;
            if buffer.len() >= 14 {
                average_true_range(buffer.bars(), 14)
                    .unwrap_or_else(|_| bar.hl_range().max(dec!(100.0)))
            } else {
                bar.hl_range().max(dec!(100.0))
            }
        };

        if let Some(fill) = self
            .simulator
            .process_bar(symbol, &bar, atr, &self.exit_config)?
        {
            record_paper_fill(&self.db, &mut self.simulator, &fill, now_ms)?;
        }

        if self.simulator.active_position.is_some() || self.simulator.working_order.is_some() {
            return Ok(());
        }

        let (hwm, _, _) = self.db.get_equity_hwm()?;
        let filter = default_filter_for_symbol(symbol);
        let buffer = self
            .bar_buffers
            .get(symbol)
            .ok_or_else(|| NodeError::BufferError {
                reason: format!("missing buffer for {}", symbol),
            })?;
        let verdict = evaluate_symbol_bar(
            symbol,
            buffer,
            self.simulator.cash_balance,
            hwm,
            self.config.risk.max_risk_per_trade,
            &self.h1_config,
            &self.exit_config,
            &self.fee_budget,
            &filter,
            &self.risk_policy,
        )?;

        match verdict {
            LiveSignalVerdict::Approved {
                symbol,
                size,
                initial_stop,
                atr,
            } => {
                println!(
                    "[signal-h1] APPROVED for {}: qty={} px={} stop={} notional={} atr={}",
                    symbol, size.qty, size.rounded_entry, initial_stop, size.notional, atr
                );
                self.submit_signal_order(&symbol, &size, initial_stop, now_ms)?;
            }
            LiveSignalVerdict::AbstainSignal { symbol, reason } => {
                println!("[signal-h1] Abstain for {}: {}", symbol, reason);
            }
            LiveSignalVerdict::AbstainRisk { symbol, reason } => {
                println!("[signal-h1] Risk abstain for {}: {}", symbol, reason);
            }
            LiveSignalVerdict::InsufficientBars {
                symbol,
                required,
                provided,
            } => {
                println!(
                    "[signal-h1] Warming buffer for {} ({}/{} bars)",
                    symbol, provided, required
                );
            }
        }
        Ok(())
    }

    fn submit_signal_order(
        &mut self,
        symbol: &str,
        size: &PositionSize,
        stop: Decimal,
        now_ms: i64,
    ) -> Result<(), NodeError> {
        let client_id = format!("paper-h1-{}-{}", symbol, now_ms);
        let intent = OrderIntent::new(
            Venue::Binance,
            symbol,
            Side::Buy,
            OrderType::LimitMaker,
            Some(size.rounded_entry),
            size.qty,
            Some(stop),
            None,
            &client_id,
        )?;
        self.db.insert_order(&intent, "WORKING", now_ms)?;
        self.simulator.submit_order(intent)
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
