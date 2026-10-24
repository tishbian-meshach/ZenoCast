//! WGPU pipelines and WGSL shaders for scene item composition and transitions.

use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::*;

/// Uniform buffer payload for a single composited scene item.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ItemUniforms {
    /// Combined model-view-projection matrix mapping item coordinates to NDC.
    pub mvp: [[f32; 4]; 4],
    /// Normalized crop insets: [left, top, right, bottom] in [0.0..1.0].
    pub crop_insets: [f32; 4],
    /// Layer opacity [0.0..1.0].
    pub opacity: f32,
    /// Blend mode enumeration (0 = Normal, 1 = Add, 2 = Multiply, 3 = Screen).
    pub blend_mode: u32,
    /// 16-byte alignment padding.
    pub _padding: [f32; 2],
}

/// Uniform buffer payload for GPU transition shader passes.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct TransitionUniforms {
    /// Eased transition progress in [0.0..1.0].
    pub progress: f32,
    /// Transition kind (0=Cut, 1=Fade, 2=DipToColor, 3=Slide, 4=Wipe, 5=Zoom, 6=Stinger).
    pub kind: u32,
    /// Direction identifier (0=Left, 1=Right, 2=Up, 3=Down for Slide; 0=H, 1=V, 2=Radial for Wipe).
    pub direction: u32,
    /// Specialized parameter (push flag, soft edge feathering, zoom direction, switch point).
    pub param1: f32,
    /// Dip target color [R, G, B, A].
    pub dip_color: [f32; 4],
    /// 16-byte alignment padding.
    pub _padding: [f32; 8],
}

const ITEM_SHADER_WGSL: &str = r#"
struct ItemUniforms {
    mvp: mat4x4<f32>,
    crop_insets: vec4<f32>,
    opacity: f32,
    blend_mode: u32,
    _pad0: f32,
    _pad1: f32,
};

@group(0) @binding(0) var<uniform> uniforms: ItemUniforms;
@group(0) @binding(1) var item_texture: texture_2d<f32>;
@group(0) @binding(2) var item_sampler: sampler;

struct VertexInput {
    @builtin(vertex_index) vertex_index: u32,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    // Unit quad vertices [-0.5..0.5] mapped to UV [0..1]
    var pos = array<vec2<f32>, 4>(
        vec2<f32>(-0.5, -0.5),
        vec2<f32>( 0.5, -0.5),
        vec2<f32>(-0.5,  0.5),
        vec2<f32>( 0.5,  0.5)
    );
    var uvs = array<vec2<f32>, 4>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 1.0)
    );

    let p = pos[in.vertex_index];
    out.clip_position = uniforms.mvp * vec4<f32>(p.x, p.y, 0.0, 1.0);
    out.uv = uvs[in.vertex_index];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Crop boundaries
    let left = uniforms.crop_insets.x;
    let top = uniforms.crop_insets.y;
    let right = 1.0 - uniforms.crop_insets.z;
    let bottom = 1.0 - uniforms.crop_insets.w;

    if (in.uv.x < left || in.uv.x > right || in.uv.y < top || in.uv.y > bottom) {
        discard;
    }

    var color = textureSample(item_texture, item_sampler, in.uv);
    color.a = color.a * uniforms.opacity;
    // Premultiply alpha for accurate compositing
    return vec4<f32>(color.rgb * color.a, color.a);
}
"#;

