use crate::error::NodeError;
use alpha_core::exits::{ActiveTrade, ExitConfig, ExitReason, ExitVerdict};
use alpha_types::{Bar, MarketTrade, OrderIntent, Side, Venue};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaperPosition {
    pub symbol: String,
    pub venue: Venue,
    pub qty: Decimal,
    pub entry_fee: Decimal,
    pub trade: ActiveTrade,
    pub take_profit_price: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaperFill {
    Entry {
        order_id: String,
        symbol: String,
        price: Decimal,
        qty: Decimal,
        fee: Decimal,
        timestamp_ms: i64,
    },
    StopLoss {
        symbol: String,
        trigger_price: Decimal,
        fill_price: Decimal,
        qty: Decimal,
        fee: Decimal,
        pnl: Decimal,
        timestamp_ms: i64,
    },
    TakeProfit {
        symbol: String,
        fill_price: Decimal,
        qty: Decimal,
        fee: Decimal,
        pnl: Decimal,
        timestamp_ms: i64,
    },
    BarExit {
        symbol: String,
        reason: ExitReason,
        fill_price: Decimal,
        qty: Decimal,
        fee: Decimal,
        pnl: Decimal,
        timestamp_ms: i64,
    },
}

#[derive(Debug, Clone)]
pub struct PaperSimulator {
    pub cash_balance: Decimal,
    pub maker_fee_bps: Decimal,
    pub taker_fee_bps: Decimal,
    pub slippage_bps: Decimal,
    pub working_order: Option<OrderIntent>,
    pub active_position: Option<PaperPosition>,
}

impl PaperSimulator {
    pub fn new(
        initial_balance: Decimal,
        maker_fee_bps: u32,
        taker_fee_bps: u32,
        slippage_bps: u32,
    ) -> Self {
        Self {
            cash_balance: initial_balance,
            maker_fee_bps: Decimal::from(maker_fee_bps),
            taker_fee_bps: Decimal::from(taker_fee_bps),
            slippage_bps: Decimal::from(slippage_bps),
            working_order: None,
            active_position: None,
        }
    }

    pub fn submit_order(&mut self, intent: OrderIntent) -> Result<(), NodeError> {
        if self.active_position.is_some() || self.working_order.is_some() {
            return Err(NodeError::StateError {
                reason: "cannot submit order: position or working order already active".into(),
            });
        }
        let price = intent.price.ok_or_else(|| NodeError::StateError {
            reason: "paper entry order requires limit price".into(),
        })?;
        let notional = intent.qty * price;
        if notional > self.cash_balance {
            return Err(NodeError::StateError {
                reason: format!("insufficient cash: {} > {}", notional, self.cash_balance),
            });
        }
        self.working_order = Some(intent);
        Ok(())
    }

    pub fn cancel_working_order(&mut self) -> Option<OrderIntent> {
        self.working_order.take()
    }

    pub fn process_trade(&mut self, trade: &MarketTrade) -> Result<Option<PaperFill>, NodeError> {
        if let Some(fill) = self.check_working_entry(trade)? {
            return Ok(Some(fill));
        }
        self.check_position_stops(trade)
    }

    fn check_working_entry(&mut self, trade: &MarketTrade) -> Result<Option<PaperFill>, NodeError> {
        let order = match &self.working_order {
            Some(o) if o.symbol == trade.symbol => o.clone(),
            _ => return Ok(None),
        };
        let limit_px = order.price.unwrap_or(Decimal::ZERO);
        if order.side == Side::Buy && trade.price <= limit_px {
            let fill_px = limit_px;
            let notional = order.qty * fill_px;
            let fee = notional * (self.maker_fee_bps / dec!(10000));
            self.cash_balance -= notional + fee;
            let stop_px = order.stop_loss_price.unwrap_or(fill_px * dec!(0.97));
            let active_trade = ActiveTrade::new(fill_px, stop_px)?;

            self.active_position = Some(PaperPosition {
                symbol: order.symbol.clone(),
                venue: order.venue,
                qty: order.qty,
                entry_fee: fee,
                trade: active_trade,
                take_profit_price: order.take_profit_price,
            });
            self.working_order = None;

            return Ok(Some(PaperFill::Entry {
                order_id: order.client_order_id,
                symbol: order.symbol,
                price: fill_px,
                qty: order.qty,
                fee,
                timestamp_ms: trade.timestamp_ms,
            }));
        }
        Ok(None)
    }

    fn check_position_stops(
        &mut self,
        trade: &MarketTrade,
    ) -> Result<Option<PaperFill>, NodeError> {
        let pos = match &mut self.active_position {
            Some(p) if p.symbol == trade.symbol => p,
            _ => return Ok(None),
        };

        if trade.price <= pos.trade.current_stop {
            let slip_frac = self.slippage_bps / dec!(10000);
            let fill_px = pos.trade.current_stop * (Decimal::ONE - slip_frac);
            let proceeds = pos.qty * fill_px;
            let fee = proceeds * (self.taker_fee_bps / dec!(10000));
            let entry_cost = (pos.qty * pos.trade.entry_price) + pos.entry_fee;
            let pnl = (proceeds - fee) - entry_cost;
            self.cash_balance += proceeds - fee;

            let trig_px = pos.trade.current_stop;
            let sym = pos.symbol.clone();
            let qty = pos.qty;
            self.active_position = None;

            return Ok(Some(PaperFill::StopLoss {
                symbol: sym,
                trigger_price: trig_px,
                fill_price: fill_px,
                qty,
                fee,
                pnl,
                timestamp_ms: trade.timestamp_ms,
            }));
        }

        if let Some(tp) = pos.take_profit_price {
            if trade.price >= tp {
                let proceeds = pos.qty * tp;
                let fee = proceeds * (self.maker_fee_bps / dec!(10000));
                let entry_cost = (pos.qty * pos.trade.entry_price) + pos.entry_fee;
                let pnl = (proceeds - fee) - entry_cost;
                self.cash_balance += proceeds - fee;

                let sym = pos.symbol.clone();
                let qty = pos.qty;
                self.active_position = None;

                return Ok(Some(PaperFill::TakeProfit {
                    symbol: sym,
                    fill_price: tp,
                    qty,
                    fee,
                    pnl,
                    timestamp_ms: trade.timestamp_ms,
                }));
            }
        }

        pos.trade.highest_high = pos.trade.highest_high.max(trade.price);
        Ok(None)
    }

    pub fn process_bar(
        &mut self,
        symbol: &str,
        bar: &Bar,
        atr: Decimal,
        config: &ExitConfig,
    ) -> Result<Option<PaperFill>, NodeError> {
        let pos = match &mut self.active_position {
            Some(p) if p.symbol == symbol => p,
            _ => return Ok(None),
        };

        let verdict = pos.trade.update_bar(bar, atr, config)?;
        if let ExitVerdict::Exit {
            reason, exit_price, ..
        } = verdict
        {
            let slip_frac = if reason == ExitReason::StagnationTimeStop {
                Decimal::ZERO
            } else {
                self.slippage_bps / dec!(10000)
            };
            let fill_px = exit_price * (Decimal::ONE - slip_frac);
            let proceeds = pos.qty * fill_px;
            let fee = proceeds * (self.taker_fee_bps / dec!(10000));
            let entry_cost = (pos.qty * pos.trade.entry_price) + pos.entry_fee;
            let pnl = (proceeds - fee) - entry_cost;
            self.cash_balance += proceeds - fee;

            let sym = pos.symbol.clone();
            let qty = pos.qty;
            self.active_position = None;

            return Ok(Some(PaperFill::BarExit {
                symbol: sym,
                reason,
                fill_price: fill_px,
                qty,
                fee,
                pnl,
                timestamp_ms: bar.close_time_ms,
            }));
        }
        Ok(None)
    }
}
