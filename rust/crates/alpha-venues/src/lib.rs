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
pub mod error;
pub mod okx;
pub mod signer;

pub use binance::{build_binance_atomic_otoco, BinanceOtocoParams};
pub use error::VenueError;
pub use okx::{
    build_okx_atomic_order, OkxAttachAlgo, OkxBookSequencer, OkxDeadManPayload, OkxOrderPayload,
};
pub use signer::{sign_binance, sign_okx};
