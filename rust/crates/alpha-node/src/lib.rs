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
pub mod error;
pub mod reconcile;
pub mod simulator;
pub mod ws;

pub use config::AppConfig;
pub use db::{DbManager, PositionRecord};
pub use error::NodeError;
pub use reconcile::{reconcile_cold_boot, ReconcileOutcome};
pub use simulator::{PaperFill, PaperPosition, PaperSimulator};
pub use ws::{build_stream_url, connect_binance_stream, parse_ws_frame, LiveMarketEvent};
