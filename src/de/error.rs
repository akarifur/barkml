//! Error types for BarkML deserialization.

// Standard library
use std::fmt;

// External crates
use serde::de;
use snafu::Snafu;

/// Result type for deserialization operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Deserialization error types.
#[derive(Debug, Snafu, Clone, PartialEq)]
#[snafu(visibility(pub))]
pub enum Error {
    /// Custom error message
    #[snafu(display("{message}"))]
    Message { message: String },

    /// Type mismatch error
    #[snafu(display("type mismatch: expected {expected}, found {found}"))]
    TypeMismatch { expected: String, found: String },

    /// Missing field error
    #[snafu(display("missing field: {field}"))]
    MissingField { field: String },

    /// Invalid value error
    #[snafu(display("invalid value '{value}' for type {expected_type}"))]
    InvalidValue {
        value: String,
        expected_type: String,
    },

    /// Unsupported operation error
    #[snafu(display("unsupported operation: {operation}"))]
    UnsupportedOperation { operation: String },

    /// Index out of bounds error
    #[snafu(display("index {index} out of bounds for length {length}"))]
    IndexOutOfBounds { index: usize, length: usize },

    /// Key not found error
    #[snafu(display("key not found: {key}"))]
    KeyNotFound { key: String },

    /// Unresolved reference encountered during deserialization
    #[snafu(display("unresolved reference: references must be resolved before deserialization"))]
    UnresolvedReference,
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(msg: T) -> Self {
        Error::Message {
            message: msg.to_string(),
        }
    }
}
