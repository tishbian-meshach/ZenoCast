//! Windows Media Foundation camera and capture card input source.

use crate::error::SourceError;
use crate::source::{Source, SourceKind, VideoFrame};
use crate::texture_buffer::DoubleBufferedTexture;
use wgpu::{Device, Queue};
use zeno_core::SourceId;
use zeno_platform::{enumerate_camera_devices, CameraDeviceInfo, CameraFormat, MfCameraSession};

/// Live webcam or capture card video source.
pub struct CameraCaptureSource {
    id: SourceId,
    name: String,
    device_id: String,
    format: CameraFormat,
    active: bool,
    healthy: bool,
    offline_rendered: bool,
    last_frame_time: f64,
    last_frame_seq: u64,
    prev_bar_x: Option<u32>,
    texture_buffer: DoubleBufferedTexture,
    pixel_buffer: Vec<u8>,
    mf_session: Option<MfCameraSession>,
}

impl CameraCaptureSource {
    /// Create a new camera capture source.
    pub fn new(device_info: &CameraDeviceInfo, format: Option<CameraFormat>) -> Self {
        let fmt = format.unwrap_or_else(|| {
            device_info
                .formats
                .first()
                .cloned()
                .unwrap_or(CameraFormat {
                    width: 1920,
                    height: 1080,
                    frame_rate: 60.0,
                    format_name: "NV12".to_string(),
                })
        });

        Self {
            id: SourceId::new(),
            name: device_info.name.clone(),
            device_id: device_info.id.clone(),
            format: fmt,
            active: false,
            healthy: true,
            offline_rendered: false,
            last_frame_time: -1.0,
            last_frame_seq: 0,
            prev_bar_x: None,
            texture_buffer: DoubleBufferedTexture::with_format(wgpu::TextureFormat::Bgra8UnormSrgb),
            pixel_buffer: Vec::new(),
            mf_session: None,
        }
    }

    /// Set explicit source ID.
    pub fn with_id(mut self, id: SourceId) -> Self {
        self.id = id;
        self
    }

    /// Finds camera by name or ID, or falls back to simulation.
    pub fn from_name_or_id(name_or_id: &str) -> Self {
        let cameras = enumerate_camera_devices();
        if let Some(cam) = cameras
            .into_iter()
            .find(|c| c.name == name_or_id || c.id == name_or_id)
        {
            Self::new(&cam, None)
        } else {
            Self::default_or_fallback()
        }
    }

    /// Automatically select the default or primary camera device, with test-feed fallback.
    pub fn default_or_fallback() -> Self {
        let cameras = enumerate_camera_devices();
        if let Some(cam) = cameras.into_iter().next() {
            Self::new(&cam, None)
        } else {
            // Broadcast camera fallback pattern
            Self {
                id: SourceId::new(),
                name: "Virtual Broadcast Camera (Simulation)".to_string(),
                device_id: "zeno-virtual-cam-0".to_string(),
                format: CameraFormat {
                    width: 1920,
                    height: 1080,
                    frame_rate: 60.0,
                    format_name: "RGBA".to_string(),
                },
                active: false,
                healthy: true,
                offline_rendered: false,
                last_frame_time: -1.0,
                last_frame_seq: 0,
                prev_bar_x: None,
                texture_buffer: DoubleBufferedTexture::with_format(
                    wgpu::TextureFormat::Bgra8UnormSrgb,
                ),
                pixel_buffer: Vec::new(),
                mf_session: None,
            }
        }
    }

    /// Sets whether the camera device is healthy/connected.
    pub fn set_healthy(&mut self, healthy: bool) {
        if self.healthy != healthy {
            self.healthy = healthy;
            self.offline_rendered = false;
        }
    }

    /// Returns the active format configuration.
    pub fn format(&self) -> &CameraFormat {
        &self.format
    }

    /// Returns the camera device identifier.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }
}

