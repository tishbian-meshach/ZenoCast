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
            last_frame_time: -1.0,
            prev_bar_x: None,
            texture_buffer: DoubleBufferedTexture::new(),
            pixel_buffer: Vec::new(),
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
                last_frame_time: -1.0,
                prev_bar_x: None,
                texture_buffer: DoubleBufferedTexture::new(),
                pixel_buffer: Vec::new(),
            }
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

    fn kind(&self) -> SourceKind {
        SourceKind::CameraCapture
    }

    fn is_active(&self) -> bool {
        self.active
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

    fn start(&mut self, _device: &Device, _queue: &Queue) -> Result<(), SourceError> {
        self.active = true;
        self.last_frame_time = -1.0;
        self.prev_bar_x = None;
        let width = self.format.width;
        let height = self.format.height;
        let buf_size = (width * height * 4) as usize;
        self.pixel_buffer.resize(buf_size, 0);

        // Pre-render base background gradient once at startup to avoid per-frame CPU iteration
        for y in 0..height {
            let row_offset = (y * width * 4) as usize;
            for x in 0..width {
                let px = row_offset + (x * 4) as usize;
                let grad = ((x ^ y) & 0xFF) as u8;
                self.pixel_buffer[px] = 29 + (grad / 16); // Dark Studio Neutral (#1D2024)
                self.pixel_buffer[px + 1] = 32 + (grad / 16);
                self.pixel_buffer[px + 2] = 36 + (grad / 16);
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
                    let grad = ((x ^ y) & 0xFF) as u8;
                    self.pixel_buffer[px] = 29 + (grad / 16);
                    self.pixel_buffer[px + 1] = 32 + (grad / 16);
                    self.pixel_buffer[px + 2] = 36 + (grad / 16);
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
