//! Solid flat color video source for background fill, testing, and compositor verification.

use crate::error::SourceError;
use crate::source::{Source, SourceKind, SourceProperties, VideoFrame};
use crate::texture_buffer::DoubleBufferedTexture;
use wgpu::{Device, Queue};
use zeno_core::SourceId;

/// Solid flat color video source.
pub struct ColorSource {
    id: SourceId,
    name: String,
    color: [f32; 4],
    width: u32,
    height: u32,
    active: bool,
    texture_buffer: DoubleBufferedTexture,
    raw_buffer: Vec<u8>,
    dirty: bool,
}

impl ColorSource {
    /// Creates a new color source with the given dimensions and RGBA color.
    pub fn new(name: impl Into<String>, color: [f32; 4], width: u32, height: u32) -> Self {
        Self {
            id: SourceId::new(),
            name: name.into(),
            color,
            width: width.max(1),
            height: height.max(1),
            active: false,
            texture_buffer: DoubleBufferedTexture::new(),
            raw_buffer: Vec::new(),
            dirty: true,
        }
    }

    /// Sets a specific SourceId for this instance.
    pub fn with_id(mut self, id: SourceId) -> Self {
        self.id = id;
        self
    }

    /// Helper to regenerate the raw byte buffer from the current color.
    fn refresh_buffer(&mut self) {
        let r = (self.color[0].clamp(0.0, 1.0) * 255.0) as u8;
        let g = (self.color[1].clamp(0.0, 1.0) * 255.0) as u8;
        let b = (self.color[2].clamp(0.0, 1.0) * 255.0) as u8;
        let a = (self.color[3].clamp(0.0, 1.0) * 255.0) as u8;

        let total_pixels = (self.width * self.height) as usize;
        self.raw_buffer.resize(total_pixels * 4, 0);

        for px in self.raw_buffer.chunks_exact_mut(4) {
            px[0] = r;
            px[1] = g;
            px[2] = b;
            px[3] = a;
        }
        self.dirty = true;
    }
}

impl Source for ColorSource {
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
        SourceKind::Color
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn properties(&self) -> SourceProperties {
        SourceProperties::Color {
            color: self.color,
            width: self.width,
            height: self.height,
        }
    }

    fn update_properties(
        &mut self,
        props: &SourceProperties,
        device: &Device,
        queue: &Queue,
    ) -> Result<(), SourceError> {
        if let SourceProperties::Color {
            color,
            width,
            height,
        } = props
        {
            self.color = *color;
            self.width = (*width).max(1);
            self.height = (*height).max(1);
            self.refresh_buffer();
            if self.active {
                self.texture_buffer.upload(
                    device,
                    queue,
                    self.width,
                    self.height,
                    &self.raw_buffer,
                    0.0,
                );
                self.dirty = false;
            }
        }
        Ok(())
    }

    fn start(&mut self, device: &Device, queue: &Queue) -> Result<(), SourceError> {
        self.active = true;
        self.refresh_buffer();
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
