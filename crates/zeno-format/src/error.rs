//! Error types encountered when reading, writing, or validating `.arx` containers.

use thiserror::Error;

/// Format, serialization, or container errors.
#[derive(Debug, Error)]
pub enum FormatError {
    /// File or stream I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Zip container compression/decompression error.
    #[error("Zip container error: {0}")]
    Zip(#[from] zip::result::ZipError),

    /// JSON serialization or deserialization failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// The package was authored in a newer format than this engine version understands.
    #[error(
        "Unsupported .arx format version: file is version {found}, but this engine only supports up to version {supported_max}. Please update Zenocast."
    )]
    UnsupportedVersion {
        /// Version found in manifest.
        found: u32,
        /// Maximum version supported by this build.
        supported_max: u32,
    },

    /// Required entry missing inside .arx zip container (e.g. manifest.json or scene.json).
    #[error("Missing required file inside .arx package: '{0}'")]
    MissingEntry(String),

    /// Referenced media asset not found in assets/ directory.
    #[error("Referenced asset not found in .arx container: '{0}'")]
    MissingAsset(String),

    /// Manifest structure is malformed or invalid.
    #[error("Invalid manifest: {0}")]
    InvalidManifest(String),

    /// Semantic validation error within the composition.
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
}
