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

pub mod carry;
pub mod carry_sim;
pub mod carry_sizing;
pub mod chop_gate;
pub mod error;
pub mod exits;
pub mod h1;
pub mod indicators;
pub mod pullback;
pub mod pullback_sim;
pub mod risk;
pub mod series;
pub mod sizing;
pub mod turtle;
pub mod turtle_sim;

pub use carry::{
    calc_funding_apr_series, check_adl_desync, funding_rate_to_apr, simulate_carry_series,
    update_funding_ema, CarryExitReason, FundingCarryConfig, SimulatedCarryTrade,
};
pub use carry_sizing::{calculate_carry_sizing, CarrySizing, CarrySizingVerdict};
pub use chop_gate::{ChopFilter, ChopVerdict};
pub use error::CoreError;
pub use exits::{calculate_initial_stop, ActiveTrade, ExitConfig, ExitReason, ExitVerdict};
pub use h1::{evaluate_h1_signal, H1Config, H1Verdict};
pub use indicators::{
    average_true_range, donchian_channel, kaufman_efficiency_ratio, true_range, volume_sma,
};
pub use pullback::{
    evaluate_pullback_bar_exit, evaluate_pullback_entry, PullbackConfig, PullbackExitReason,
    PullbackExitState, PullbackVerdict,
};
pub use pullback_sim::{simulate_pullback_series, SimulatedPullbackTrade};
pub use risk::{RiskPolicy, RiskVerdict};
pub use series::{calc_atr_series, calc_ma_series, calc_rsi_series};
pub use sizing::{calculate_position_size, ExchangeFilter, FeeBudget, PositionSize};
pub use turtle::{
    evaluate_turtle_entry, evaluate_turtle_exit, TurtleConfig, TurtleExitReason, TurtleVerdict,
};
pub use turtle_sim::{simulate_turtle_series, SimulatedTurtleTrade};
