//! Error types for hardware encoding, streaming, and recording.

use thiserror::Error;

/// Output pipeline error enumeration.
#[derive(Debug, Error)]
pub enum OutputError {
    /// Standard I/O error occurred during process piping or disk recording.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Required FFmpeg binary was not found on PATH or specified location.
    #[error("FFmpeg executable not found: {0}")]
    FfmpegNotFound(String),

    /// The specified hardware or software encoder is not supported or failed to initialize.
    #[error("Encoder initialization failed for '{encoder}': {details}")]
    EncoderFailed {
        /// Codec name attempted
        encoder: String,
        /// Failure details or driver error
        details: String,
    },

    /// GPU color conversion or compute shader failure.
    #[error("GPU conversion error: {0}")]
    GpuConversion(String),

    /// Streaming destination disconnected or failed to connect.
    #[error("Stream connection failed to '{url}': {reason}")]
    StreamConnectionFailed {
        /// Stream destination endpoint
        url: String,
        /// Failure reason
        reason: String,
    },

    /// Channel send/receive error on real-time queues.
    #[error("Pipeline channel error: {0}")]
    Channel(String),

    /// Invalid encoder or stream parameters.
    #[error("Invalid output parameters: {0}")]
    InvalidParameters(String),

    /// Process or external API operation failed.
    #[error("Process or API error: {0}")]
    ProcessFailed(String),

    /// Muxer or remux operation failed.
    #[error("Remux failed: {0}")]
    RemuxFailed(String),
}

/// Convenience result type for output operations.
pub type OutputResult<T> = Result<T, OutputError>;
