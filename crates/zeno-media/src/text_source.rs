//! Formatted static and data-bound text source for broadcast overlays.

use crate::error::SourceError;
use crate::source::{Source, SourceKind, SourceProperties, VideoFrame};
use crate::texture_buffer::DoubleBufferedTexture;
use wgpu::{Device, Queue};
use zeno_core::SourceId;

/// Formatted text rendering source.
pub struct TextSource {
    id: SourceId,
    name: String,
    text: String,
    font_size: f32,
    color_rgba: [f32; 4],
    width: u32,
    height: u32,
    active: bool,
    dirty: bool,
    texture_buffer: DoubleBufferedTexture,
    raw_buffer: Vec<u8>,
}

impl TextSource {
    /// Creates a new text source with given string and styling.
    pub fn new(
        name: impl Into<String>,
        text: impl Into<String>,
        font_size: f32,
        color_rgba: [f32; 4],
        width: u32,
        height: u32,
    ) -> Self {
        Self {
            id: SourceId::new(),
            name: name.into(),
            text: text.into(),
            font_size: font_size.clamp(10.0, 200.0),
            color_rgba,
            width: width.max(64),
            height: height.max(32),
            active: false,
            dirty: true,
            texture_buffer: DoubleBufferedTexture::new(),
            raw_buffer: Vec::new(),
        }
    }

    /// Set explicit source ID.
    pub fn with_id(mut self, id: SourceId) -> Self {
        self.id = id;
        self
    }

