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

pub use config::AppConfig;
pub use db::{DbManager, PositionRecord};
pub use error::NodeError;
pub use reconcile::{reconcile_cold_boot, ReconcileOutcome};