impl Source for CameraCaptureSource {
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
        SourceKind::CameraCapture
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn is_healthy(&self) -> bool {
        self.active && self.healthy
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.format.width, self.format.height)
    }

    fn properties(&self) -> crate::source::SourceProperties {
        crate::source::SourceProperties::Camera {
            device_name: if self.device_id.is_empty() {
                self.name.clone()
            } else {
                self.device_id.clone()
            },
            format_index: 0,
            resolution: (self.format.width, self.format.height),
        }
    }

    fn update_properties(
        &mut self,
        properties: &crate::source::SourceProperties,
        device: &Device,
        queue: &Queue,
    ) -> Result<(), SourceError> {
        if let crate::source::SourceProperties::Camera {
            device_name,
            resolution,
            ..
        } = properties
        {
            let id_changed = &self.device_id != device_name && &self.name != device_name;
            self.format.width = resolution.0;
            self.format.height = resolution.1;

            if id_changed || self.mf_session.is_none() {
                let cameras = enumerate_camera_devices();
                if let Some(cam) = cameras
                    .into_iter()
                    .find(|c| &c.name == device_name || &c.id == device_name)
                {
                    self.device_id = cam.id;
                    self.name = cam.name;
                } else {
                    self.device_id = device_name.clone();
                }
                if self.active {
                    let _ = self.start(device, queue);
                }
            }
            Ok(())
        } else {
            Err(SourceError::InvalidProperties(
                "Expected Camera properties".to_string(),
            ))
        }
    }

    fn start(&mut self, _device: &Device, _queue: &Queue) -> Result<(), SourceError> {
        self.active = true;
        self.last_frame_time = -1.0;
        self.last_frame_seq = 0;
        self.prev_bar_x = None;
        self.offline_rendered = false;

        let width = self.format.width;
        let height = self.format.height;
        let buf_size = (width * height * 4) as usize;
        self.pixel_buffer.resize(buf_size, 0);

        // Pre-render base background canvas once at startup to avoid per-frame CPU iteration
        for y in 0..height {
            let row_offset = (y * width * 4) as usize;
            for x in 0..width {
                let px = row_offset + (x * 4) as usize;
                let pattern = ((x ^ y) & 0xFF) as u8;
                self.pixel_buffer[px] = 36 + (pattern / 16); // Blue
                self.pixel_buffer[px + 1] = 32 + (pattern / 16); // Green
                self.pixel_buffer[px + 2] = 29 + (pattern / 16); // Red
                self.pixel_buffer[px + 3] = 255;
            }
        }

        // Initialize hardware Media Foundation camera session if targeting a hardware camera
        if self.device_id != "zeno-virtual-cam-0" && !self.device_id.is_empty() {
            match MfCameraSession::new(
                &self.device_id,
                self.format.width,
                self.format.height,
                &self.format.format_name,
            ) {
                Ok(session) => {
                    self.mf_session = Some(session);
                    self.healthy = true;
                    tracing::info!(
                        "[CameraCaptureSource] Started MfCameraSession for device '{}'",
                        self.name
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "[CameraCaptureSource] MfCameraSession initialization failed for '{}': {}",
                        self.name,
                        e
                    );
                    self.mf_session = None;
                }
            }
        } else {
            self.mf_session = None;
        }

        Ok(())
    }

    fn stop(&mut self) {
        self.active = false;
        self.prev_bar_x = None;
        if let Some(mut session) = self.mf_session.take() {
            session.stop();
        }
    }

    fn get_frame(&mut self) -> Option<&VideoFrame> {
        self.texture_buffer.current_frame()
    }

    fn update(&mut self, device: &Device, queue: &Queue, current_time: f64) {
        if !self.active {
            return;
        }

        let width = self.format.width;
        let height = self.format.height;
        let buf_size = (width * height * 4) as usize;

        if self.pixel_buffer.len() < buf_size {
            self.pixel_buffer.resize(buf_size, 0);
        }

        // 1. Process hardware Media Foundation frames
        if let Some(ref session) = self.mf_session {
            if session.is_permission_denied() {
                self.healthy = false;
                if !self.offline_rendered {
                    self.render_privacy_blocked_placeholder(width, height);
                    self.offline_rendered = true;
                }
                self.texture_buffer.upload(
                    device,
                    queue,
                    width,
                    height,
                    &self.pixel_buffer,
                    current_time,
                );
                self.last_frame_time = current_time;
                return;
            }

            if let Some(frame) = session.get_latest_frame() {
                if frame.sequence != self.last_frame_seq && frame.width > 0 && frame.height > 0 {
                    self.last_frame_seq = frame.sequence;
                    self.healthy = true;
                    self.offline_rendered = false;
                    self.texture_buffer.upload(
                        device,
                        queue,
                        frame.width,
                        frame.height,
                        &frame.rgba,
                        current_time,
                    );
                    self.last_frame_time = current_time;
                    return;
                } else if frame.sequence == self.last_frame_seq {
                    // Frame hasn't changed; avoid re-uploading identical texture
                    return;
                }
            }
        }

        // 2. Fallback to simulation/offline placeholder if hardware stream is not yet delivering frames
        let interval = 1.0 / (self.format.frame_rate as f64);
        if self.last_frame_time >= 0.0 && (current_time - self.last_frame_time) < interval {
            return;
        }

        if !self.healthy {
            if !self.offline_rendered {
                self.render_offline_placeholder(width, height);
                self.offline_rendered = true;
            }
            self.texture_buffer.upload(
                device,
                queue,
                width,
                height,
                &self.pixel_buffer,
                current_time,
            );
            self.last_frame_time = current_time;
            return;
        }

        self.offline_rendered = false;

        // Render high-framerate dynamic camera feed simulation incrementally
        let phase = ((current_time * 2.0) % 1.0) as f32;
        let bar_x = (phase * width as f32) as u32;

        // Restore previous bar columns to avoid full-buffer rasterization
        if let Some(prev_x) = self.prev_bar_x {
            let start_x = prev_x.saturating_sub(20);
            let end_x = (prev_x + 20).min(width.saturating_sub(1));
            for y in 0..height {
                let row_offset = (y * width * 4) as usize;
                for x in start_x..=end_x {
                    let px = row_offset + (x * 4) as usize;
                    let pattern = ((x ^ y) & 0xFF) as u8;
                    self.pixel_buffer[px] = 36 + (pattern / 16);
                    self.pixel_buffer[px + 1] = 32 + (pattern / 16);
                    self.pixel_buffer[px + 2] = 29 + (pattern / 16);
                    self.pixel_buffer[px + 3] = 255;
                }
            }
        }

        // Draw active accent indicator
        let start_x = bar_x.saturating_sub(20);
        let end_x = (bar_x + 20).min(width.saturating_sub(1));
        for y in 0..height {
            let row_offset = (y * width * 4) as usize;
            for x in start_x..=end_x {
                let px = row_offset + (x * 4) as usize;
                self.pixel_buffer[px] = 255; // Accent Blue (#4C8DFF in BGRA)
                self.pixel_buffer[px + 1] = 141;
                self.pixel_buffer[px + 2] = 76;
                self.pixel_buffer[px + 3] = 255;
            }
        }
        self.prev_bar_x = Some(bar_x);

        self.texture_buffer.upload(
            device,
            queue,
            width,
            height,
            &self.pixel_buffer,
            current_time,
        );
        self.last_frame_time = current_time;
    }
}

