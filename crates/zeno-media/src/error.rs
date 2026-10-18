//! Error types encountered during capture source initialization and execution.

use thiserror::Error;

/// Errors arising from capture device initialization, execution, or frame ingestion.
#[derive(Debug, Error)]
pub enum SourceError {
    /// Target capture device or window was not found.
    #[error("Capture target not found: {0}")]
    TargetNotFound(String),

    /// Failed to initialize capture session or GPU resources.
    #[error("Source initialization failure: {0}")]
    InitializationFailed(String),

    /// Frame acquisition or texture upload failed.
    #[error("Frame capture error: {0}")]
    CaptureFailed(String),

    /// Invalid properties passed during runtime reconfiguration.
    #[error("Invalid properties: {0}")]
    InvalidProperties(String),

    /// Underlying Windows platform error.
    #[error("Platform API error: {0}")]
    PlatformError(String),
}
