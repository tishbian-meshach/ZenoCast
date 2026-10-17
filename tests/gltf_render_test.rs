//! M19 Acceptance Tests: glTF 3D Models with PBR Lighting and Animation Playback.
//!
//! Validates:
//! 1. 3D procedural esports crest / logo mesh generation with analytical surface normals.
//! 2. Binary GLB container header and chunk parsing.
//! 3. Deterministic animation playback and auto-rotation evaluation across timestamps.
//! 4. Full GPU rendering of a rotating 3D metallic logo overlay with PBR lighting into 1080p frame.

use glam::Vec3;
use pollster::block_on;
use zeno_core::{AssetId, Color, LayerId, Property};
use zeno_engine::eval::evaluate_at;
use zeno_engine::model::{CameraSettings, Composition, Layer, LayerContent, Model3DContent};
use zeno_engine::render::{GltfModel, GpuContext, ZenoRenderer};

#[test]
fn test_m19_procedural_logo_and_cube_geometry() {
    let logo = GltfModel::create_esports_logo();
    assert_eq!(logo.name, "EsportsTournamentLogo");
    assert!(!logo.vertices.is_empty());
    assert!(!logo.indices.is_empty());
    assert_eq!(
        logo.indices.len() % 3,
        0,
        "Indices must form complete triangles"
    );

    // Check normal vectors are non-zero and roughly unit length
    for v in &logo.vertices {
        let norm_len = (v.normal[0].powi(2) + v.normal[1].powi(2) + v.normal[2].powi(2)).sqrt();
        assert!(norm_len > 0.1, "Normal vector must be non-zero");
    }

    let cube = GltfModel::create_cube(100.0);
    assert_eq!(cube.name, "Cube3D");
    assert_eq!(cube.vertices.len(), 24); // 6 faces * 4 vertices
    assert_eq!(cube.indices.len(), 36); // 6 faces * 2 triangles * 3 indices
}

#[test]
fn test_m19_glb_parser() {
    // Construct a minimal valid binary GLB container
    let mut glb_bytes = Vec::new();

    // 1. Header (12 bytes)
    glb_bytes.extend_from_slice(&0x46546C67u32.to_le_bytes()); // "glTF"
    glb_bytes.extend_from_slice(&2u32.to_le_bytes()); // Version 2

    // 2. Chunk 0: JSON
    let json_str = r#"{"asset":{"version":"2.0"},"scenes":[{"nodes":[0]}],"nodes":[{"mesh":0}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}]}"#;
    let mut json_data = json_str.as_bytes().to_vec();
    while json_data.len() % 4 != 0 {
        json_data.push(b' '); // 4-byte alignment
    }

    let chunk0_len = json_data.len() as u32;
    let chunk0_type = 0x4E4F534Au32; // "JSON"

    // 3. Chunk 1: BIN
    let bin_data = vec![0u8; 64];
    let chunk1_len = bin_data.len() as u32;
    let chunk1_type = 0x004E4942u32; // "BIN"

    let total_len = 12 + 8 + chunk0_len + 8 + chunk1_len;
    glb_bytes.extend_from_slice(&total_len.to_le_bytes());

    // Write Chunk 0
    glb_bytes.extend_from_slice(&chunk0_len.to_le_bytes());
    glb_bytes.extend_from_slice(&chunk0_type.to_le_bytes());
    glb_bytes.extend_from_slice(&json_data);

    // Write Chunk 1
    glb_bytes.extend_from_slice(&chunk1_len.to_le_bytes());
    glb_bytes.extend_from_slice(&chunk1_type.to_le_bytes());
    glb_bytes.extend_from_slice(&bin_data);

    let parsed = GltfModel::parse_glb(&glb_bytes).expect("Failed to parse valid GLB container");
    assert!(!parsed.vertices.is_empty());
    assert!(!parsed.indices.is_empty());
}

#[test]
fn test_m19_pbr_evaluation_and_rotation_playback() {
    let mut comp = Composition::new(1920, 1080, 60.0, 5.0);

    let model_content = Model3DContent {
        auto_rotate_deg_per_sec: Property::constant(Vec3::new(0.0, 60.0, 0.0)), // 60 deg/sec
        metallic: Property::constant(0.8),
        roughness: Property::constant(0.15),
        base_color: Property::constant(Color::rgb(1.0, 0.85, 0.2)), // Gold
        ..Default::default()
    };

    let layer_id = LayerId::new();
    let mut layer = Layer::new(
        layer_id,
        "3D Rotating Trophy Logo",
        LayerContent::Model3D(model_content),
    );
    layer.is_3d = true;
    comp.layers.push(layer);

    // 1. Evaluate at t = 0.0s
    let frame_t0 = evaluate_at(&comp, 0.0);
    assert_eq!(frame_t0.layers.len(), 1);
    if let zeno_engine::eval::ResolvedContent::Model3D {
        rotation_degrees,
        metallic,
        roughness,
        ..
    } = &frame_t0.layers[0].content
    {
        assert_eq!(rotation_degrees.y, 0.0);
        assert!((metallic - 0.8).abs() < 1e-4);
        assert!((roughness - 0.15).abs() < 1e-4);
    } else {
        panic!("Expected ResolvedContent::Model3D");
    }

    // 2. Evaluate at t = 1.5s -> 60 deg/s * 1.5s = 90 degrees rotation
    let frame_t1_5 = evaluate_at(&comp, 1.5);
    if let zeno_engine::eval::ResolvedContent::Model3D {
        rotation_degrees, ..
    } = &frame_t1_5.layers[0].content
    {
        assert!(
            (rotation_degrees.y - 90.0).abs() < 1e-3,
            "Expected 90 deg rotation, got {}",
            rotation_degrees.y
        );
    } else {
        panic!("Expected ResolvedContent::Model3D");
    }
}

