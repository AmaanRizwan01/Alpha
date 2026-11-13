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

pub mod binance;
pub mod binance_ws;
pub mod error;
pub mod fapi;
pub mod okx;
pub mod signer;

pub use binance::{build_binance_atomic_otoco, BinanceOtocoParams};
pub use binance_ws::{
    BinanceCombinedStreamMessage, BinanceWsBookTicker, BinanceWsKlineData, BinanceWsKlineEvent,
    BinanceWsMessage, BinanceWsTrade,
};
pub use error::VenueError;
pub use fapi::{
    build_funding_rate_path, build_premium_index_path, parse_funding_rates, parse_premium_index,
    BinanceFundingRateRecord, BinancePremiumIndexRecord,
};
pub use okx::{
    build_okx_atomic_order, OkxAttachAlgo, OkxBookSequencer, OkxDeadManPayload, OkxOrderPayload,
};
pub use signer::{sign_binance, sign_okx};
