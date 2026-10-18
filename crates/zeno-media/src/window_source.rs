//! Desktop top-level application window capture source.

use crate::error::SourceError;
use crate::source::{Source, SourceKind, VideoFrame};
use crate::texture_buffer::DoubleBufferedTexture;
use wgpu::{Device, Queue};
use zeno_core::SourceId;
use zeno_platform::capture_window_raw;

/// Window capture source capturing top-level application windows.
pub struct WindowCaptureSource {
    id: SourceId,
    name: String,
    hwnd: isize,
    capture_cursor: bool,
    borderless: bool,
    target_fps: f32,
    last_capture_time: f64,
    active: bool,
    prev_dot_x: Option<u32>,
    texture_buffer: DoubleBufferedTexture,
    raw_buffer: Vec<u8>,
}

impl WindowCaptureSource {
    /// Create a new window capture source.
    pub fn new(hwnd: isize, name: impl Into<String>) -> Self {
        Self {
            id: SourceId::new(),
            name: name.into(),
            hwnd,
            capture_cursor: true,
            borderless: false,
            target_fps: 60.0,
            last_capture_time: -1.0,
            active: false,
            prev_dot_x: None,
            texture_buffer: DoubleBufferedTexture::new(),
            raw_buffer: Vec::new(),
        }
    }

    /// Sets whether the mouse cursor is composited into the captured frame.
    pub fn set_capture_cursor(&mut self, enabled: bool) {
        self.capture_cursor = enabled;
    }

    /// Sets borderless capture mode.
    pub fn set_borderless(&mut self, borderless: bool) {
        self.borderless = borderless;
    }

    /// Sets the target capture framerate.
    pub fn set_target_fps(&mut self, fps: f32) {
        self.target_fps = fps.max(1.0);
    }

    /// Returns the target HWND handle.
    pub fn hwnd(&self) -> isize {
        self.hwnd
    }

    /// Automatically select the first visible desktop window, or fallback to simulation.
    pub fn default_or_fallback() -> Self {
        let windows = zeno_platform::enumerate_windows();
        if let Some(win) = windows.into_iter().find(|w| !w.is_minimized) {
            Self::new(win.hwnd, win.title)
        } else {
            Self::new(0, "Broadcast Window Capture (Simulation)")
        }
    }
}

impl Source for WindowCaptureSource {
    fn id(&self) -> SourceId {
        self.id
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn kind(&self) -> SourceKind {
        SourceKind::WindowCapture
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn dimensions(&self) -> (u32, u32) {
        self.texture_buffer.dimensions()
    }

    fn properties(&self) -> crate::source::SourceProperties {
        crate::source::SourceProperties::Window {
            window_title: self.name.clone(),
            cursor_visible: self.capture_cursor,
        }
    }

    fn start(&mut self, _device: &Device, _queue: &Queue) -> Result<(), SourceError> {
        self.active = true;
        self.last_capture_time = -1.0;
        self.prev_dot_x = None;

        // Pre-render synthetic fallback window once at startup
        let w = 1280u32;
        let h = 720u32;
        let buf_size = (w * h * 4) as usize;
        self.raw_buffer.resize(buf_size, 0);

        for y in 0..h {
            let row_offset = (y * w * 4) as usize;
            let is_header = y < 32;
            for x in 0..w {
                let px = row_offset + (x * 4) as usize;
                if is_header || x < 4 || x >= w - 4 || y >= h - 4 {
                    self.raw_buffer[px] = 39; // Surface neutral #272B30
                    self.raw_buffer[px + 1] = 43;
                    self.raw_buffer[px + 2] = 48;
                    self.raw_buffer[px + 3] = 255;
                } else {
                    self.raw_buffer[px] = 21; // Neutral surface #15171A
                    self.raw_buffer[px + 1] = 23;
                    self.raw_buffer[px + 2] = 26;
                    self.raw_buffer[px + 3] = 255;
                }
            }
        }
        Ok(())
    }

    fn stop(&mut self) {
        self.active = false;
        self.prev_dot_x = None;
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

        let mut width = 0u32;
        let mut height = 0u32;

        if self.hwnd != 0
            && capture_window_raw(
                self.hwnd,
                self.capture_cursor,
                &mut width,
                &mut height,
                &mut self.raw_buffer,
            )
            .is_ok()
            && width > 0
            && height > 0
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
        } else {
            // Fast incremental update on synthetic fallback window
            let w = 1280u32;
            let h = 720u32;
            let phase = ((current_time * 1.5) % 1.0) as f32;
            let dot_x = (phase * (w - 40) as f32) as u32 + 20;

            // Restore previous dot area
            if let Some(prev_x) = self.prev_dot_x {
                let start_x = prev_x.saturating_sub(10);
                let end_x = (prev_x + 10).min(w.saturating_sub(1));
                for y in 40..=50 {
                    let row_offset = (y * w * 4) as usize;
                    for x in start_x..=end_x {
                        let px = row_offset + (x * 4) as usize;
                        self.raw_buffer[px] = 21; // Neutral surface #15171A
                        self.raw_buffer[px + 1] = 23;
                        self.raw_buffer[px + 2] = 26;
                        self.raw_buffer[px + 3] = 255;
                    }
                }
            }

            // Draw new active dot
            let start_x = dot_x.saturating_sub(10);
            let end_x = (dot_x + 10).min(w.saturating_sub(1));
            for y in 40..=50 {
                let row_offset = (y * w * 4) as usize;
                for x in start_x..=end_x {
                    let px = row_offset + (x * 4) as usize;
                    self.raw_buffer[px] = 76; // Accent Blue #4C8DFF
                    self.raw_buffer[px + 1] = 141;
                    self.raw_buffer[px + 2] = 255;
                    self.raw_buffer[px + 3] = 255;
                }
            }
            self.prev_dot_x = Some(dot_x);

            self.texture_buffer
                .upload(device, queue, w, h, &self.raw_buffer, current_time);
            self.last_capture_time = current_time;
        }
    }
}
