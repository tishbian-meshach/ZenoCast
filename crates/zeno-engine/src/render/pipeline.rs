//! GPU render pipeline setup, vertex layouts, uniform buffers, and quad meshes.

use crate::model::BlendMode;
use bytemuck::{Pod, Zeroable};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
    BlendComponent, BlendFactor, BlendOperation, BlendState, Buffer, BufferAddress,
    BufferBindingType, BufferUsages, ColorTargetState, ColorWrites, Device, Extent3d,
    FragmentState, FrontFace, IndexFormat, MultisampleState, PipelineCompilationOptions,
    PipelineLayoutDescriptor, PrimitiveState, PrimitiveTopology, Queue, RenderPass, RenderPipeline,
    RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor,
    ShaderModuleDescriptor, ShaderSource, ShaderStages, Texture, TextureAspect, TextureDescriptor,
    TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension, VertexAttribute, VertexBufferLayout, VertexFormat,
    VertexState, VertexStepMode,
};

/// 2D Quad vertex with normalized position and texture coordinate.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct Vertex {
    /// 2D position in local space [0.0..1.0].
    pub position: [f32; 2],
    /// Normalized UV coordinate [0.0..1.0].
    pub uv: [f32; 2],
}

impl Vertex {
    const ATTRIBUTES: [VertexAttribute; 2] = [
        VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: VertexFormat::Float32x2,
        },
        VertexAttribute {
            offset: std::mem::size_of::<[f32; 2]>() as BufferAddress,
            shader_location: 1,
            format: VertexFormat::Float32x2,
        },
    ];

    /// Vertex buffer layout for wgpu pipeline.
    pub fn desc<'a>() -> VertexBufferLayout<'a> {
        VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as BufferAddress,
            step_mode: VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// Unit quad vertices [0..1, 0..1].
pub const QUAD_VERTICES: &[Vertex] = &[
    Vertex {
        position: [0.0, 0.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [1.0, 0.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.0, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0],
        uv: [1.0, 1.0],
    },
];

/// Unit quad index buffer.
pub const QUAD_INDICES: &[u16] = &[0, 2, 1, 1, 2, 3];

/// Global uniforms passed to vertex/fragment shaders.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct GlobalUniforms {
    /// 4x4 Orthographic or Perspective Projection-View matrix.
    pub view_proj: [[f32; 4]; 4],
}

/// Per-layer uniforms passed to vertex/fragment shaders.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct LayerUniforms {
    /// 4x4 layer world transform matrix.
    pub world: [[f32; 4]; 4],
    /// Primary color or texture tint.
    pub color: [f32; 4],
    /// Parameters: [mode (0=rect, 1=textured, 2=gradient), opacity, corner_radius, unused].
    pub params: [f32; 4],
    /// Gradient start UV.
    pub gradient_start: [f32; 2],
    /// Gradient end UV.
    pub gradient_end: [f32; 2],
    /// Gradient ending color.
    pub gradient_color_end: [f32; 4],
}

/// GPU resources managing compositing shaders, mesh buffers, and render pipelines.
pub struct CompositorPipeline {
    /// Active render pipeline (default/normal).
    pub pipeline: RenderPipeline,
    /// Premultiplied alpha Normal blend pipeline.
    pub pipeline_normal: RenderPipeline,
    /// Premultiplied alpha Add blend pipeline.
    pub pipeline_add: RenderPipeline,
    /// Premultiplied alpha Subtract blend pipeline.
    pub pipeline_subtract: RenderPipeline,
    /// Premultiplied alpha Screen blend pipeline.
    pub pipeline_screen: RenderPipeline,
    /// Premultiplied alpha Multiply blend pipeline.
    pub pipeline_multiply: RenderPipeline,
    /// Premultiplied alpha Lighten blend pipeline.
    pub pipeline_lighten: RenderPipeline,
    /// Premultiplied alpha Darken blend pipeline.
    pub pipeline_darken: RenderPipeline,
    /// Shared unit quad vertex buffer.
    pub vertex_buffer: Buffer,
    /// Shared unit quad index buffer.
    pub index_buffer: Buffer,
    /// Global uniform buffer.
    pub global_buffer: Buffer,
    /// Global uniform bind group layout.
    pub global_bind_group_layout: BindGroupLayout,
    /// Global bind group.
    pub global_bind_group: BindGroup,
    /// Layer uniform bind group layout.
    pub layer_bind_group_layout: BindGroupLayout,
    /// Texture bind group layout.
    pub texture_bind_group_layout: BindGroupLayout,
    /// Default linear clamp sampler.
    pub linear_sampler: Sampler,
    /// 1x1 solid white dummy texture.
    pub white_texture: Texture,
    /// View of 1x1 white texture.
    pub white_texture_view: TextureView,
    /// Default bind group for 1x1 white texture.
    pub white_bind_group: BindGroup,
}

