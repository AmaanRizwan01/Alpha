use thiserror::Error;

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("decimal conversion error: {0}")]
    DecimalError(#[from] rust_decimal::Error),

    #[error("core calculation error: {0}")]
    CoreError(#[from] alpha_core::CoreError),

    #[error("type error: {0}")]
    TypeError(#[from] alpha_types::TypeError),

    #[error("venue error: {0}")]
    VenueError(#[from] alpha_venues::VenueError),

    #[error("websocket error: {0}")]
    WebSocket(String),

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

impl From<tokio_tungstenite::tungstenite::Error> for NodeError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(err.to_string())
    }
}
