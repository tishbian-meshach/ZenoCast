//! Base Source trait and video frame definitions.

use crate::error::SourceError;
use std::sync::Arc;
use wgpu::{Device, Queue, Texture, TextureView};
use zeno_core::SourceId;

/// Classification of live video input sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceKind {
    /// Full physical desktop monitor capture.
    DisplayCapture,
    /// Desktop top-level application window capture.
    WindowCapture,
    /// USB UVC webcam or PCIe capture card video feed.
    CameraCapture,
    /// Looping or one-shot video/image file player.
    MediaFile,
    /// Solid flat or gradient color generator.
    Color,
    /// Off-screen embedded web browser surface.
    Browser,
    /// NDI network video stream.
    Ndi,
    /// Spout shared GPU texture.
    Spout,
}

/// A captured video frame backed by an immutable GPU texture with a presentation timestamp.
#[derive(Clone)]
pub struct VideoFrame {
    /// Frame horizontal resolution in pixels.
    pub width: u32,
    /// Frame vertical resolution in pixels.
    pub height: u32,
    /// Monotonic capture timestamp in seconds.
    pub timestamp_seconds: f64,
    /// Dedicated GPU texture holding the frame pixels (Rgba8UnormSrgb).
    pub texture: Arc<Texture>,
    /// GPU texture view ready for immediate pipeline binding.
    pub texture_view: Arc<TextureView>,
}

/// Trait implemented by all visual inputs in Zenocast.
pub trait Source: Send + Sync {
    /// Unique identifier for this source instance.
    fn id(&self) -> SourceId;

    /// User-visible source name.
    fn name(&self) -> &str;

    /// Categorical kind of this source.
    fn kind(&self) -> SourceKind;

    /// Returns `true` if the source is actively capturing.
    fn is_active(&self) -> bool;

    /// Current resolution of the source in pixels (width, height).
    fn dimensions(&self) -> (u32, u32);

    /// Initializes hardware handles and allocates GPU textures for frame streaming.
    fn start(&mut self, device: &Device, queue: &Queue) -> Result<(), SourceError>;

    /// Stops capture and releases native Windows/GPU handles.
    fn stop(&mut self);

    /// Retrieves the most recent frame ready for compositor rendering.
    ///
    /// Real-time guarantee: this method executes in O(1) time without blocking locks,
    /// file IO, or heap allocations.
    fn get_frame(&mut self) -> Option<&VideoFrame>;

    /// Ticks the capture source, updating frame buffers if a new frame is ready.
    fn update(&mut self, device: &Device, queue: &Queue, current_time: f64);
}
