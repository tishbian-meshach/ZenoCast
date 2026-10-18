//! Base Source trait and video frame definitions.

use crate::error::SourceError;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use wgpu::{Device, Queue, Texture, TextureView};
use zeno_core::SourceId;

/// Classification of live video and audio input sources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SourceKind {
    /// Full physical desktop monitor capture.
    DisplayCapture,
    /// Desktop top-level application window capture.
    WindowCapture,
    /// USB UVC webcam or PCIe capture card video feed.
    CameraCapture,
    /// Looping or one-shot video file player.
    MediaFile,
    /// Static image file (PNG, JPG).
    Image,
    /// Solid flat color generator.
    Color,
    /// Text element or data-bound text overlay.
    Text,
    /// Audio input microphone or line-in.
    AudioInput,
    /// Audio output desktop loopback.
    AudioOutput,
    /// Off-screen embedded web browser surface.
    Browser,
    /// NDI network video stream.
    Ndi,
    /// Spout shared GPU texture.
    Spout,
}

/// Typed, source-specific settings for configuration and live editing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SourceProperties {
    Display {
        monitor_index: usize,
        cursor_visible: bool,
    },
    Window {
        window_title: String,
        cursor_visible: bool,
    },
    Color {
        color: [f32; 4],
        width: u32,
        height: u32,
    },
    Text {
        text: String,
        font_family: String,
        font_size: f32,
        color: [f32; 4],
    },
    Image {
        path: String,
        scale_mode: String,
    },
    Video {
        path: String,
        looping: bool,
    },
    Camera {
        device_name: String,
        format_index: usize,
        resolution: (u32, u32),
    },
    AudioInput {
        device_name: String,
        volume: f32,
        muted: bool,
    },
    AudioOutput {
        device_name: String,
        volume: f32,
        muted: bool,
    },
    Browser {
        url: String,
        width: u32,
        height: u32,
        custom_css: String,
    },
    Custom {
        kind: String,
        data: String,
    },
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

    /// Sets or renames the user-visible source name.
    fn set_name(&mut self, _name: String) {}

    /// Categorical kind of this source.
    fn kind(&self) -> SourceKind;

    /// Returns `true` if the source is actively capturing.
    fn is_active(&self) -> bool;

    /// Returns `true` if the source is currently healthy and functioning.
    fn is_healthy(&self) -> bool {
        self.is_active()
    }

    /// Current resolution of the source in pixels (width, height).
    fn dimensions(&self) -> (u32, u32);

    /// Typed properties representing current configuration.
    fn properties(&self) -> SourceProperties;

    /// Updates configuration live during runtime.
    fn update_properties(
        &mut self,
        _props: &SourceProperties,
        _device: &Device,
        _queue: &Queue,
    ) -> Result<(), SourceError> {
        Ok(())
    }

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
