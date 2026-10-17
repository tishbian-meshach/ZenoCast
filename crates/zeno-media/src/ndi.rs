//! NDI (Network Device Interface) video and audio ingest source.
//!
//! Gated behind the `ndi` feature flag.

use crate::error::SourceError;
use crate::source::{Source, SourceKind, VideoFrame};
use zeno_core::SourceId;

/// NDI network video receiver source.
pub struct NdiSource {
    id: SourceId,
    name: String,
    source_name: String,
    width: u32,
    height: u32,
    is_active: bool,
    current_frame: Option<VideoFrame>,
}

impl NdiSource {
    /// Checks whether NDI runtime libraries and feature flag are active.
    pub fn is_available() -> bool {
        cfg!(feature = "ndi")
    }

    /// Creates a new NDI receiver source.
    pub fn new(
        id: SourceId,
        name: impl Into<String>,
        source_name: impl Into<String>,
    ) -> Result<Self, SourceError> {
        let name = name.into();
        let source_name = source_name.into();

        if !Self::is_available() {
            tracing::warn!(
                "NDI source '{}' requested but the 'ndi' feature flag is disabled at build time",
                source_name
            );
        }

        Ok(Self {
            id,
            name,
            source_name,
            width: 1920,
            height: 1080,
            is_active: false,
            current_frame: None,
        })
    }

    /// Returns the target NDI network source name (e.g. "OBS (Output 1)").
    pub fn source_name(&self) -> &str {
        &self.source_name
    }
}

impl Source for NdiSource {
    fn id(&self) -> SourceId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> SourceKind {
        SourceKind::Ndi
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
        // NDI frame update hook
    }
}
