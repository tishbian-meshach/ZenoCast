//! Static image file (PNG, JPEG, etc.) source for broadcast graphics.

use crate::error::SourceError;
use crate::source::{Source, SourceKind, SourceProperties, VideoFrame};
use crate::texture_buffer::DoubleBufferedTexture;
use std::path::Path;
use tracing::warn;
use wgpu::{Device, Queue};
use zeno_core::SourceId;

/// Static image source loading PNG or JPEG files into GPU textures.
pub struct ImageSource {
    id: SourceId,
    name: String,
    file_path: String,
    scale_mode: String,
    width: u32,
    height: u32,
    active: bool,
    dirty: bool,
    healthy: bool,
    texture_buffer: DoubleBufferedTexture,
    raw_buffer: Vec<u8>,
}

impl ImageSource {
    /// Creates a new image source targeting an image file path.
    pub fn new(name: impl Into<String>, file_path: impl Into<String>) -> Self {
        let path_str = file_path.into();
        Self {
            id: SourceId::new(),
            name: name.into(),
            file_path: path_str,
            scale_mode: "Fit".to_string(),
            width: 1920,
            height: 1080,
            active: false,
            dirty: true,
            healthy: false,
            texture_buffer: DoubleBufferedTexture::new(),
            raw_buffer: Vec::new(),
        }
    }

    /// Set explicit source ID.
    pub fn with_id(mut self, id: SourceId) -> Self {
        self.id = id;
        self
    }

    /// Set image file path.
    pub fn set_file_path(&mut self, path: impl Into<String>) {
        self.file_path = path.into();
        self.dirty = true;
    }

    /// Current file path.
    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    /// Loads image bytes from disk into RGBA8 buffer.
    fn load_image_pixels(&mut self) {
        let path = Path::new(&self.file_path);
        if path.exists() {
            if let Ok(dyn_img) = image::open(path) {
                let rgba = dyn_img.to_rgba8();
                self.width = rgba.width().max(1);
                self.height = rgba.height().max(1);
                self.raw_buffer = rgba.into_raw();
                self.healthy = true;
                return;
            } else {
                warn!("ImageSource failed to decode image at '{}'", self.file_path);
            }
        }

        // Image file missing or unreadable: generate clear offline graphic placeholder
        self.healthy = false;
        self.width = 1280;
        self.height = 720;
        let buf_size = (self.width * self.height * 4) as usize;
        self.raw_buffer.resize(buf_size, 0);

        for y in 0..self.height {
            let row_offset = (y * self.width * 4) as usize;
            let is_edge = y < 8 || y >= self.height - 8;
            for x in 0..self.width {
                let px = row_offset + (x * 4) as usize;
                if is_edge || x < 8 || x >= self.width - 8 {
                    self.raw_buffer[px] = 39; // Neutral frame border #272B30
                    self.raw_buffer[px + 1] = 43;
                    self.raw_buffer[px + 2] = 48;
                    self.raw_buffer[px + 3] = 255;
                } else if (x == y || x + y == self.width) && (400..=880).contains(&x) {
                    // Placeholder diagonal crossbars
                    self.raw_buffer[px] = 76; // Accent indicator #4C8DFF
                    self.raw_buffer[px + 1] = 141;
                    self.raw_buffer[px + 2] = 255;
                    self.raw_buffer[px + 3] = 255;
                } else {
                    self.raw_buffer[px] = 21; // Dark neutral background #15171A
                    self.raw_buffer[px + 1] = 23;
                    self.raw_buffer[px + 2] = 26;
                    self.raw_buffer[px + 3] = 255;
                }
            }
        }
    }
}

impl Source for ImageSource {
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
        SourceKind::Image
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn is_healthy(&self) -> bool {
        self.active && self.healthy
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn properties(&self) -> SourceProperties {
        SourceProperties::Image {
            path: self.file_path.clone(),
            scale_mode: self.scale_mode.clone(),
        }
    }

    fn update_properties(
        &mut self,
        properties: &SourceProperties,
        device: &Device,
        queue: &Queue,
    ) -> Result<(), SourceError> {
        if let SourceProperties::Image { path, scale_mode } = properties {
            self.file_path = path.clone();
            self.scale_mode = scale_mode.clone();
            self.load_image_pixels();
            self.texture_buffer.upload(
                device,
                queue,
                self.width,
                self.height,
                &self.raw_buffer,
                0.0,
            );
            self.dirty = false;
            Ok(())
        } else {
            Err(SourceError::InvalidProperties(
                "Expected Image properties".to_string(),
            ))
        }
    }

    fn start(&mut self, device: &Device, queue: &Queue) -> Result<(), SourceError> {
        self.active = true;
        self.dirty = true;
        self.load_image_pixels();
        self.texture_buffer.upload(
            device,
            queue,
            self.width,
            self.height,
            &self.raw_buffer,
            0.0,
        );
        self.dirty = false;
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

        if self.dirty {
            self.load_image_pixels();
            self.texture_buffer.upload(
                device,
                queue,
                self.width,
                self.height,
                &self.raw_buffer,
                current_time,
            );
            self.dirty = false;
        }
    }
}
