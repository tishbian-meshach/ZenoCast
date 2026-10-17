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

    #[error("Spreadsheet error: {0}")]
    Spreadsheet(#[from] SheetError),
}

/// Errors specific to spreadsheet file reading, parsing, and polling.
#[derive(Debug, thiserror::Error, Clone, PartialEq, Eq)]
pub enum SheetError {
    #[error("File not found: {0}")]
    FileNotFound(std::path::PathBuf),

    #[error("File is locked by another process: {0}")]
    FileLocked(std::path::PathBuf),

    #[error("Excel lock file ignored: {0}")]
    LockFileIgnored(std::path::PathBuf),

    #[error("Sheet not found: '{0}'")]
    SheetNotFound(String),

    #[error("Invalid cell coordinate address: '{0}'")]
    InvalidAddress(String),

    #[error("Archive or format error: {0}")]
    Format(String),

    #[error("Network or fetch error: {0}")]
    Network(String),
}
