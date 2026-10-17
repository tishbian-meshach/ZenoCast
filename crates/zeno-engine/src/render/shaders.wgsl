// Zenocast Core Compositor Shader (WGSL)

struct GlobalUniforms {
    view_proj: mat4x4<f32>,
};

struct LayerUniforms {
    world: mat4x4<f32>,
    color: vec4<f32>,
    params: vec4<f32>, // x: render_mode (0 = solid/rect, 1 = textured/image/text, 2 = linear_gradient), y: opacity (0..1), z: corner_radius, w: unused
    gradient_start: vec2<f32>,
    gradient_end: vec2<f32>,
    gradient_color_end: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> globals: GlobalUniforms;

@group(1) @binding(0)
var<uniform> layer: LayerUniforms;

@group(2) @binding(0)
var t_diffuse: texture_2d<f32>;

@group(2) @binding(1)
var s_diffuse: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) world_pos: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.uv = in.uv;
    
    let local_pos = vec4<f32>(in.position, 0.0, 1.0);
    let world_pos = layer.world * local_pos;
    out.world_pos = world_pos.xy;
    out.clip_position = globals.view_proj * world_pos;
    
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let mode = u32(round(layer.params.x));
    let opacity = layer.params.y;
    
    var base_color: vec4<f32>;
    
    if mode == 0u {
        // Solid color rectangle
        base_color = layer.color;
    } else if mode == 1u {
        // Textured image or rasterized text quad
        let sampled = textureSample(t_diffuse, s_diffuse, in.uv);
        // Modulate with layer tint color
        base_color = sampled * layer.color;
    } else if mode == 2u {
        // Linear gradient
        let start = layer.gradient_start;
        let end = layer.gradient_end;
        let dir = end - start;
        let len_sq = dot(dir, dir);
        var t = 0.0;
        if len_sq > 0.0001 {
            t = clamp(dot(in.uv - start, dir) / len_sq, 0.0, 1.0);
        }
        base_color = mix(layer.color, layer.gradient_color_end, t);
    } else {
        base_color = layer.color;
    }
    
    // Apply master layer opacity
    base_color.a = base_color.a * opacity;
    
    return base_color;
}
