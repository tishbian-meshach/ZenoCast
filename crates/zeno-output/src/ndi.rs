//! NDI broadcast output sink.
//!
//! Gated behind the `ndi` feature flag.

use crate::error::OutputResult;

/// NDI network video/audio broadcast output sink.
pub struct NdiOutput {
    stream_name: String,
    width: u32,
    height: u32,
    fps: u32,
    is_active: bool,
}

impl NdiOutput {
    /// Checks whether NDI output support is enabled at build time.
    pub fn is_available() -> bool {
        cfg!(feature = "ndi")
    }

    /// Creates a new NDI broadcast sender.
    pub fn new(
        stream_name: impl Into<String>,
        width: u32,
        height: u32,
        fps: u32,
    ) -> OutputResult<Self> {
        let stream_name = stream_name.into();
        if !Self::is_available() {
            tracing::warn!(
                "NDI output '{}' initialized but the 'ndi' feature flag is disabled at compile time",
                stream_name
            );
        }

        Ok(Self {
            stream_name,
            width,
            height,
            fps,
            is_active: false,
        })
    }

    /// Starts NDI broadcast streaming.
    pub fn start(&mut self) -> OutputResult<()> {
        self.is_active = true;
        Ok(())
    }

    /// Stops NDI broadcast streaming.
    pub fn stop(&mut self) {
        self.is_active = false;
    }

    /// Whether this NDI output is currently active.
    pub fn is_active(&self) -> bool {
        self.is_active
    }

    /// Target stream name.
    pub fn stream_name(&self) -> &str {
        &self.stream_name
    }

    /// Output video dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Output framerate.
    pub fn fps(&self) -> u32 {
        self.fps
    }
}