#[test]
fn test_m19_rotating_logo_renders_in_overlay() {
    let gpu = block_on(GpuContext::init_headless()).expect("GpuContext::init_headless failed");
    let mut renderer =
        ZenoRenderer::new(&gpu.device, &gpu.queue, wgpu::TextureFormat::Rgba8UnormSrgb);

    let mut comp = Composition::new(1920, 1080, 60.0, 5.0);

    // Camera layer
    let mut camera_settings = CameraSettings::broadcast_default(1920, 1080);
    camera_settings.position = Property::constant(Vec3::new(960.0, 540.0, -800.0));
    let camera_layer = Layer::new(
        LayerId::new(),
        "Broadcast Camera",
        LayerContent::Camera(camera_settings),
    );
    comp.layers.push(camera_layer);

    // 3D Rotating Logo Layer
    let model_asset_id = AssetId::new();
    let logo_model = GltfModel::create_esports_logo();
    renderer.register_model_asset(&gpu.device, model_asset_id, &logo_model);

    let model_content = Model3DContent {
        asset: model_asset_id,
        auto_rotate_deg_per_sec: Property::constant(Vec3::new(0.0, 45.0, 0.0)),
        metallic: Property::constant(0.9),
        roughness: Property::constant(0.1),
        base_color: Property::constant(Color::rgb(0.95, 0.8, 0.25)), // Broadcast gold
        ..Default::default()
    };

    let mut model_layer = Layer::new(
        LayerId::new(),
        "Championship Crest 3D",
        LayerContent::Model3D(model_content),
    );
    model_layer.is_3d = true;
    model_layer.transform.position = Property::constant(Vec3::new(960.0, 540.0, 0.0));
    model_layer.transform.scale_percent = Property::constant(Vec3::splat(150.0));
    comp.layers.push(model_layer);

    // Render frame at t = 1.0s (rotated 45 degrees)
    let frame = evaluate_at(&comp, 1.0);
    assert_eq!(frame.layers.len(), 2);

    let target_texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("RenderTarget1080p"),
        size: wgpu::Extent3d {
            width: 1920,
            height: 1080,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });

    let target_view = target_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let render_start = std::time::Instant::now();
    renderer.render_resolved_frame(&gpu.device, &gpu.queue, &frame, &target_view, 1920, 1080);
    let render_duration = render_start.elapsed();
    println!(
        "Rendered 1080p 3D Model Frame in {:.2}ms",
        render_duration.as_secs_f64() * 1000.0
    );

    // Readback rendered pixels to verify lit 3D model geometry
    let output_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ReadbackBuffer"),
        size: (1920 * 1080 * 4) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = gpu
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ReadbackEncoder"),
        });

    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &target_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(1920 * 4),
                rows_per_image: Some(1080),
            },
        },
        wgpu::Extent3d {
            width: 1920,
            height: 1080,
            depth_or_array_layers: 1,
        },
    );

    gpu.queue.submit(Some(encoder.finish()));

    let buffer_slice = output_buffer.slice(..);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
        tx.send(res).unwrap();
    });
    gpu.device.poll(wgpu::Maintain::Wait);
    rx.recv().unwrap().unwrap();

    let data = buffer_slice.get_mapped_range();
    let mut lit_pixels = 0usize;
    let mut max_brightness = 0u8;

    for chunk in data.chunks_exact(4) {
        let r = chunk[0];
        let g = chunk[1];
        let b = chunk[2];
        let a = chunk[3];
        if a > 10 {
            lit_pixels += 1;
            let brightness = r.max(g).max(b);
            if brightness > max_brightness {
                max_brightness = brightness;
            }
        }
    }

    drop(data);
    output_buffer.unmap();

    println!(
        "3D Model Render Validation: lit_pixels = {}, max_brightness = {}",
        lit_pixels, max_brightness
    );
    assert!(
        lit_pixels > 1000,
        "3D model should render visible geometry into the overlay"
    );
    assert!(
        max_brightness > 150,
        "PBR lighting should produce bright specular highlights on gold logo"
    );
}
