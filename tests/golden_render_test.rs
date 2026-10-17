//! Golden image render acceptance tests for 1080p broadcast compositions.

use pollster::FutureExt;
use std::path::{Path, PathBuf};
use zeno_core::{AssetId, Color, LayerId, Property, Vec2, Vec3};
use zeno_engine::{
    Composition, Fill, GpuContext, Layer, LayerContent, ShapeContent, ShapeGeometry, TextContent,
    ZenoRenderer,
};

/// Helper to generate a 64x64 RGBA8 test checkerboard pattern.
fn make_test_image(width: u32, height: u32) -> Vec<u8> {
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let is_white = ((x / 8) + (y / 8)) % 2 == 0;
            if is_white {
                pixels.extend_from_slice(&[76, 141, 255, 255]); // Accent blue (#4C8DFF)
            } else {
                pixels.extend_from_slice(&[229, 72, 77, 255]); // Live red (#E5484D)
            }
        }
    }
    pixels
}

/// Computes root-mean-square error per pixel channel between two RGBA images.
fn compare_images(actual: &image::RgbaImage, expected: &image::RgbaImage) -> (f64, u32) {
    assert_eq!(
        actual.dimensions(),
        expected.dimensions(),
        "Image dimensions mismatch"
    );

    let mut total_diff_sq = 0.0;
    let mut max_diff: u32 = 0;
    let count = (actual.width() * actual.height() * 4) as f64;

    for (p1, p2) in actual.pixels().zip(expected.pixels()) {
        for c in 0..4 {
            let diff = (p1[c] as i32 - p2[c] as i32).unsigned_abs();
            if diff > max_diff {
                max_diff = diff;
            }
            total_diff_sq += (diff * diff) as f64;
        }
    }

    let rmse = (total_diff_sq / count).sqrt();
    (rmse, max_diff)
}

