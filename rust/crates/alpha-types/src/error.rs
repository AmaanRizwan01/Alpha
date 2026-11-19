use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum TypeError {
    #[error("invalid bar data: {reason}")]
    InvalidBar { reason: &'static str },

    #[error("invalid price: {reason}")]
    InvalidPrice { reason: &'static str },

    #[error("invalid quantity: {reason}")]
    InvalidQuantity { reason: &'static str },

    #[error("invalid book top: {reason}")]
    InvalidBookTop { reason: &'static str },
}
