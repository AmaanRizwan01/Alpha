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

pub mod chop_gate;
pub mod error;
pub mod exits;
pub mod h1;
pub mod indicators;
pub mod risk;
pub mod sizing;

pub use chop_gate::{ChopFilter, ChopVerdict};
pub use error::CoreError;
pub use exits::{calculate_initial_stop, ActiveTrade, ExitConfig, ExitReason, ExitVerdict};
pub use h1::{evaluate_h1_signal, H1Config, H1Verdict};
pub use indicators::{
    average_true_range, donchian_channel, kaufman_efficiency_ratio, true_range, volume_sma,
};
pub use risk::{RiskPolicy, RiskVerdict};
pub use sizing::{calculate_position_size, ExchangeFilter, FeeBudget, PositionSize};