#[test]
fn test_m2_sample_composition_renders_1080p() {
    let gpu = GpuContext::init_headless()
        .block_on()
        .expect("GPU context initialization for headless rendering");

    let mut renderer =
        ZenoRenderer::new(&gpu.device, &gpu.queue, wgpu::TextureFormat::Rgba8UnormSrgb);

    // 1. Register test logo asset (64x64)
    let logo_asset_id = AssetId::new();
    let logo_bytes = make_test_image(64, 64);
    renderer.register_image_asset(&gpu.device, &gpu.queue, logo_asset_id, 64, 64, &logo_bytes);

    // 2. Build 1080p composition with shapes, images, text, and parenting
    let mut comp = Composition::new(1920, 1080, 60.0, 5.0);

    // Layer 1: Background dark studio canvas
    let bg_id = LayerId::new();
    let mut bg_layer = Layer::new(
        bg_id,
        "Background Canvas",
        LayerContent::Shape(ShapeContent {
            geometry: ShapeGeometry::Rectangle {
                size: Property::constant(Vec2::new(1920.0, 1080.0)),
            },
            fill: Some(Fill::Solid(Property::constant(
                Color::from_hex("#0E0F11FF").unwrap(),
            ))),
            stroke: None,
        }),
    );
    bg_layer.transform.position = Property::constant(Vec3::ZERO);
    comp.layers.push(bg_layer);

    // Layer 2: Lower-third container bar (parent)
    let bar_id = LayerId::new();
    let mut bar_layer = Layer::new(
        bar_id,
        "Lower Third Bar",
        LayerContent::Shape(ShapeContent {
            geometry: ShapeGeometry::Rectangle {
                size: Property::constant(Vec2::new(720.0, 120.0)),
            },
            fill: Some(Fill::Solid(Property::constant(
                Color::from_hex("#1D2024FF").unwrap(),
            ))),
            stroke: None,
        }),
    );
    bar_layer.transform.position = Property::constant(Vec3::new(120.0, 860.0, 0.0));
    comp.layers.push(bar_layer);

    // Layer 3: Team logo badge (child of bar_layer)
    let logo_id = LayerId::new();
    let mut logo_layer = Layer::new(
        logo_id,
        "Team Logo Badge",
        LayerContent::Image {
            asset: logo_asset_id,
        },
    );
    logo_layer.parent = Some(bar_id);
    logo_layer.transform.position = Property::constant(Vec3::new(28.0, 28.0, 0.0));
    comp.layers.push(logo_layer);

    // Layer 4: Title text (child of bar_layer)
    let title_id = LayerId::new();
    let mut title_layer = Layer::new(
        title_id,
        "Tournament Title",
        LayerContent::Text(TextContent {
            text: Property::constant("ZENOCAST ESPORTS".to_string()),
            font_family: "Inter".to_string(),
            font_size: Property::constant(28.0),
            font_weight: 700,
            color: Property::constant(Color::from_hex("#E7E9ECFF").unwrap()),
            ..Default::default()
        }),
    );
    title_layer.parent = Some(bar_id);
    title_layer.transform.position = Property::constant(Vec3::new(110.0, 20.0, 0.0));
    comp.layers.push(title_layer);

    // Layer 5: Subtitle text (child of bar_layer)
    let subtitle_id = LayerId::new();
    let mut subtitle_layer = Layer::new(
        subtitle_id,
        "Match Subtitle",
        LayerContent::Text(TextContent {
            text: Property::constant("GRAND FINALS - MAP 1".to_string()),
            font_family: "JetBrains Mono".to_string(),
            font_size: Property::constant(18.0),
            font_weight: 400,
            color: Property::constant(Color::from_hex("#4C8DFFFF").unwrap()),
            ..Default::default()
        }),
    );
    subtitle_layer.parent = Some(bar_id);
    subtitle_layer.transform.position = Property::constant(Vec3::new(110.0, 68.0, 0.0));
    comp.layers.push(subtitle_layer);

    // 3. Render frame to in-memory 1080p image
    let rendered_image = renderer.render_to_image(&gpu.device, &gpu.queue, &comp, 0.0, 1920, 1080);

    assert_eq!(rendered_image.width(), 1920);
    assert_eq!(rendered_image.height(), 1080);

    // Verify non-empty pixels
    let non_zero_count = rendered_image
        .pixels()
        .filter(|p| p[0] != 0 || p[1] != 0 || p[2] != 0)
        .count();
    assert!(
        non_zero_count > 100_000,
        "Expected substantial pixel coverage, found {}",
        non_zero_count
    );

    // 4. Golden image check
    let golden_path = if Path::new("tests/golden/m2_sample_1080p.png").exists() {
        PathBuf::from("tests/golden/m2_sample_1080p.png")
    } else if Path::new("../../tests/golden/m2_sample_1080p.png").exists() {
        PathBuf::from("../../tests/golden/m2_sample_1080p.png")
    } else {
        let golden_dir = Path::new("tests/golden");
        let _ = std::fs::create_dir_all(golden_dir);
        golden_dir.join("m2_sample_1080p.png")
    };

    if !golden_path.exists() {
        rendered_image
            .save(&golden_path)
            .expect("save initial golden image");
        println!(
            "Generated initial golden image at: {}",
            golden_path.display()
        );
    } else {
        let expected_image = image::open(&golden_path)
            .expect("open golden image")
            .to_rgba8();

        let (rmse, max_diff) = compare_images(&rendered_image, &expected_image);
        println!(
            "Golden comparison: RMSE = {:.4}, max channel diff = {}",
            rmse, max_diff
        );

        // Allow up to small GPU float tolerance across hardware/drivers
        assert!(
            rmse < 2.0,
            "Rendered output deviated from golden image: RMSE = {}",
            rmse
        );
        assert!(
            max_diff <= 5,
            "Max pixel channel difference exceeded tolerance: {}",
            max_diff
        );
    }
}
