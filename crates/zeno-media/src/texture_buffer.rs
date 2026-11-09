//! Double-buffered GPU texture holder for tear-free asynchronous video frame presentation.

use crate::source::VideoFrame;
use std::sync::Arc;
use wgpu::{
    Device, Extent3d, Origin3d, Queue, TexelCopyBufferLayout, TexelCopyTextureInfo, Texture,
    TextureAspect, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor,
};

/// Ping-pong double-buffered GPU texture pair for jitter-free real-time frame delivery.
pub struct DoubleBufferedTexture {
    width: u32,
    height: u32,
    format: TextureFormat,
    front: Option<VideoFrame>,
    back_texture: Option<Arc<Texture>>,
    back_view: Option<Arc<TextureView>>,
}

impl DoubleBufferedTexture {
    /// Create an empty texture buffer using standard Rgba8UnormSrgb format.
    pub fn new() -> Self {
        Self::with_format(TextureFormat::Rgba8UnormSrgb)
    }

    /// Create an empty texture buffer with specified texture format (e.g. Bgra8UnormSrgb for WGC/GDI).
    pub fn with_format(format: TextureFormat) -> Self {
        Self {
            width: 0,
            height: 0,
            format,
            front: None,
            back_texture: None,
            back_view: None,
        }
    }

    /// Uploads a pixel buffer into the GPU texture, swapping front/back buffers without recreating views.
    pub fn upload(
        &mut self,
        device: &Device,
        queue: &Queue,
        width: u32,
        height: u32,
        pixel_bytes: &[u8],
        timestamp_seconds: f64,
    ) {
        if width == 0 || height == 0 || pixel_bytes.len() < (width * height * 4) as usize {
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
                format: self.format,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            };

            let tex_a = Arc::new(device.create_texture(&desc));
            let view_a = Arc::new(tex_a.create_view(&TextureViewDescriptor::default()));
            let tex_b = Arc::new(device.create_texture(&desc));
            let view_b = Arc::new(tex_b.create_view(&TextureViewDescriptor::default()));

            self.front = Some(VideoFrame {
                width,
                height,
                timestamp_seconds,
                texture: tex_a,
                texture_view: view_a,
            });
            self.back_texture = Some(tex_b);
            self.back_view = Some(view_b);
            self.width = width;
            self.height = height;
        }

        // Write pixels to back texture
        if let (Some(back_tex), Some(back_view)) = (&self.back_texture, &self.back_view) {
            queue.write_texture(
                TexelCopyTextureInfo {
                    texture: back_tex,
                    mip_level: 0,
                    origin: Origin3d::ZERO,
                    aspect: TextureAspect::All,
                },
                pixel_bytes,
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

            // Swap front and back (zero view allocation)
            let new_front = VideoFrame {
                width,
                height,
                timestamp_seconds,
                texture: Arc::clone(back_tex),
                texture_view: Arc::clone(back_view),
            };

            if let Some(old_front) = self.front.replace(new_front) {
                self.back_texture = Some(old_front.texture);
                self.back_view = Some(old_front.texture_view);
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
