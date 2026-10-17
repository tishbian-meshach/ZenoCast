//! Package manifest describing container metadata and engine compatibility.

use serde::{Deserialize, Serialize};

/// Current `.arx` container format version.
pub const CURRENT_FORMAT_VERSION: u32 = 1;

/// Current engine minimum required version string.
pub const CURRENT_ENGINE_VERSION: &str = "0.1.0";

/// Metadata manifest stored as `manifest.json` at the root of every `.arx` container.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    /// Schema format version integer.
    pub format_version: u32,
    /// Template or overlay title.
    pub name: String,
    /// Author / creator name.
    pub author: String,
    /// Composition canvas width in pixels.
    pub width: u32,
    /// Composition canvas height in pixels.
    pub height: u32,
    /// Frame rate in frames per second.
    pub fps: f32,
    /// Minimum Zenocast engine version required to load this package.
    pub engine_min_version: String,
}

impl Default for Manifest {
    fn default() -> Self {
        Self {
            format_version: CURRENT_FORMAT_VERSION,
            name: "Untitled Overlay".to_string(),
            author: "Zenocast User".to_string(),
            width: 1920,
            height: 1080,
            fps: 60.0,
            engine_min_version: CURRENT_ENGINE_VERSION.to_string(),
        }
    }
}

impl Manifest {
    /// Construct a new manifest with current format version.
    pub fn new(
        name: impl Into<String>,
        author: impl Into<String>,
        width: u32,
        height: u32,
        fps: f32,
    ) -> Self {
        Self {
            format_version: CURRENT_FORMAT_VERSION,
            name: name.into(),
            author: author.into(),
            width,
            height,
            fps,
            engine_min_version: CURRENT_ENGINE_VERSION.to_string(),
        }
    }
}
