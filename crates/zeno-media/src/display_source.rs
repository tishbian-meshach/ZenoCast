//! Physical display monitor capture source.

use crate::error::SourceError;
use crate::source::{Source, SourceKind, VideoFrame};
use crate::texture_buffer::DoubleBufferedTexture;
use wgpu::{Device, Queue};
use zeno_core::SourceId;
use zeno_platform::{enumerate_displays, DisplayInfo};

/// Physical monitor display capture source.
pub struct DisplayCaptureSource {
    id: SourceId,
    name: String,
    hmonitor: isize,
    bounds: (i32, i32, u32, u32),
    capture_cursor: bool,
    target_fps: f32,
    last_capture_time: f64,
    active: bool,
    texture_buffer: DoubleBufferedTexture,
    raw_buffer: Vec<u8>,
}

impl DisplayCaptureSource {
    /// Create a new display capture source from monitor info.
    pub fn new(display: &DisplayInfo) -> Self {
        Self {
            id: SourceId::new(),
            name: display.name.clone(),
            hmonitor: display.hmonitor,
            bounds: display.bounds,
            capture_cursor: true,
            target_fps: 60.0,
            last_capture_time: -1.0,
            active: false,
            texture_buffer: DoubleBufferedTexture::new(),
            raw_buffer: Vec::new(),
        }
    }

    /// Create capture source for the primary display.
    pub fn primary() -> Result<Self, SourceError> {
        let displays = enumerate_displays();
        let display = displays.into_iter().find(|d| d.is_primary).ok_or_else(|| {
            SourceError::TargetNotFound("No primary display detected".to_string())
        })?;
        Ok(Self::new(&display))
    }

    /// Sets whether the mouse cursor is rendered in the capture output.
    pub fn set_capture_cursor(&mut self, enabled: bool) {
        self.capture_cursor = enabled;
    }

    /// Sets the target framerate.
    pub fn set_target_fps(&mut self, fps: f32) {
        self.target_fps = fps.max(1.0);
    }

    /// Returns the native HMONITOR handle.
    pub fn hmonitor(&self) -> isize {
        self.hmonitor
    }
}

impl Source for DisplayCaptureSource {
    fn id(&self) -> SourceId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> SourceKind {
        SourceKind::DisplayCapture
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.bounds.2, self.bounds.3)
    }

    fn start(&mut self, _device: &Device, _queue: &Queue) -> Result<(), SourceError> {
        self.active = true;
        self.last_capture_time = -1.0;
        Ok(())
    }

    fn stop(&mut self) {
        self.active = false;
    }

    fn get_frame(&mut self) -> Option<&VideoFrame> {
        self.texture_buffer.current_frame()
    }

    fn update(&mut self, device: &Device, queue: &Queue, current_time: f64) {
        if !self.active {
            return;
        }

        let interval = 1.0 / (self.target_fps as f64);
        if self.last_capture_time >= 0.0 && (current_time - self.last_capture_time) < interval {
            return;
        }

        let (x, y, width, height) = self.bounds;
        if zeno_platform::capture_display_raw(
            x,
            y,
            width,
            height,
            self.capture_cursor,
            &mut self.raw_buffer,
        )
        .is_ok()
        {
            self.texture_buffer.upload(
                device,
                queue,
                width,
                height,
                &self.raw_buffer,
                current_time,
            );
            self.last_capture_time = current_time;
        }
    }
}
