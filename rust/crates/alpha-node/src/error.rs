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

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("toml parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("configuration error: {reason}")]
    ConfigError { reason: String },
}