const TRANSITION_SHADER_WGSL: &str = r#"
struct TransitionUniforms {
    progress: f32,
    kind: u32,
    direction: u32,
    param1: f32,
    dip_color: vec4<f32>,
    _pad0: vec4<f32>,
    _pad1: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: TransitionUniforms;
@group(0) @binding(1) var tex_a: texture_2d<f32>;
@group(0) @binding(2) var tex_b: texture_2d<f32>;
@group(0) @binding(3) var trans_sampler: sampler;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;
    // Fullscreen triangle covering [-1..1] NDC
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0)
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(2.0, 0.0),
        vec2<f32>(0.0, 2.0)
    );

    out.clip_position = vec4<f32>(pos[vertex_index], 0.0, 1.0);
    out.uv = uvs[vertex_index];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv = in.uv;
    let t = clamp(u.progress, 0.0, 1.0);

    // Kind 0: Cut
    if (u.kind == 0u) {
        if (t >= 1.0) {
            return textureSample(tex_b, trans_sampler, uv);
        } else {
            return textureSample(tex_a, trans_sampler, uv);
        }
    }

    // Kind 1: Fade (cross-dissolve)
    if (u.kind == 1u) {
        let col_a = textureSample(tex_a, trans_sampler, uv);
        let col_b = textureSample(tex_b, trans_sampler, uv);
        return mix(col_a, col_b, t);
    }

    // Kind 2: Dip to Color
    if (u.kind == 2u) {
        let col_a = textureSample(tex_a, trans_sampler, uv);
        let col_b = textureSample(tex_b, trans_sampler, uv);
        if (t < 0.5) {
            return mix(col_a, u.dip_color, t * 2.0);
        } else {
            return mix(u.dip_color, col_b, (t - 0.5) * 2.0);
        }
    }

    // Kind 3: Slide (Push or Cover in 4 directions)
    if (u.kind == 3u) {
        let is_push = u.param1 > 0.5;
        var offset = vec2<f32>(0.0, 0.0);
        if (u.direction == 0u) { // Left
            offset = vec2<f32>(-t, 0.0);
        } else if (u.direction == 1u) { // Right
            offset = vec2<f32>(t, 0.0);
        } else if (u.direction == 2u) { // Up
            offset = vec2<f32>(0.0, -t);
        } else { // Down
            offset = vec2<f32>(0.0, t);
        }

        if (is_push) {
            let uv_a = uv - offset;
            let uv_b = uv - (offset + normalize(offset));
            if (uv_a.x >= 0.0 && uv_a.x <= 1.0 && uv_a.y >= 0.0 && uv_a.y <= 1.0) {
                return textureSample(tex_a, trans_sampler, uv_a);
            } else if (uv_b.x >= 0.0 && uv_b.x <= 1.0 && uv_b.y >= 0.0 && uv_b.y <= 1.0) {
                return textureSample(tex_b, trans_sampler, uv_b);
            }
            return vec4<f32>(0.0, 0.0, 0.0, 0.0);
        } else {
            // Cover
            var uv_b = uv;
            if (u.direction == 0u) { uv_b.x = uv.x + (1.0 - t); }
            else if (u.direction == 1u) { uv_b.x = uv.x - (1.0 - t); }
            else if (u.direction == 2u) { uv_b.y = uv.y + (1.0 - t); }
            else { uv_b.y = uv.y - (1.0 - t); }

            if (uv_b.x >= 0.0 && uv_b.x <= 1.0 && uv_b.y >= 0.0 && uv_b.y <= 1.0) {
                return textureSample(tex_b, trans_sampler, uv_b);
            } else {
                return textureSample(tex_a, trans_sampler, uv);
            }
        }
    }

    // Kind 4: Wipe (Horizontal, Vertical, Radial with soft edge)
    if (u.kind == 4u) {
        let soft = max(u.param1, 0.001);
        var dist = 0.0;
        if (u.direction == 0u) { // Horizontal
            dist = uv.x;
        } else if (u.direction == 1u) { // Vertical
            dist = uv.y;
        } else { // Radial
            dist = distance(uv, vec2<f32>(0.5, 0.5)) * 1.4142;
        }

        let edge = dist - t;
        let factor = smoothstep(-soft, soft, edge);
        let col_a = textureSample(tex_a, trans_sampler, uv);
        let col_b = textureSample(tex_b, trans_sampler, uv);
        return mix(col_b, col_a, factor);
    }

    // Kind 5: Zoom
    if (u.kind == 5u) {
        let col_a = textureSample(tex_a, trans_sampler, uv);
        let scale_b = max(t, 0.001);
        let uv_b = (uv - vec2<f32>(0.5, 0.5)) / scale_b + vec2<f32>(0.5, 0.5);
        if (uv_b.x >= 0.0 && uv_b.x <= 1.0 && uv_b.y >= 0.0 && uv_b.y <= 1.0) {
            let col_b = textureSample(tex_b, trans_sampler, uv_b);
            return mix(col_a, col_b, t);
        } else {
            return col_a;
        }
    }

    // Kind 6: Stinger / Overlay switch point fallback
    let switch_point = u.param1;
    let col_a = textureSample(tex_a, trans_sampler, uv);
    let col_b = textureSample(tex_b, trans_sampler, uv);
    if (t < switch_point) {
        return col_a;
    } else {
        return col_b;
    }
}
"#;

/// GPU rendering pipelines for scene item compositing and transition shaders.
pub struct CompositorGpuPipeline {
    pub item_pipeline: RenderPipeline,
    pub item_bind_group_layout: BindGroupLayout,
    pub transition_pipeline: RenderPipeline,
    pub transition_bind_group_layout: BindGroupLayout,
    pub sampler: Arc<Sampler>,
}

impl CompositorGpuPipeline {
    /// Compiles shaders and initializes pipeline states for DX12/Vulkan.
    pub fn new(device: &Device) -> Self {
        // Sampler with clamp-to-edge
        let sampler = Arc::new(device.create_sampler(&SamplerDescriptor {
            label: Some("CompositorSampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        }));

        // Item compositing pipeline
        let item_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("SceneItemShader"),
            source: ShaderSource::Wgsl(ITEM_SHADER_WGSL.into()),
        });

        let item_bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("SceneItemBindGroupLayout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let item_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("SceneItemPipelineLayout"),
            bind_group_layouts: &[&item_bind_group_layout],
            push_constant_ranges: &[],
        });

        let item_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("SceneItemPipeline"),
            layout: Some(&item_pipeline_layout),
            vertex: VertexState {
                module: &item_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &item_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb,
                    blend: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        // Transition shader pipeline
        let transition_shader = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("TransitionShader"),
            source: ShaderSource::Wgsl(TRANSITION_SHADER_WGSL.into()),
        });

        let transition_bind_group_layout =
            device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("TransitionBindGroupLayout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let transition_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("TransitionPipelineLayout"),
            bind_group_layouts: &[&transition_bind_group_layout],
            push_constant_ranges: &[],
        });

        let transition_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("TransitionPipeline"),
            layout: Some(&transition_pipeline_layout),
            vertex: VertexState {
                module: &transition_shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: Some(FragmentState {
                module: &transition_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(ColorTargetState {
                    format: TextureFormat::Rgba8UnormSrgb,
                    blend: Some(BlendState::REPLACE),
                    write_mask: ColorWrites::ALL,
                })],
                compilation_options: PipelineCompilationOptions::default(),
            }),
            primitive: PrimitiveState {
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Self {
            item_pipeline,
            item_bind_group_layout,
            transition_pipeline,
            transition_bind_group_layout,
            sampler,
        }
    }
}
