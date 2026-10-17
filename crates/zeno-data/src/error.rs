//! Error types for live data store and binding engine.

use thiserror::Error;

/// Errors arising during data ingestion, conversion, or binding resolution.
#[derive(Debug, Error)]
pub enum DataError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("CSV format error: {0}")]
    Csv(String),

    #[error("Invalid data path: {0}")]
    InvalidPath(String),

    #[error("State machine error: {0}")]
    StateMachine(String),
}
