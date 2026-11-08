use thiserror::Error;

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("decimal conversion error: {0}")]
    DecimalError(#[from] rust_decimal::Error),

    #[error("state error: {reason}")]
    StateError { reason: String },

    #[error("circuit breaker tripped: {reason}")]
    CircuitBreaker { reason: String },
}
