//! High-performance GPU texture recycling pool to prevent per-frame allocations.

use std::collections::HashMap;
use std::sync::Mutex;
use wgpu::{
    Device, Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
};

/// Key uniquely identifying compatible GPU textures in the pool.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TexturePoolKey {
    /// Texture width in pixels.
    pub width: u32,
    /// Texture height in pixels.
    pub height: u32,
    /// Pixel format.
    pub format: TextureFormat,
    /// Usage flags.
    pub usage: TextureUsages,
}

/// A pooled GPU texture with automatic release back into the pool on drop.
pub struct PooledTexture {
    texture: Option<Texture>,
    key: TexturePoolKey,
    pool: std::sync::Arc<TexturePoolInner>,
}

impl PooledTexture {
    /// Access the underlying wgpu texture.
    #[inline]
    pub fn texture(&self) -> &Texture {
        self.texture.as_ref().expect("pooled texture active")
    }

    /// Access the underlying wgpu texture key.
    #[inline]
    pub fn key(&self) -> TexturePoolKey {
        self.key
    }
}

impl std::ops::Deref for PooledTexture {
    type Target = Texture;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.texture()
    }
}

impl Drop for PooledTexture {
    fn drop(&mut self) {
        if let Some(tex) = self.texture.take() {
            let mut free_list = self.pool.free_textures.lock().unwrap();
            free_list.entry(self.key).or_default().push(tex);
        }
    }
}

struct TexturePoolInner {
    free_textures: Mutex<HashMap<TexturePoolKey, Vec<Texture>>>,
    total_allocated: Mutex<usize>,
}

/// Reusable texture pool for intermediate render targets and frame buffers.
#[derive(Clone)]
pub struct TexturePool {
    inner: std::sync::Arc<TexturePoolInner>,
}

impl Default for TexturePool {
    fn default() -> Self {
        Self::new()
    }
}

impl TexturePool {
    /// Create a new empty texture pool.
    pub fn new() -> Self {
        Self {
            inner: std::sync::Arc::new(TexturePoolInner {
                free_textures: Mutex::new(HashMap::new()),
                total_allocated: Mutex::new(0),
            }),
        }
    }

    /// Acquire a texture matching the specified dimensions, format, and usage.
    /// Reuses a previously allocated texture if available, or creates a new one on `device`.
    pub fn acquire(
        &self,
        device: &Device,
        width: u32,
        height: u32,
        format: TextureFormat,
        usage: TextureUsages,
    ) -> PooledTexture {
        let key = TexturePoolKey {
            width,
            height,
            format,
            usage,
        };

        let recycled = {
            let mut free_list = self.inner.free_textures.lock().unwrap();
            free_list.get_mut(&key).and_then(|list| list.pop())
        };

        let texture = match recycled {
            Some(tex) => tex,
            None => {
                let mut count = self.inner.total_allocated.lock().unwrap();
                *count += 1;
                device.create_texture(&TextureDescriptor {
                    label: Some("PooledTexture"),
                    size: Extent3d {
                        width,
                        height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
            }
        };

        PooledTexture {
            texture: Some(texture),
            key,
            pool: self.inner.clone(),
        }
    }

    /// Number of total textures allocated across the lifetime of the pool.
    pub fn total_allocated(&self) -> usize {
        *self.inner.total_allocated.lock().unwrap()
    }

    /// Number of currently idle textures ready for reuse.
    pub fn free_count(&self) -> usize {
        let free_list = self.inner.free_textures.lock().unwrap();
        free_list.values().map(|v| v.len()).sum()
    }

    /// Clear all pooled textures from memory.
    pub fn clear(&self) {
        let mut free_list = self.inner.free_textures.lock().unwrap();
        free_list.clear();
    }
}
