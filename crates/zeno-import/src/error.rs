//! Error types for asset and project importers.

use thiserror::Error;

/// Errors arising during PSD or Lottie asset importation.
#[derive(Debug, Error)]
pub enum ImportError {
    #[error("Invalid or unrecognized file signature: {0}")]
    InvalidSignature(String),

    #[error("Unsupported format version: {0}")]
    UnsupportedVersion(u16),

    #[error("Malformed layer structure: {0}")]
    MalformedLayer(String),

    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Image asset decoding/encoding error: {0}")]
    Image(String),

    #[error("General parse error: {0}")]
    Parse(String),
}
