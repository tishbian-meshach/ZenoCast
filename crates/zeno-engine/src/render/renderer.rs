use crate::eval::{
    evaluate_at, ResolvedContent, ResolvedFill, ResolvedFrame, ResolvedShapeGeometry,
};
use crate::model::{Composition, LayerContent};
use crate::render::pipeline::{CompositorPipeline, LayerUniforms};
use crate::render::text_raster::RasterCache;
use crate::render::texture_pool::TexturePool;
use std::collections::HashMap;
use wgpu::{
    BufferUsages, CommandEncoderDescriptor, Device, Extent3d, Origin3d, Queue,
    RenderPassColorAttachment, RenderPassDescriptor, TexelCopyBufferInfo, TexelCopyBufferLayout,
    TexelCopyTextureInfo, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView, TextureViewDescriptor,
};
use zeno_core::{AssetId, Color, Mat4, Vec3};

/// Headless and on-screen compositor rendering engine.
pub struct ZenoRenderer {
    /// GPU shader pipeline and mesh buffers.
    pub pipeline: CompositorPipeline,
    /// Texture pool for zero-allocation intermediate frame targets.
    pub texture_pool: TexturePool,
    /// Text and vector rasterization cache.
    pub raster_cache: RasterCache,
    /// Loaded image texture cache: AssetId -> (Texture, TextureView, width, height).
    pub image_cache: HashMap<AssetId, (wgpu::Texture, TextureView, u32, u32)>,
    /// Target surface format.
    pub target_format: TextureFormat,
}

impl ZenoRenderer {
    /// Initialize a new renderer instance.
    pub fn new(device: &Device, queue: &Queue, target_format: TextureFormat) -> Self {
        let pipeline = CompositorPipeline::new(device, queue, target_format);
        let texture_pool = TexturePool::new();
        let raster_cache = RasterCache::new();

        Self {
            pipeline,
            texture_pool,
            raster_cache,
            image_cache: HashMap::new(),
            target_format,
        }
    }

