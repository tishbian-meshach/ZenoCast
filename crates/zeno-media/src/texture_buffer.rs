//! Double-buffered GPU texture holder for tear-free asynchronous video frame presentation.

use crate::source::VideoFrame;
use std::sync::Arc;
use wgpu::{
    Device, Extent3d, Origin3d, Queue, TexelCopyBufferLayout, TexelCopyTextureInfo, Texture,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureViewDescriptor,
};

/// Ping-pong double-buffered GPU texture pair for jitter-free real-time frame delivery.
pub struct DoubleBufferedTexture {
    width: u32,
    height: u32,
    front: Option<VideoFrame>,
    back_texture: Option<Arc<Texture>>,
}

impl DoubleBufferedTexture {
    /// Create an empty texture buffer.
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            front: None,
            back_texture: None,
        }
    }

    /// Uploads an RGBA8 pixel buffer into the GPU texture, swapping buffers if valid.
    pub fn upload(
        &mut self,
        device: &Device,
        queue: &Queue,
        width: u32,
        height: u32,
        rgba_bytes: &[u8],
        timestamp_seconds: f64,
    ) {
        if width == 0 || height == 0 || rgba_bytes.len() < (width * height * 4) as usize {
            return;
        }

        // Reallocate textures if dimensions changed or uninitialized
        if self.width != width || self.height != height || self.back_texture.is_none() {
            let desc = TextureDescriptor {
                label: Some("CaptureSourceTexture"),
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
            };

            let tex_a = Arc::new(device.create_texture(&desc));
            let view_a = Arc::new(tex_a.create_view(&TextureViewDescriptor::default()));
            let tex_b = Arc::new(device.create_texture(&desc));

            self.front = Some(VideoFrame {
                width,
                height,
                timestamp_seconds,
                texture: tex_a,
                texture_view: view_a,
            });
            self.back_texture = Some(tex_b);
            self.width = width;
            self.height = height;
        }

        // Write pixels to back texture
        if let Some(back_tex) = &self.back_texture {
            queue.write_texture(
                TexelCopyTextureInfo {
                    texture: back_tex,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                rgba_bytes,
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

            // Swap front and back
            let back_view = Arc::new(back_tex.create_view(&TextureViewDescriptor::default()));
            let old_front_tex = self.front.as_ref().map(|f| Arc::clone(&f.texture));

            self.front = Some(VideoFrame {
                width,
                height,
                timestamp_seconds,
                texture: Arc::clone(back_tex),
                texture_view: back_view,
            });

            if let Some(old_tex) = old_front_tex {
                self.back_texture = Some(old_tex);
            }
        }
    }

    /// Retrieve reference to the latest presented video frame.
    #[inline]
    pub fn current_frame(&self) -> Option<&VideoFrame> {
        self.front.as_ref()
    }

    /// Current resolution.
    #[inline]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
}

impl Default for DoubleBufferedTexture {
    fn default() -> Self {
        Self::new()
    }
}
