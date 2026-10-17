//! Spout GPU texture sharing receiver source.
//!
//! Gated behind the `spout` feature flag.

use crate::error::SourceError;
use crate::source::{Source, SourceKind, VideoFrame};
use zeno_core::SourceId;

/// Spout shared texture receiver source for real-time inter-process GPU frame sharing.
pub struct SpoutSource {
    id: SourceId,
    name: String,
    sender_name: String,
    width: u32,
    height: u32,
    is_active: bool,
    current_frame: Option<VideoFrame>,
}

impl SpoutSource {
    /// Checks whether Spout runtime support and feature flag are active.
    pub fn is_available() -> bool {
        cfg!(feature = "spout")
    }

    /// Creates a new Spout receiver source.
    pub fn new(
        id: SourceId,
        name: impl Into<String>,
        sender_name: impl Into<String>,
    ) -> Result<Self, SourceError> {
        let name = name.into();
        let sender_name = sender_name.into();

        if !Self::is_available() {
            tracing::warn!(
                "Spout source '{}' requested but the 'spout' feature flag is disabled at build time",
                sender_name
            );
        }

        Ok(Self {
            id,
            name,
            sender_name,
            width: 1920,
            height: 1080,
            is_active: false,
            current_frame: None,
        })
    }

    /// Returns the target Spout sender name (e.g. "Resolume Arena 1").
    pub fn sender_name(&self) -> &str {
        &self.sender_name
    }
}

impl Source for SpoutSource {
    fn id(&self) -> SourceId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Spout
    }

    fn is_active(&self) -> bool {
        self.is_active
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn start(&mut self, _device: &wgpu::Device, _queue: &wgpu::Queue) -> Result<(), SourceError> {
        self.is_active = true;
        Ok(())
    }

    fn stop(&mut self) {
        self.is_active = false;
        self.current_frame = None;
    }

    fn get_frame(&mut self) -> Option<&VideoFrame> {
        self.current_frame.as_ref()
    }

    fn update(&mut self, _device: &wgpu::Device, _queue: &wgpu::Queue, _current_time: f64) {
        // Spout frame update hook
    }
}
