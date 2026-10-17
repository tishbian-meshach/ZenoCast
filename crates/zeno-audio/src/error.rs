//! Error types for audio capture, mixing, and DSP processing.

use thiserror::Error;
use zeno_core::SourceId;

/// Audio subsystem errors covering device capture, resampling, and DSP failures.
#[derive(Debug, Error)]
pub enum AudioError {
    /// Device not found or disconnected.
    #[error("Audio device not found: {0}")]
    DeviceNotFound(String),

    /// Audio source not found.
    #[error("Audio source not found: {0}")]
    SourceNotFound(SourceId),

    /// WASAPI stream creation or initialization error.
    #[error("WASAPI stream error: {0}")]
    StreamError(String),

    /// Rubato resampler initialization or processing error.
    #[error("Resampling error: {0}")]
    ResampleError(String),

    /// Invalid channel count or sample rate.
    #[error("Unsupported audio format: {channels} channels @ {sample_rate} Hz")]
    UnsupportedFormat { channels: usize, sample_rate: u32 },
}
