use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum VenueError {
    #[error("cryptographic signing error: {reason}")]
    SignError { reason: String },

    #[error("invalid order configuration: {reason}")]
    InvalidOrder { reason: String },

    #[error("websocket sequence gap detected: expected {expected}, got {got}")]
    SequenceGap { expected: i64, got: i64 },

    #[error("serialization error: {reason}")]
    SerializationError { reason: String },

    #[error("type validation error: {0}")]
    TypeError(#[from] alpha_types::TypeError),

    #[error("exchange api rejection: code={code}, msg={message}")]
    ExchangeRejection { code: String, message: String },
}
