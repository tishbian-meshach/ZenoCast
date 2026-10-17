//! Core error definitions.

use thiserror::Error;

/// Result alias for Core operations.
pub type CoreResult<T> = Result<T, CoreError>;

/// Fundamental errors occurring within core data types and operations.
#[derive(Debug, Error)]
pub enum CoreError {
    /// Invalid format or syntax in identifier string.
    #[error("Invalid identifier: {0}")]
    InvalidId(String),

    /// Parsing error for numeric or color data.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Data type mismatch during variable resolution.
    #[error("Type mismatch: expected {expected}, found {found}")]
    TypeMismatch {
        /// Expected type name.
        expected: String,
        /// Actual type name.
        found: String,
    },

    /// Missing or undefined property path.
    #[error("Undefined property path: {0}")]
    UndefinedPath(String),

    /// General I/O or conversion error.
    #[error("Internal error: {0}")]
    Internal(String),

    /// File I/O error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Validation or serialization error.
    #[error("Validation error: {0}")]
    Validation(String),
}
