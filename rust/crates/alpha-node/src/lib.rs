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

pub mod bootstrap;
pub mod buffer;
pub mod config;
pub mod db;
pub mod engine;
pub mod error;
pub mod fill_recorder;
pub mod reconcile;
pub mod signal;
pub mod simulator;
pub mod ws;

pub use bootstrap::{fetch_binance_klines, parse_binance_kline_value};
pub use buffer::BarBuffer;
pub use config::AppConfig;
pub use db::{DbManager, PositionRecord};
pub use engine::TradeEngine;
pub use error::NodeError;
pub use reconcile::{reconcile_cold_boot, ReconcileOutcome};
pub use signal::{default_filter_for_symbol, evaluate_symbol_bar, LiveSignalVerdict};
pub use simulator::{PaperFill, PaperPosition, PaperSimulator};
pub use ws::{build_stream_url, connect_binance_stream, parse_ws_frame, LiveMarketEvent};