    /// Register a raw RGBA8 bitmap image asset into the GPU texture cache.
    pub fn register_image_asset(
        &mut self,
        device: &Device,
        queue: &Queue,
        asset_id: AssetId,
        width: u32,
        height: u32,
        rgba_bytes: &[u8],
    ) {
        let texture = device.create_texture(&TextureDescriptor {
            label: Some("ImageAssetTexture"),
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

        let view = texture.create_view(&TextureViewDescriptor::default());
        self.image_cache
            .insert(asset_id, (texture, view, width, height));
    }

    /// Render a pre-evaluated frame onto a target texture view.
    pub fn render_resolved_frame(
        &mut self,
        device: &Device,
        queue: &Queue,
        frame: &ResolvedFrame,
        target_view: &TextureView,
        target_width: u32,
        target_height: u32,
    ) {
        // 1. Compute view-projection matrix
        let view_proj = Mat4::orthographic_lh(
            0.0,
            target_width as f32,
            target_height as f32,
            0.0,
            -1000.0,
            1000.0,
        );

        self.pipeline.update_global_matrix(queue, view_proj);

        // 2. Pre-rasterize text layers into cache before opening render pass
        for resolved in &frame.layers {
            if let LayerContent::Text(text) = &resolved.layer.content {
                self.raster_cache.get_or_rasterize_text(device, queue, text);
            }
        }

        // 3. Build per-layer draw items
        let mut draw_items = Vec::new();

        for resolved in &frame.layers {
            let opacity = resolved.net_opacity;
            if opacity <= 0.001 {
                continue;
            }

            match &resolved.content {
                ResolvedContent::Shape(shape) => {
                    let (width, height) = match &shape.geometry {
                        ResolvedShapeGeometry::Rectangle { size } => (size.x, size.y),
                        ResolvedShapeGeometry::RoundedRectangle { size, .. } => (size.x, size.y),
                        ResolvedShapeGeometry::Ellipse { radius } => {
                            (radius.x * 2.0, radius.y * 2.0)
                        }
                        ResolvedShapeGeometry::Path { .. } => (100.0, 100.0),
                    };

                    let quad_scale = Mat4::from_scale(Vec3::new(width, height, 1.0));
                    let final_world = resolved.world_matrix * quad_scale;

                    let (color, mode, grad_start, grad_end, grad_color_end) = match &shape.fill {
                        Some(ResolvedFill::Solid(c)) => (
                            c.to_f32_array(),
                            0.0f32,
                            [0.0, 0.0],
                            [1.0, 1.0],
                            [1.0, 1.0, 1.0, 1.0],
                        ),
                        Some(ResolvedFill::LinearGradient { start, end, stops }) => {
                            let start_col = stops
                                .first()
                                .map(|s| s.color.to_f32_array())
                                .unwrap_or([1.0, 1.0, 1.0, 1.0]);
                            let end_col = stops
                                .last()
                                .map(|s| s.color.to_f32_array())
                                .unwrap_or([0.0, 0.0, 0.0, 1.0]);
                            (
                                start_col,
                                2.0f32,
                                [start.x, start.y],
                                [end.x, end.y],
                                end_col,
                            )
                        }
                        Some(ResolvedFill::RadialGradient { center, stops, .. }) => {
                            let start_col = stops
                                .first()
                                .map(|s| s.color.to_f32_array())
                                .unwrap_or([1.0, 1.0, 1.0, 1.0]);
                            let end_col = stops
                                .last()
                                .map(|s| s.color.to_f32_array())
                                .unwrap_or([0.0, 0.0, 0.0, 1.0]);
                            (start_col, 2.0f32, [center.x, center.y], [1.0, 1.0], end_col)
                        }
                        None => (
                            Color::WHITE.to_f32_array(),
                            0.0f32,
                            [0.0, 0.0],
                            [1.0, 1.0],
                            [1.0, 1.0, 1.0, 1.0],
                        ),
                    };

                    let uniforms = LayerUniforms {
                        world: final_world.to_cols_array_2d(),
                        color,
                        params: [mode, opacity, 0.0, 0.0],
                        gradient_start: grad_start,
                        gradient_end: grad_end,
                        gradient_color_end: grad_color_end,
                    };

                    let (_buf, layer_bind_group) =
                        self.pipeline.create_layer_bind_group(device, &uniforms);

                    draw_items.push((layer_bind_group, None));
                }
                ResolvedContent::Image { asset } => {
                    if let Some((_tex, view, width, height)) = self.image_cache.get(asset) {
                        let quad_scale =
                            Mat4::from_scale(Vec3::new(*width as f32, *height as f32, 1.0));
                        let final_world = resolved.world_matrix * quad_scale;

                        let uniforms = LayerUniforms {
                            world: final_world.to_cols_array_2d(),
                            color: [1.0, 1.0, 1.0, 1.0],
                            params: [1.0, opacity, 0.0, 0.0],
                            gradient_start: [0.0, 0.0],
                            gradient_end: [1.0, 1.0],
                            gradient_color_end: [1.0, 1.0, 1.0, 1.0],
                        };

                        let (_buf, layer_bind_group) =
                            self.pipeline.create_layer_bind_group(device, &uniforms);
                        let tex_bind_group = self.pipeline.create_texture_bind_group(device, view);

                        draw_items.push((layer_bind_group, Some(tex_bind_group)));
                    }
                }
                ResolvedContent::Text(_) => {
                    if let LayerContent::Text(text) = &resolved.layer.content {
                        let key = RasterCache::hash_text(text);
                        if let Some(rasterized) = self.raster_cache.get_by_hash(key) {
                            let (width, height) =
                                (rasterized.width as f32, rasterized.height as f32);
                            let quad_scale = Mat4::from_scale(Vec3::new(width, height, 1.0));
                            let final_world = resolved.world_matrix * quad_scale;

                            let uniforms = LayerUniforms {
                                world: final_world.to_cols_array_2d(),
                                color: [1.0, 1.0, 1.0, 1.0],
                                params: [1.0, opacity, 0.0, 0.0],
                                gradient_start: [0.0, 0.0],
                                gradient_end: [1.0, 1.0],
                                gradient_color_end: [1.0, 1.0, 1.0, 1.0],
                            };

                            let text_view = rasterized
                                .texture
                                .create_view(&TextureViewDescriptor::default());
                            let (_buf, layer_bind_group) =
                                self.pipeline.create_layer_bind_group(device, &uniforms);
                            let tex_bind_group =
                                self.pipeline.create_texture_bind_group(device, &text_view);

                            draw_items.push((layer_bind_group, Some(tex_bind_group)));
                        }
                    }
                }
                _ => {}
            }
        }

        // 4. Begin render pass and execute all draw calls
        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("CompositorCommandEncoder"),
        });

        {
            let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("CompositorRenderPass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: target_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(&self.pipeline.pipeline);
            rpass.set_bind_group(0, &self.pipeline.global_bind_group, &[]);
            self.pipeline.bind_quad(&mut rpass);

            for (layer_bind_group, tex_bind_group) in &draw_items {
                rpass.set_bind_group(1, layer_bind_group, &[]);
                if let Some(tex_bg) = tex_bind_group {
                    rpass.set_bind_group(2, tex_bg, &[]);
                } else {
                    rpass.set_bind_group(2, &self.pipeline.white_bind_group, &[]);
                }
                rpass.draw_indexed(0..6, 0, 0..1);
            }
        }

        queue.submit(Some(encoder.finish()));
    }

    /// Render a composition at the specified timestamp onto a target texture view.
    pub fn render_composition(
        &mut self,
        device: &Device,
        queue: &Queue,
        composition: &Composition,
        time_seconds: f64,
        target_view: &TextureView,
        target_width: u32,
        target_height: u32,
    ) {
        let frame = evaluate_at(composition, time_seconds);
        self.render_resolved_frame(
            device,
            queue,
            &frame,
            target_view,
            target_width,
            target_height,
        );
    }

    /// Render a pre-evaluated frame to an in-memory RGBA image for headless testing or frame export.
    pub fn render_frame_to_image(
        &mut self,
        device: &Device,
        queue: &Queue,
        frame: &ResolvedFrame,
        width: u32,
        height: u32,
    ) -> image::RgbaImage {
        let render_target = self.texture_pool.acquire(
            device,
            width,
            height,
            TextureFormat::Rgba8UnormSrgb,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
        );

        let target_view = render_target.create_view(&TextureViewDescriptor::default());

        self.render_resolved_frame(device, queue, frame, &target_view, width, height);

        // Download rendered texture via staging buffer
        let unpadded_bytes_per_row = width * 4;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = unpadded_bytes_per_row.div_ceil(align) * align;
        let buffer_size = (padded_bytes_per_row * height) as u64;

        let staging_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("StagingBuffer"),
            size: buffer_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("ReadbackEncoder"),
        });