    /// Update text content.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.dirty = true;
    }

    /// Current text content.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Render text string into RGBA pixel memory.
    fn rasterize_text(&mut self) {
        let w = self.width;
        let h = self.height;
        let buf_size = (w * h * 4) as usize;
        self.raw_buffer.resize(buf_size, 0);
        // Clear transparent
        self.raw_buffer.fill(0);

        let scale = (self.font_size / 16.0).max(1.0);
        let char_w = (10.0 * scale).round() as u32;
        let char_h = (16.0 * scale).round() as u32;

        let [cr, cg, cb, ca] = [
            (self.color_rgba[0] * 255.0).clamp(0.0, 255.0) as u8,
            (self.color_rgba[1] * 255.0).clamp(0.0, 255.0) as u8,
            (self.color_rgba[2] * 255.0).clamp(0.0, 255.0) as u8,
            (self.color_rgba[3] * 255.0).clamp(0.0, 255.0) as u8,
        ];

        let mut cursor_y = 8u32;

        for line in self.text.split('\n') {
            let mut cursor_x = 8u32;
            for ch in line.chars() {
                if cursor_x + char_w >= w {
                    break;
                }
                if ch != ' ' {
                    let mask = get_glyph_bitmap(ch);
                    for (row, &mask_byte) in mask.iter().enumerate() {
                        let y_start = cursor_y + (row as f32 * scale).round() as u32;
                        let y_end = (cursor_y + ((row + 1) as f32 * scale).round() as u32).min(h);

                        for col in 0..8 {
                            if (mask_byte & (1 << (7 - col))) != 0 {
                                let x_start = cursor_x + (col as f32 * scale).round() as u32;
                                let x_end =
                                    (cursor_x + ((col + 1) as f32 * scale).round() as u32).min(w);

                                for py in y_start..y_end {
                                    let row_offset = (py * w * 4) as usize;
                                    for px in x_start..x_end {
                                        let offset = row_offset + (px * 4) as usize;
                                        if offset + 3 < self.raw_buffer.len() {
                                            self.raw_buffer[offset] = cr;
                                            self.raw_buffer[offset + 1] = cg;
                                            self.raw_buffer[offset + 2] = cb;
                                            self.raw_buffer[offset + 3] = ca;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                cursor_x += char_w;
            }
            cursor_y += char_h + 4;
            if cursor_y + char_h >= h {
                break;
            }
        }
    }
}

impl Source for TextSource {
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
        SourceKind::Text
    }

    fn is_active(&self) -> bool {
        self.active
    }

    fn is_healthy(&self) -> bool {
        self.active && !self.text.is_empty()
    }

    fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn properties(&self) -> SourceProperties {
        SourceProperties::Text {
            text: self.text.clone(),
            font_family: "Inter".to_string(),
            font_size: self.font_size,
            color: self.color_rgba,
        }
    }

    fn update_properties(
        &mut self,
        properties: &SourceProperties,
        device: &Device,
        queue: &Queue,
    ) -> Result<(), SourceError> {
        if let SourceProperties::Text {
            text,
            font_size,
            color,
            ..
        } = properties
        {
            self.text = text.clone();
            self.font_size = *font_size;
            self.color_rgba = *color;
            self.dirty = true;
            self.rasterize_text();
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
                "Expected Text properties".to_string(),
            ))
        }
    }

    fn start(&mut self, device: &Device, queue: &Queue) -> Result<(), SourceError> {
        self.active = true;
        self.dirty = true;
        self.rasterize_text();
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
            self.rasterize_text();
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

/// 8x12 standard bitmap glyph table
fn get_glyph_bitmap(c: char) -> [u8; 12] {
    match c {
        '0' => [
            0x3C, 0x66, 0xC3, 0xC7, 0xCF, 0xDB, 0xF3, 0xE3, 0xC3, 0x66, 0x3C, 0x00,
        ],
        '1' => [
            0x18, 0x38, 0x78, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00,
        ],
        '2' => [
            0x7C, 0xC6, 0x06, 0x0C, 0x18, 0x30, 0x60, 0xC0, 0xC0, 0xC6, 0xFE, 0x00,
        ],
        '3' => [
            0x7C, 0xC6, 0x06, 0x06, 0x3C, 0x06, 0x06, 0x06, 0x06, 0xC6, 0x7C, 0x00,
        ],
        '4' => [
            0x0C, 0x1C, 0x3C, 0x6C, 0xCC, 0xFE, 0x0C, 0x0C, 0x0C, 0x0C, 0x1E, 0x00,
        ],
        '5' => [
            0xFE, 0xC0, 0xC0, 0xC0, 0xFC, 0x06, 0x06, 0x06, 0x06, 0xC6, 0x7C, 0x00,
        ],
        '6' => [
            0x38, 0x60, 0xC0, 0xC0, 0xFC, 0xC6, 0xC6, 0xC6, 0xC6, 0x66, 0x3C, 0x00,
        ],
        '7' => [
            0xFE, 0xC6, 0x06, 0x0C, 0x18, 0x30, 0x30, 0x60, 0x60, 0x60, 0x60, 0x00,
        ],
        '8' => [
            0x3C, 0x66, 0xC3, 0xC3, 0x66, 0x3C, 0x66, 0xC3, 0xC3, 0x66, 0x3C, 0x00,
        ],
        '9' => [
            0x3C, 0x66, 0xC3, 0xC3, 0xC3, 0x7E, 0x06, 0x06, 0x06, 0x0C, 0x38, 0x00,
        ],
        'A' | 'a' => [
            0x18, 0x3C, 0x66, 0xC3, 0xC3, 0xFF, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0x00,
        ],
        'B' | 'b' => [
            0xFC, 0x66, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x66, 0x66, 0x66, 0xFC, 0x00,
        ],
        'C' | 'c' => [
            0x3C, 0x66, 0xC3, 0xC0, 0xC0, 0xC0, 0xC0, 0xC0, 0xC3, 0x66, 0x3C, 0x00,
        ],
        'D' | 'd' => [
            0xF8, 0x6C, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x66, 0x6C, 0xF8, 0x00,
        ],
        'E' | 'e' => [
            0xFE, 0x62, 0x62, 0x68, 0x78, 0x68, 0x60, 0x62, 0x62, 0x62, 0xFE, 0x00,
        ],
        'F' | 'f' => [
            0xFE, 0x62, 0x62, 0x68, 0x78, 0x68, 0x60, 0x60, 0x60, 0x60, 0xF0, 0x00,
        ],
        'G' | 'g' => [
            0x3C, 0x66, 0xC3, 0xC0, 0xC0, 0xCF, 0xC3, 0xC3, 0xC3, 0x66, 0x3E, 0x00,
        ],
        'H' | 'h' => [
            0xC3, 0xC3, 0xC3, 0xC3, 0xFF, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0x00,
        ],
        'I' | 'i' => [
            0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x7E, 0x00,
        ],
        'J' | 'j' => [
            0x1E, 0x06, 0x06, 0x06, 0x06, 0x06, 0x06, 0xC6, 0xC6, 0x66, 0x3C, 0x00,
        ],
        'K' | 'k' => [
            0xC3, 0xC6, 0xCC, 0xD8, 0xF0, 0xF8, 0xDC, 0xCE, 0xC6, 0xC3, 0xC3, 0x00,
        ],
        'L' | 'l' => [
            0xE0, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x60, 0x62, 0x66, 0xFE, 0x00,
        ],
        'M' | 'm' => [
            0xC3, 0xE7, 0xFF, 0xDB, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0x00,
        ],
        'N' | 'n' => [
            0xC3, 0xE3, 0xF3, 0xFB, 0xDF, 0xCF, 0xC7, 0xC3, 0xC3, 0xC3, 0xC3, 0x00,
        ],
        'O' | 'o' => [
            0x3C, 0x66, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0x66, 0x3C, 0x00,
        ],
        'P' | 'p' => [
            0xFC, 0x66, 0x66, 0x66, 0x7C, 0x60, 0x60, 0x60, 0x60, 0x60, 0xF0, 0x00,
        ],
        'R' | 'r' => [
            0xFC, 0x66, 0x66, 0x66, 0x7C, 0x6C, 0x66, 0x66, 0x66, 0x66, 0xE3, 0x00,
        ],
        'S' | 's' => [
            0x3C, 0x66, 0xC3, 0xE0, 0x7C, 0x0E, 0x07, 0x03, 0xC3, 0x66, 0x3C, 0x00,
        ],
        'T' | 't' => [
            0xFF, 0xDB, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00,
        ],
        'U' | 'u' => [
            0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0x66, 0x3C, 0x00,
        ],
        'V' | 'v' => [
            0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0x66, 0x66, 0x66, 0x3C, 0x18, 0x18, 0x00,
        ],
        'W' | 'w' => [
            0xC3, 0xC3, 0xC3, 0xC3, 0xC3, 0xDB, 0xDB, 0xFF, 0x66, 0x66, 0x24, 0x00,
        ],
        'X' | 'x' => [
            0xC3, 0xC3, 0x66, 0x3C, 0x18, 0x18, 0x3C, 0x66, 0xC3, 0xC3, 0xC3, 0x00,
        ],
        'Y' | 'y' => [
            0xC3, 0xC3, 0x66, 0x3C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3C, 0x00,
        ],
        'Z' | 'z' => [
            0xFF, 0xC3, 0x06, 0x0C, 0x18, 0x30, 0x60, 0xC0, 0xC1, 0xC3, 0xFF, 0x00,
        ],
        '-' => [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x7E, 0x7E, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
        ':' => [
            0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00,
        ],
        '.' => [
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x18, 0x18, 0x00,
        ],
        _ => [
            0x7E, 0xC3, 0x81, 0xBD, 0xA5, 0xA5, 0xBD, 0x81, 0xC3, 0x7E, 0x00, 0x00,
        ],
    }
}
