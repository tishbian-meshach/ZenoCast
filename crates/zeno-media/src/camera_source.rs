//! Windows Media Foundation camera and capture card input source.

use crate::error::SourceError;
use crate::source::{Source, SourceKind, VideoFrame};
use crate::texture_buffer::DoubleBufferedTexture;
use wgpu::{Device, Queue};
use zeno_core::SourceId;
use zeno_platform::{enumerate_camera_devices, CameraDeviceInfo, CameraFormat};

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
    prev_bar_x: Option<u32>,
    texture_buffer: DoubleBufferedTexture,
    pixel_buffer: Vec<u8>,
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
            prev_bar_x: None,
            texture_buffer: DoubleBufferedTexture::new(),
            pixel_buffer: Vec::new(),
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
                prev_bar_x: None,
                texture_buffer: DoubleBufferedTexture::new(),
                pixel_buffer: Vec::new(),
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
            device_name: self.name.clone(),
            format_index: 0,
            resolution: (self.format.width, self.format.height),
        }
    }

    fn update_properties(
        &mut self,
        properties: &crate::source::SourceProperties,
        _device: &Device,
        _queue: &Queue,
    ) -> Result<(), SourceError> {
        if let crate::source::SourceProperties::Camera {
            device_name,
            resolution,
            ..
        } = properties
        {
            self.name = device_name.clone();
            self.format.width = resolution.0;
            self.format.height = resolution.1;
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
        self.prev_bar_x = None;
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
                self.pixel_buffer[px] = 29 + (pattern / 16); // Dark Studio Neutral (#1D2024)
                self.pixel_buffer[px + 1] = 32 + (pattern / 16);
                self.pixel_buffer[px + 2] = 36 + (pattern / 16);
                self.pixel_buffer[px + 3] = 255;
            }
        }
        Ok(())
    }

    fn stop(&mut self) {
        self.active = false;
        self.prev_bar_x = None;
    }

    fn get_frame(&mut self) -> Option<&VideoFrame> {
        self.texture_buffer.current_frame()
    }

    fn update(&mut self, device: &Device, queue: &Queue, current_time: f64) {
        if !self.active {
            return;
        }

        let interval = 1.0 / (self.format.frame_rate as f64);
        if self.last_frame_time >= 0.0 && (current_time - self.last_frame_time) < interval {
            return;
        }

        let width = self.format.width;
        let height = self.format.height;
        let buf_size = (width * height * 4) as usize;

        if self.pixel_buffer.len() < buf_size {
            self.pixel_buffer.resize(buf_size, 0);
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
                    self.pixel_buffer[px] = 29 + (pattern / 16);
                    self.pixel_buffer[px + 1] = 32 + (pattern / 16);
                    self.pixel_buffer[px + 2] = 36 + (pattern / 16);
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
                self.pixel_buffer[px] = 76; // Accent Blue (#4C8DFF)
                self.pixel_buffer[px + 1] = 141;
                self.pixel_buffer[px + 2] = 255;
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
                        self.pixel_buffer[px] = 255; // Warning red
                        self.pixel_buffer[px + 1] = 59;
                        self.pixel_buffer[px + 2] = 48;
                        self.pixel_buffer[px + 3] = 255;
                    } else {
                        self.pixel_buffer[px] = 39; // Neutral #272B30
                        self.pixel_buffer[px + 1] = 43;
                        self.pixel_buffer[px + 2] = 48;
                        self.pixel_buffer[px + 3] = 255;
                    }
                } else {
                    self.pixel_buffer[px] = 21; // #15171A
                    self.pixel_buffer[px + 1] = 23;
                    self.pixel_buffer[px + 2] = 26;
                    self.pixel_buffer[px + 3] = 255;
                }
            }
        }
    }
}
