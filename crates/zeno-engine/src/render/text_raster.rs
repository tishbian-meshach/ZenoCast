use crate::model::TextContent;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use wgpu::{
    Device, Extent3d, Origin3d, Queue, TexelCopyBufferLayout, TexelCopyTextureInfo, Texture,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

/// Cached GPU texture representation of rasterized vector or text content.
pub struct CachedRasterTexture {
    /// GPU texture.
    pub texture: Texture,
    /// Width of the rasterized surface in pixels.
    pub width: u32,
    /// Height of the rasterized surface in pixels.
    pub height: u32,
}

/// Cache of pre-rasterized text and vector assets keyed by content hash.
pub struct RasterCache {
    cache: HashMap<u64, CachedRasterTexture>,
}

impl Default for RasterCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RasterCache {
    /// Create a new empty raster cache.
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
        }
    }

    /// Compute 64-bit content hash for text content.
    pub fn hash_text(content: &TextContent) -> u64 {
        let mut hasher = DefaultHasher::new();
        content.text.fallback_value().hash(&mut hasher);
        content.font_family.hash(&mut hasher);
        (content.font_size.fallback_value().to_bits()).hash(&mut hasher);
        content.font_weight.hash(&mut hasher);
        content.color.fallback_value().to_hex().hash(&mut hasher);
        hasher.finish()
    }

    /// Retrieve previously cached rasterized texture by its content hash.
    pub fn get_by_hash(&self, key: u64) -> Option<&CachedRasterTexture> {
        self.cache.get(&key)
    }

    /// Retrieve or rasterize text content into a GPU texture.
    pub fn get_or_rasterize_text(
        &mut self,
        device: &Device,
        queue: &Queue,
        content: &TextContent,
    ) -> &CachedRasterTexture {
        let key = Self::hash_text(content);

        self.cache
            .entry(key)
            .or_insert_with(|| rasterize_text_to_texture(device, queue, content))
    }
}

/// Rasterizes formatted text string into an RGBA8 GPU texture.
fn rasterize_text_to_texture(
    device: &Device,
    queue: &Queue,
    content: &TextContent,
) -> CachedRasterTexture {
    let text = content.text.fallback_value();
    let font_size = *content.font_size.fallback_value();
    let color = *content.color.fallback_value();

    let scale = (font_size / 16.0).max(1.0);
    let char_w = (10.0 * scale).round() as u32;
    let char_h = (16.0 * scale).round() as u32;

    let width = ((text.len() as u32 * char_w) + 16).max(32);
    let height = (char_h + 16).max(32);

    let mut pixels = vec![0u8; (width * height * 4) as usize];
    let [cr, cg, cb, ca] = color.to_u8_array();

    // Render characters using an 8x12 bitmap font definition scaled to font_size
    let mut x_offset = 8u32;
    let y_offset = 8u32;

    for ch in text.chars() {
        if ch == ' ' {
            x_offset += char_w;
            continue;
        }

        let glyph_mask = get_char_bitmap(ch);
        for (row, &mask_byte) in glyph_mask.iter().enumerate() {
            for col in 0..8 {
                if (mask_byte & (1 << (7 - col))) != 0 {
                    // Fill pixel block according to scale
                    let px_start = x_offset + (col as f32 * scale).round() as u32;
                    let py_start = y_offset + (row as f32 * scale).round() as u32;

                    let px_end = (px_start + scale.ceil() as u32).min(width);
                    let py_end = (py_start + scale.ceil() as u32).min(height);

                    for py in py_start..py_end {
                        for px in px_start..px_end {
                            let idx = ((py * width + px) * 4) as usize;
                            if idx + 3 < pixels.len() {
                                pixels[idx] = cr;
                                pixels[idx + 1] = cg;
                                pixels[idx + 2] = cb;
                                pixels[idx + 3] = ca;
                            }
                        }
                    }
                }
            }
        }
        x_offset += char_w;
    }

    let texture = device.create_texture(&TextureDescriptor {
        label: Some("RasterizedTextTexture"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });

    queue.write_texture(
        TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        &pixels,
        TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(height),
        },
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );

    CachedRasterTexture {
        texture,
        width,
        height,
    }
}

