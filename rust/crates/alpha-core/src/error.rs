use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum CoreError {
    #[error("insufficient bars for calculation: required {required}, provided {provided}")]
    InsufficientBars { required: usize, provided: usize },

    #[error("invalid parameter '{name}': {reason}")]
    InvalidParameter {
        name: &'static str,
        reason: &'static str,
    },

    #[error("non-finite statistical calculation result: {name}")]
    NonFiniteCalculation { name: &'static str },

    #[error("type error: {0}")]
    TypeError(#[from] alpha_types::TypeError),
}
