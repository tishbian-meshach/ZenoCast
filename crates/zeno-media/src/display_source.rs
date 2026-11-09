//! Physical display monitor capture source.

use crate::error::SourceError;
use crate::source::{Source, SourceKind, VideoFrame};
use crate::texture_buffer::DoubleBufferedTexture;
use wgpu::{Device, Queue};
use zeno_core::SourceId;
use zeno_platform::{enumerate_displays, DisplayInfo, WgcCaptureSession};

/// Physical monitor display capture source.
pub struct DisplayCaptureSource {
    id: SourceId,
    name: String,
    monitor_index: usize,
    hmonitor: isize,
    bounds: (i32, i32, u32, u32),
    capture_cursor: bool,
    target_fps: f32,
    last_capture_time: f64,
    active: bool,
    texture_buffer: DoubleBufferedTexture,
    raw_buffer: Vec<u8>,
    wgc_session: Option<WgcCaptureSession>,
    last_frame_seq: u64,
}

impl DisplayCaptureSource {
    /// Create a new display capture source from monitor info.
    pub fn new(display: &DisplayInfo) -> Self {
        Self::with_monitor(display, 0)
    }

    /// Create with specific monitor index.
    pub fn with_monitor(display: &DisplayInfo, monitor_index: usize) -> Self {
        Self {
            id: SourceId::new(),
            name: display.name.clone(),
            monitor_index,
            hmonitor: display.hmonitor,
            bounds: display.bounds,
            capture_cursor: true,
            target_fps: 60.0,
            last_capture_time: -1.0,
            active: false,
            texture_buffer: DoubleBufferedTexture::with_format(wgpu::TextureFormat::Bgra8UnormSrgb),
            raw_buffer: Vec::new(),
            wgc_session: None,
            last_frame_seq: 0,
        }
    }

    /// Set deterministic or specific source ID.
    pub fn with_id(mut self, id: SourceId) -> Self {
        self.id = id;
        self
    }

    /// Create capture source for the primary display.
    pub fn primary() -> Result<Self, SourceError> {
        let displays = enumerate_displays();
        let (idx, display) = displays
            .iter()
            .enumerate()
            .find(|(_, d)| d.is_primary)
            .ok_or_else(|| {
                SourceError::TargetNotFound("No primary display detected".to_string())
            })?;
        Ok(Self::with_monitor(display, idx))
    }

    /// Create capture source for a specific monitor index.
    pub fn from_index(index: usize) -> Result<Self, SourceError> {
        let displays = enumerate_displays();
        if displays.is_empty() {
            return Err(SourceError::TargetNotFound(
                "No displays detected".to_string(),
            ));
        }
        let (idx, display) = if index < displays.len() {
            (index, &displays[index])
        } else {
            (0, &displays[0])
        };
        Ok(Self::with_monitor(display, idx))
    }

    /// Automatically select the primary monitor, first available, or fallback to dummy 1920x1080 display.
    pub fn default_or_fallback() -> Self {
        Self::primary()
            .or_else(|_| Self::from_index(0))
            .unwrap_or_else(|_| {
                let dummy = DisplayInfo {
                    name: "Primary Display (Fallback)".to_string(),
                    bounds: (0, 0, 1920, 1080),
                    is_primary: true,
                    hmonitor: 0,
                };
                Self::with_monitor(&dummy, 0)
            })
    }

    /// Sets whether the mouse cursor is rendered in the capture output.
    pub fn set_capture_cursor(&mut self, enabled: bool) {
        if self.capture_cursor != enabled {
            self.capture_cursor = enabled;
            if self.active && self.hmonitor != 0 {
                self.wgc_session =
                    WgcCaptureSession::for_monitor(self.hmonitor, self.capture_cursor).ok();
                self.last_frame_seq = 0;
            }
        }
    }

    /// Sets the target framerate.
    pub fn set_target_fps(&mut self, fps: f32) {
        self.target_fps = fps.max(1.0);
    }

    /// Returns the native HMONITOR handle.
    pub fn hmonitor(&self) -> isize {
        self.hmonitor
    }

    /// Returns the monitor index.
    pub fn monitor_index(&self) -> usize {
        self.monitor_index
    }
}

impl Source for DisplayCaptureSource {
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
        SourceKind::DisplayCapture
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn is_healthy(&self) -> bool {
        self.active && self.bounds.2 > 0 && self.bounds.3 > 0
    }

    fn dimensions(&self) -> (u32, u32) {
        let (tb_w, tb_h) = self.texture_buffer.dimensions();
        if tb_w > 0 && tb_h > 0 {
            (tb_w, tb_h)
        } else {
            (self.bounds.2, self.bounds.3)
        }
    }