        encoder.copy_texture_to_buffer(
            TexelCopyTextureInfo {
                texture: &render_target,
                mip_level: 0,
                origin: Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            TexelCopyBufferInfo {
                buffer: &staging_buffer,
                layout: TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(Some(encoder.finish()));

        // Map buffer and read bytes synchronously
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
            sender.send(res).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);
        receiver.recv().unwrap().expect("map staging buffer");

        let data = buffer_slice.get_mapped_range();
        let mut image_bytes = Vec::with_capacity((width * height * 4) as usize);

        for y in 0..height {
            let start = (y * padded_bytes_per_row) as usize;
            let end = start + unpadded_bytes_per_row as usize;
            image_bytes.extend_from_slice(&data[start..end]);
        }

        drop(data);
        staging_buffer.unmap();

        image::RgbaImage::from_raw(width, height, image_bytes)
            .expect("create RgbaImage from raw rendered bytes")
    }

    /// Render a composition to an in-memory RGBA image for headless testing or frame export.
    pub fn render_to_image(
        &mut self,
        device: &Device,
        queue: &Queue,
        composition: &Composition,
        time_seconds: f64,
        width: u32,
        height: u32,
    ) -> image::RgbaImage {
        let frame = evaluate_at(composition, time_seconds);
        self.render_frame_to_image(device, queue, &frame, width, height)
    }
}
