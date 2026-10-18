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
    target_found: bool,
    offline_rendered: bool,
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
            target_found: hwnd != 0,
            offline_rendered: false,
            texture_buffer: DoubleBufferedTexture::new(),
            raw_buffer: Vec::new(),
        }
    }

    /// Set explicit source ID.
    pub fn with_id(mut self, id: SourceId) -> Self {
        self.id = id;
        self
    }

    /// Automatically find window by title from active top-level windows.
    pub fn from_title(title: &str) -> Self {
        let windows = zeno_platform::enumerate_windows();
        if let Some(win) = windows.into_iter().find(|w| w.title == title) {
            Self::new(win.hwnd, win.title)
        } else {
            let mut src = Self::new(0, title);
            src.target_found = false;
            src
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

    fn set_name(&mut self, name: String) {
        self.name = name;
    }

    fn kind(&self) -> SourceKind {
        SourceKind::WindowCapture
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn is_healthy(&self) -> bool {
        self.active && self.target_found
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

    fn update_properties(
        &mut self,
        properties: &crate::source::SourceProperties,
        _device: &Device,
        _queue: &Queue,
    ) -> Result<(), SourceError> {
        if let crate::source::SourceProperties::Window {
            window_title,
            cursor_visible,
        } = properties
        {
            self.capture_cursor = *cursor_visible;
            if &self.name != window_title {
                self.name = window_title.clone();
                let windows = zeno_platform::enumerate_windows();
                if let Some(win) = windows.into_iter().find(|w| &w.title == window_title) {
                    self.hwnd = win.hwnd;
                    self.target_found = true;
                } else {
                    self.hwnd = 0;
                    self.target_found = false;
                }
            }
            Ok(())
        } else {
            Err(SourceError::InvalidProperties(
                "Expected Window properties".to_string(),
            ))
        }
    }

    fn start(&mut self, _device: &Device, _queue: &Queue) -> Result<(), SourceError> {
        self.active = true;
        self.last_capture_time = -1.0;

        // Pre-render synthetic fallback window once at startup
        let w = 1280u32;
        let h = 720u32;
        self.render_offline_placeholder(w, h);
        self.offline_rendered = true;
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
            self.target_found = true;
            self.offline_rendered = false;
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
            self.target_found = false;
            // Clear broadcast placeholder: Window Closed / Target Offline
            let w = 1280u32;
            let h = 720u32;
            let buf_size = (w * h * 4) as usize;
            if self.raw_buffer.len() < buf_size {
                self.raw_buffer.resize(buf_size, 0);
                self.offline_rendered = false;
            }

            if !self.offline_rendered {
                self.render_offline_placeholder(w, h);
                self.offline_rendered = true;
            }

            self.texture_buffer
                .upload(device, queue, w, h, &self.raw_buffer, current_time);
            self.last_capture_time = current_time;
        }
    }
}

impl WindowCaptureSource {
    fn render_offline_placeholder(&mut self, w: u32, h: u32) {
        let buf_size = (w * h * 4) as usize;
        if self.raw_buffer.len() < buf_size {
            self.raw_buffer.resize(buf_size, 0);
        }

        for y in 0..h {
            let row_offset = (y * w * 4) as usize;
            let is_header = y < 36;
            let is_border = x_is_border(y, h);
            let in_offline_banner = (320..=400).contains(&y);

            for x in 0..w {
                let px = row_offset + (x * 4) as usize;
                if is_header || is_border || x < 4 || x >= w - 4 {
                    self.raw_buffer[px] = 39; // Neutral surface #272B30
                    self.raw_buffer[px + 1] = 43;
                    self.raw_buffer[px + 2] = 48;
                    self.raw_buffer[px + 3] = 255;
                } else if in_offline_banner && (440..=840).contains(&x) {
                    // Offline notice banner with warning highlight
                    let is_banner_edge = y == 320 || y == 400 || x == 440 || x == 840;
                    if is_banner_edge {
                        self.raw_buffer[px] = 255; // Warning red/accent
                        self.raw_buffer[px + 1] = 59;
                        self.raw_buffer[px + 2] = 48;
                        self.raw_buffer[px + 3] = 255;
                    } else {
                        self.raw_buffer[px] = 39; // #272B30
                        self.raw_buffer[px + 1] = 43;
                        self.raw_buffer[px + 2] = 48;
                        self.raw_buffer[px + 3] = 255;
                    }
                } else {
                    // Background neutral canvas #15171A
                    self.raw_buffer[px] = 21;
                    self.raw_buffer[px + 1] = 23;
                    self.raw_buffer[px + 2] = 26;
                    self.raw_buffer[px + 3] = 255;
                }
            }
        }
    }
}

#[inline]
fn x_is_border(y: u32, h: u32) -> bool {
    y == 36 || y >= h - 4
}