    fn properties(&self) -> crate::source::SourceProperties {
        crate::source::SourceProperties::Display {
            monitor_index: self.monitor_index,
            cursor_visible: self.capture_cursor,
        }
    }

    fn update_properties(
        &mut self,
        properties: &crate::source::SourceProperties,
        _device: &Device,
        _queue: &Queue,
    ) -> Result<(), SourceError> {
        if let crate::source::SourceProperties::Display {
            monitor_index,
            cursor_visible,
        } = properties
        {
            let cursor_changed = self.capture_cursor != *cursor_visible;
            self.capture_cursor = *cursor_visible;

            let monitor_changed = self.monitor_index != *monitor_index;
            if monitor_changed {
                let displays = enumerate_displays();
                if let Some(disp) = displays.get(*monitor_index) {
                    let old_hmonitor = self.hmonitor;
                    self.monitor_index = *monitor_index;
                    self.hmonitor = disp.hmonitor;
                    self.bounds = disp.bounds;
                    self.name = disp.name.clone();

                    if self.active
                        && (old_hmonitor != self.hmonitor
                            || cursor_changed
                            || self.wgc_session.is_none())
                    {
                        self.wgc_session =
                            WgcCaptureSession::for_monitor(self.hmonitor, self.capture_cursor).ok();
                        self.last_frame_seq = 0;
                        self.last_capture_time = -1.0;
                    }
                }
            } else if cursor_changed && self.active && self.hmonitor != 0 {
                self.wgc_session =
                    WgcCaptureSession::for_monitor(self.hmonitor, self.capture_cursor).ok();
                self.last_frame_seq = 0;
                self.last_capture_time = -1.0;
            }
            Ok(())
        } else {
            Err(SourceError::InvalidProperties(
                "Expected Display properties".to_string(),
            ))
        }
    }

    fn start(&mut self, _device: &Device, _queue: &Queue) -> Result<(), SourceError> {
        self.active = true;
        self.last_capture_time = -1.0;
        self.last_frame_seq = 0;

        // Initialize hardware-accelerated Windows Graphics Capture (WGC) for target HMONITOR
        if self.hmonitor != 0 {
            match WgcCaptureSession::for_monitor(self.hmonitor, self.capture_cursor) {
                Ok(session) => {
                    tracing::info!(
                        "[DisplayCapture] Initialized hardware WGC capture for monitor index {} (HMONITOR 0x{:x})",
                        self.monitor_index,
                        self.hmonitor
                    );
                    self.wgc_session = Some(session);
                }
                Err(e) => {
                    tracing::warn!(
                        "[DisplayCapture] WGC initialization failed for HMONITOR 0x{:x} (falling back to GDI): {}",
                        self.hmonitor,
                        e
                    );
                    self.wgc_session = None;
                }
            }
        } else {
            self.wgc_session = None;
        }

        Ok(())
    }

    fn stop(&mut self) {
        self.active = false;
        self.wgc_session = None;
    }

    fn get_frame(&mut self) -> Option<&VideoFrame> {
        self.texture_buffer.current_frame()
    }

    fn update(&mut self, device: &Device, queue: &Queue, current_time: f64) {
        if !self.active {
            return;
        }

        // 1. Check for hardware-accelerated WGC frame first
        if let Some(ref session) = self.wgc_session {
            if let Some(frame) = session.get_latest_frame() {
                if frame.sequence != self.last_frame_seq && frame.width > 0 && frame.height > 0 {
                    let t_upload_start = std::time::Instant::now();
                    self.last_frame_seq = frame.sequence;

                    let capture_latency_ms =
                        (current_time - frame.timestamp_seconds).max(0.0) * 1000.0;

                    self.texture_buffer.upload(
                        device,
                        queue,
                        frame.width,
                        frame.height,
                        &frame.rgba,
                        current_time,
                    );
                    self.last_capture_time = current_time;

                    let upload_duration_ms = t_upload_start.elapsed().as_secs_f64() * 1000.0;
                    tracing::trace!(
                        seq = frame.sequence,
                        capture_latency_ms,
                        upload_duration_ms,
                        "[DisplayCapture] Uploaded new WGC frame to texture"
                    );
                    return;
                } else if frame.sequence == self.last_frame_seq {
                    // Frame hasn't changed; avoid re-uploading identical buffer and burning GPU queue
                    return;
                }
            }
        }

        // 2. Fallback to GDI capture if WGC is not active or hasn't produced frames
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