impl CompositorPipeline {
    /// Initialize compositor render pipeline for the specified output target format.
    pub fn new(device: &Device, queue: &Queue, target_format: TextureFormat) -> Self {
        let shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("CompositorShader"),
            source: ShaderSource::Wgsl(include_str!("shaders.wgsl").into()),
        });

        // Bind group 0: GlobalUniforms
        let global_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("GlobalBindGroupLayout"),
                entries: &[BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let global_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("GlobalUniformBuffer"),
            contents: bytemuck::cast_slice(&[GlobalUniforms {
                view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            }]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let global_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GlobalBindGroup"),
            layout: &global_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: global_buffer.as_entire_binding(),
            }],
        });

        // Bind group 1: LayerUniforms
        let layer_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("LayerBindGroupLayout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Bind group 2: Texture + Sampler
        let texture_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("TextureBindGroupLayout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            multisampled: false,
                            view_dimension: TextureViewDimension::D2,
                            sample_type: TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("CompositorPipelineLayout"),
            bind_group_layouts: &[
                &global_bind_group_layout,
                &layer_bind_group_layout,
                &texture_bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let make_pipeline = |label: &'static str, blend: BlendState| {
            device.create_render_pipeline(&RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&pipeline_layout),
                vertex: VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[Vertex::desc()],
                    compilation_options: PipelineCompilationOptions::default(),
                },
                fragment: Some(FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(ColorTargetState {
                        format: target_format,
                        blend: Some(blend),
                        write_mask: ColorWrites::ALL,
                    })],
                    compilation_options: PipelineCompilationOptions::default(),
                }),
                primitive: PrimitiveState {
                    topology: PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: None,
                multisample: MultisampleState::default(),
                multiview: None,
                cache: None,
            })
        };

        // 7 fixed-function premultiplied-alpha blend pipelines:
        let pipeline_normal = make_pipeline(
            "CompositorRenderPipelineNormal",
            BlendState::PREMULTIPLIED_ALPHA_BLENDING,
        );

        let pipeline_add = make_pipeline(
            "CompositorRenderPipelineAdd",
            BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            },
        );

        let pipeline_subtract = make_pipeline(
            "CompositorRenderPipelineSubtract",
            BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::ReverseSubtract,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            },
        );

        let pipeline_screen = make_pipeline(
            "CompositorRenderPipelineScreen",
            BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrc,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
            },
        );

        let pipeline_multiply = make_pipeline(
            "CompositorRenderPipelineMultiply",
            BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::Dst,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
            },
        );

        let pipeline_lighten = make_pipeline(
            "CompositorRenderPipelineLighten",
            BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Max,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Max,
                },
            },
        );

        let pipeline_darken = make_pipeline(
            "CompositorRenderPipelineDarken",
            BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Min,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Min,
                },
            },
        );

        let pipeline = pipeline_normal.clone();

        // Quad buffers
        let vertex_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("QuadVertexBuffer"),
            contents: bytemuck::cast_slice(QUAD_VERTICES),
            usage: BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("QuadIndexBuffer"),
            contents: bytemuck::cast_slice(QUAD_INDICES),
            usage: BufferUsages::INDEX,
        });

        // 1x1 White Texture
        let white_texture = device.create_texture(&TextureDescriptor {
            label: Some("1x1WhiteTexture"),
            size: Extent3d {
                width: 1,
                height: 1,
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
            wgpu::TexelCopyTextureInfo {
                texture: &white_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: TextureAspect::All,
            },
            &[255, 255, 255, 255],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );

        let white_texture_view = white_texture.create_view(&TextureViewDescriptor::default());

        let linear_sampler = device.create_sampler(&SamplerDescriptor {
            label: Some("LinearSampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let white_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("WhiteTextureBindGroup"),
            layout: &texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&white_texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&linear_sampler),
                },
            ],
        });

        Self {
            pipeline,
            pipeline_normal,
            pipeline_add,
            pipeline_subtract,
            pipeline_screen,
            pipeline_multiply,
            pipeline_lighten,
            pipeline_darken,
            vertex_buffer,
            index_buffer,
            global_buffer,
            global_bind_group_layout,
            global_bind_group,
            layer_bind_group_layout,
            texture_bind_group_layout,
            linear_sampler,
            white_texture,
            white_texture_view,
            white_bind_group,
        }
    }

    /// Select appropriate render pipeline for a given BlendMode.
    pub fn get_pipeline(&self, mode: BlendMode) -> &RenderPipeline {
        match mode {
            BlendMode::Normal => &self.pipeline_normal,
            BlendMode::Add => &self.pipeline_add,
            BlendMode::Subtract => &self.pipeline_subtract,
            BlendMode::Screen => &self.pipeline_screen,
            BlendMode::Multiply => &self.pipeline_multiply,
            BlendMode::Lighten => &self.pipeline_lighten,
            BlendMode::Darken => &self.pipeline_darken,
            _ => &self.pipeline_normal,
        }
    }

    /// Update view-projection uniform matrix.
    pub fn update_global_matrix(&self, queue: &Queue, view_proj: glam::Mat4) {
        let uniforms = GlobalUniforms {
            view_proj: view_proj.to_cols_array_2d(),
        };
        queue.write_buffer(&self.global_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Create uniform buffer and bind group for an individual layer draw call.
    pub fn create_layer_bind_group(
        &self,
        device: &Device,
        uniforms: &LayerUniforms,
    ) -> (Buffer, BindGroup) {
        let buffer = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("LayerUniformBuffer"),
            contents: bytemuck::cast_slice(&[*uniforms]),
            usage: BufferUsages::UNIFORM,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LayerBindGroup"),
            layout: &self.layer_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        (buffer, bind_group)
    }

    /// Create texture bind group for a specific texture view.
    pub fn create_texture_bind_group(&self, device: &Device, view: &TextureView) -> BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CustomTextureBindGroup"),
            layout: &self.texture_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.linear_sampler),
                },
            ],
        })
    }

    /// Bind the standard quad mesh.
    pub fn bind_quad<'a>(&'a self, rpass: &mut RenderPass<'a>) {
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        rpass.set_index_buffer(self.index_buffer.slice(..), IndexFormat::Uint16);
    }
}