impl CameraCaptureSource {
    fn render_offline_placeholder(&mut self, width: u32, height: u32) {
        let buf_size = (width * height * 4) as usize;
        if self.pixel_buffer.len() < buf_size {
            self.pixel_buffer.resize(buf_size, 0);
        }

        for y in 0..height {
            let row_offset = (y * width * 4) as usize;
            let in_banner = y >= height / 2 - 40 && y <= height / 2 + 40;
            for x in 0..width {
                let px = row_offset + (x * 4) as usize;
                if in_banner && x >= width / 4 && x <= 3 * width / 4 {
                    let is_edge = y == height / 2 - 40
                        || y == height / 2 + 40
                        || x == width / 4
                        || x == 3 * width / 4;
                    if is_edge {
                        self.pixel_buffer[px] = 48; // Red (#FF3B30 in BGRA)
                        self.pixel_buffer[px + 1] = 59;
                        self.pixel_buffer[px + 2] = 255;
                        self.pixel_buffer[px + 3] = 255;
                    } else {
                        self.pixel_buffer[px] = 48; // Neutral #272B30
                        self.pixel_buffer[px + 1] = 43;
                        self.pixel_buffer[px + 2] = 39;
                        self.pixel_buffer[px + 3] = 255;
                    }
                } else {
                    self.pixel_buffer[px] = 26; // #15171A
                    self.pixel_buffer[px + 1] = 23;
                    self.pixel_buffer[px + 2] = 21;
                    self.pixel_buffer[px + 3] = 255;
                }
            }
        }
    }

    fn render_privacy_blocked_placeholder(&mut self, width: u32, height: u32) {
        let buf_size = (width * height * 4) as usize;
        if self.pixel_buffer.len() < buf_size {
            self.pixel_buffer.resize(buf_size, 0);
        }

        for y in 0..height {
            let row_offset = (y * width * 4) as usize;
            let in_banner = y >= height / 2 - 60 && y <= height / 2 + 60;
            for x in 0..width {
                let px = row_offset + (x * 4) as usize;
                if in_banner && x >= width / 6 && x <= 5 * width / 6 {
                    let is_edge = y == height / 2 - 60
                        || y == height / 2 + 60
                        || x == width / 6
                        || x == 5 * width / 6;
                    if is_edge {
                        self.pixel_buffer[px] = 0; // Amber Alert (#FFA500 in BGRA)
                        self.pixel_buffer[px + 1] = 165;
                        self.pixel_buffer[px + 2] = 255;
                        self.pixel_buffer[px + 3] = 255;
                    } else {
                        self.pixel_buffer[px] = 40; // Dark Amber Notice
                        self.pixel_buffer[px + 1] = 30;
                        self.pixel_buffer[px + 2] = 50;
                        self.pixel_buffer[px + 3] = 255;
                    }
                } else {
                    self.pixel_buffer[px] = 26; // #15171A
                    self.pixel_buffer[px + 1] = 23;
                    self.pixel_buffer[px + 2] = 21;
                    self.pixel_buffer[px + 3] = 255;
                }
            }
        }
    }
}