/// 8x12 bitmap font glyph definition for ASCII characters 32..=126.
fn get_char_bitmap(c: char) -> [u8; 12] {
    match c {
        'A' => [
            0x18, 0x24, 0x42, 0x42, 0x7E, 0x42, 0x42, 0x42, 0x42, 0x00, 0x00, 0x00,
        ],
        'B' => [
            0x7C, 0x42, 0x42, 0x7C, 0x42, 0x42, 0x42, 0x7C, 0x00, 0x00, 0x00, 0x00,
        ],
        'C' => [
            0x3C, 0x42, 0x40, 0x40, 0x40, 0x40, 0x42, 0x3C, 0x00, 0x00, 0x00, 0x00,
        ],
        'D' => [
            0x78, 0x44, 0x42, 0x42, 0x42, 0x42, 0x44, 0x78, 0x00, 0x00, 0x00, 0x00,
        ],
        'E' => [
            0x7E, 0x40, 0x40, 0x78, 0x40, 0x40, 0x40, 0x7E, 0x00, 0x00, 0x00, 0x00,
        ],
        'F' => [
            0x7E, 0x40, 0x40, 0x78, 0x40, 0x40, 0x40, 0x40, 0x00, 0x00, 0x00, 0x00,
        ],
        'G' => [
            0x3C, 0x42, 0x40, 0x4E, 0x42, 0x42, 0x42, 0x3C, 0x00, 0x00, 0x00, 0x00,
        ],
        'H' => [
            0x42, 0x42, 0x42, 0x7E, 0x42, 0x42, 0x42, 0x42, 0x00, 0x00, 0x00, 0x00,
        ],
        'I' => [
            0x3E, 0x1C, 0x18, 0x18, 0x18, 0x18, 0x18, 0x3E, 0x00, 0x00, 0x00, 0x00,
        ],
        'J' => [
            0x1E, 0x0C, 0x0C, 0x0C, 0x0C, 0x4C, 0x4C, 0x38, 0x00, 0x00, 0x00, 0x00,
        ],
        'K' => [
            0x42, 0x44, 0x48, 0x70, 0x58, 0x4C, 0x46, 0x42, 0x00, 0x00, 0x00, 0x00,
        ],
        'L' => [
            0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x40, 0x7E, 0x00, 0x00, 0x00, 0x00,
        ],
        'M' => [
            0x42, 0x66, 0x5A, 0x42, 0x42, 0x42, 0x42, 0x42, 0x00, 0x00, 0x00, 0x00,
        ],
        'N' => [
            0x42, 0x62, 0x52, 0x4A, 0x46, 0x42, 0x42, 0x42, 0x00, 0x00, 0x00, 0x00,
        ],
        'O' => [
            0x3C, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00, 0x00, 0x00, 0x00,
        ],
        'P' => [
            0x7C, 0x42, 0x42, 0x7C, 0x40, 0x40, 0x40, 0x40, 0x00, 0x00, 0x00, 0x00,
        ],
        'Q' => [
            0x3C, 0x42, 0x42, 0x42, 0x42, 0x4A, 0x44, 0x3A, 0x00, 0x00, 0x00, 0x00,
        ],
        'R' => [
            0x7C, 0x42, 0x42, 0x7C, 0x48, 0x44, 0x42, 0x42, 0x00, 0x00, 0x00, 0x00,
        ],
        'S' => [
            0x3C, 0x42, 0x40, 0x38, 0x04, 0x02, 0x42, 0x3C, 0x00, 0x00, 0x00, 0x00,
        ],
        'T' => [
            0x7E, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00,
        ],
        'U' => [
            0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x42, 0x3C, 0x00, 0x00, 0x00, 0x00,
        ],
        'V' => [
            0x42, 0x42, 0x42, 0x42, 0x42, 0x24, 0x24, 0x18, 0x00, 0x00, 0x00, 0x00,
        ],
        'W' => [
            0x42, 0x42, 0x42, 0x42, 0x5A, 0x66, 0x66, 0x42, 0x00, 0x00, 0x00, 0x00,
        ],
        'X' => [
            0x42, 0x42, 0x24, 0x18, 0x18, 0x24, 0x42, 0x42, 0x00, 0x00, 0x00, 0x00,
        ],
        'Y' => [
            0x42, 0x42, 0x24, 0x18, 0x18, 0x18, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00,
        ],
        'Z' => [
            0x7E, 0x04, 0x08, 0x10, 0x20, 0x40, 0x40, 0x7E, 0x00, 0x00, 0x00, 0x00,
        ],
        '0' => [
            0x3C, 0x46, 0x4A, 0x52, 0x62, 0x42, 0x42, 0x3C, 0x00, 0x00, 0x00, 0x00,
        ],
        '1' => [
            0x18, 0x28, 0x08, 0x08, 0x08, 0x08, 0x08, 0x3E, 0x00, 0x00, 0x00, 0x00,
        ],
        '2' => [
            0x3C, 0x42, 0x02, 0x0C, 0x30, 0x40, 0x40, 0x7E, 0x00, 0x00, 0x00, 0x00,
        ],
        '3' => [
            0x3C, 0x42, 0x02, 0x1C, 0x02, 0x02, 0x42, 0x3C, 0x00, 0x00, 0x00, 0x00,
        ],
        '4' => [
            0x08, 0x18, 0x28, 0x48, 0x7E, 0x08, 0x08, 0x08, 0x00, 0x00, 0x00, 0x00,
        ],
        '5' => [
            0x7E, 0x40, 0x7C, 0x02, 0x02, 0x02, 0x42, 0x3C, 0x00, 0x00, 0x00, 0x00,
        ],
        '6' => [
            0x3C, 0x40, 0x40, 0x7C, 0x42, 0x42, 0x42, 0x3C, 0x00, 0x00, 0x00, 0x00,
        ],
        '7' => [
            0x7E, 0x02, 0x04, 0x08, 0x10, 0x20, 0x20, 0x20, 0x00, 0x00, 0x00, 0x00,
        ],
        '8' => [
            0x3C, 0x42, 0x42, 0x3C, 0x42, 0x42, 0x42, 0x3C, 0x00, 0x00, 0x00, 0x00,
        ],
        '9' => [
            0x3C, 0x42, 0x42, 0x3E, 0x02, 0x02, 0x02, 0x3C, 0x00, 0x00, 0x00, 0x00,
        ],
        ':' => [
            0x00, 0x18, 0x18, 0x00, 0x00, 0x18, 0x18, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
        '-' => [
            0x00, 0x00, 0x00, 0x7E, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
        '/' => [
            0x02, 0x04, 0x08, 0x10, 0x20, 0x40, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
        _ => [
            0x00, 0x00, 0x3C, 0x24, 0x24, 0x3C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ],
    }
}
