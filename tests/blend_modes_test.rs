//! Acceptance test for premultiplied-alpha blend modes and blending methods.

use pollster::FutureExt;
use zeno_core::{BlendingMethod, Color, LayerId, Property, Vec2, Vec3};
use zeno_engine::{
    BlendMode, Composition, Fill, GpuContext, Layer, LayerContent, ShapeContent, ShapeGeometry,
    ZenoRenderer,
};

fn create_blend_test_composition(
    blend_mode: BlendMode,
    blending_method: BlendingMethod,
) -> Composition {
    let mut comp = Composition::new(200, 200, 60.0, 1.0);

    // Layer 1 (Bottom): Solid Blue rectangle at (0, 0), size (150, 150)
    let bottom_id = LayerId::new();
    let mut bottom_layer = Layer::new(
        bottom_id,
        "Bottom Blue Layer",
        LayerContent::Shape(ShapeContent {
            geometry: ShapeGeometry::Rectangle {
                size: Property::constant(Vec2::new(150.0, 150.0)),
            },
            fill: Some(Fill::Solid(Property::constant(Color::rgba(
                0.0, 0.0, 1.0, 0.8,
            )))),
            stroke: None,
        }),
    );
    bottom_layer.transform.position = Property::constant(Vec3::new(0.0, 0.0, 0.0));
    bottom_layer.blend_mode = BlendMode::Normal;
    bottom_layer.blending_method = BlendingMethod::Linear;
    comp.layers.push(bottom_layer);

    // Layer 2 (Top): Solid Red rectangle at (50, 50), size (150, 150)
    let top_id = LayerId::new();
    let mut top_layer = Layer::new(
        top_id,
        "Top Red Layer",
        LayerContent::Shape(ShapeContent {
            geometry: ShapeGeometry::Rectangle {
                size: Property::constant(Vec2::new(150.0, 150.0)),
            },
            fill: Some(Fill::Solid(Property::constant(Color::rgba(
                1.0, 0.0, 0.0, 0.6,
            )))),
            stroke: None,
        }),
    );
    top_layer.transform.position = Property::constant(Vec3::new(50.0, 50.0, 0.0));
    top_layer.blend_mode = blend_mode;
    top_layer.blending_method = blending_method;
    comp.layers.push(top_layer);

    comp
}

#[test]
fn test_all_seven_blend_modes() {
    let gpu = GpuContext::init_headless()
        .block_on()
        .expect("GPU context initialization for headless rendering");

    let mut renderer =
        ZenoRenderer::new(&gpu.device, &gpu.queue, wgpu::TextureFormat::Rgba8UnormSrgb);

    let modes = [
        (BlendMode::Normal, "Normal"),
        (BlendMode::Add, "Add"),
        (BlendMode::Subtract, "Subtract"),
        (BlendMode::Screen, "Screen"),
        (BlendMode::Multiply, "Multiply"),
        (BlendMode::Lighten, "Lighten"),
        (BlendMode::Darken, "Darken"),
    ];

    for (mode, name) in modes {
        let comp = create_blend_test_composition(mode, BlendingMethod::Linear);
        let img = renderer.render_to_image(&gpu.device, &gpu.queue, &comp, 0.0, 200, 200);

        // 1. Non-overlapping bottom region at (25, 25): should be predominantly blue
        let p_bottom = img.get_pixel(25, 25);
        assert_eq!(p_bottom[0], 0, "{name}: expected 0 red in bottom region");
        assert!(
            p_bottom[2] > 180,
            "{name}: expected high blue in bottom region, got {}",
            p_bottom[2]
        );

        // 2. Overlap region at (100, 100): test distinct blend behaviors
        let p_overlap = img.get_pixel(100, 100);
        println!("{name} overlap RGBA at (100, 100) = {:?}", p_overlap);

        match mode {
            BlendMode::Normal => {
                // Over compositing: both red and blue present
                assert!(
                    p_overlap[0] > 100,
                    "Normal: red should be present, got {}",
                    p_overlap[0]
                );
                assert!(
                    p_overlap[2] > 50,
                    "Normal: blue should be attenuated but present, got {}",
                    p_overlap[2]
                );
            }
            BlendMode::Add => {
                // Additive: both colors summed at full unattenuated intensity
                assert!(
                    p_overlap[0] > 120,
                    "Add: red should be fully summed, got {}",
                    p_overlap[0]
                );
                assert!(
                    p_overlap[2] > 180,
                    "Add: blue should be fully preserved, got {}",
                    p_overlap[2]
                );
            }
            BlendMode::Subtract => {
                // ReverseSubtract: Dst - Src. Bottom blue has 0 red, so 0 - Red = 0.
                assert_eq!(
                    p_overlap[0], 0,
                    "Subtract: red should be subtracted to 0, got {}",
                    p_overlap[0]
                );
                assert!(
                    p_overlap[2] > 150,
                    "Subtract: blue dst should remain, got {}",
                    p_overlap[2]
                );
            }
            BlendMode::Screen => {
                // Screen: Src + Dst * (1 - Src). Both channels illuminated
                assert!(
                    p_overlap[0] > 100,
                    "Screen: red channel lit, got {}",
                    p_overlap[0]
                );
                assert!(
                    p_overlap[2] > 150,
                    "Screen: blue channel lit, got {}",
                    p_overlap[2]
                );
            }
            BlendMode::Multiply => {
                // Multiply: Src * Dst + Dst * (1 - A_src)
                // Red channel: Src_r * Dst_r(0) + Dst_r(0)*(1-A) = 0
                assert_eq!(
                    p_overlap[0], 0,
                    "Multiply: red should be 0 because dst red was 0, got {}",
                    p_overlap[0]
                );
                // Blue channel: Src_b(0)*Dst_b + Dst_b*(1-A_src) = Dst_b * 0.4
                assert!(
                    p_overlap[2] > 50 && p_overlap[2] <= 160,
                    "Multiply: blue should be attenuated by (1 - A_src), got {}",
                    p_overlap[2]
                );
            }
            BlendMode::Lighten => {
                // Max(Src, Dst): red = max(red, 0), blue = max(0, blue)
                assert!(p_overlap[0] > 120, "Lighten: max red, got {}", p_overlap[0]);
                assert!(
                    p_overlap[2] > 180,
                    "Lighten: max blue, got {}",
                    p_overlap[2]
                );
            }
            BlendMode::Darken => {
                // Min(Src, Dst): red = min(red, 0) = 0, blue = min(0, blue) = 0
                assert_eq!(
                    p_overlap[0], 0,
                    "Darken: min(red, 0) should be 0, got {}",
                    p_overlap[0]
                );
                assert_eq!(
                    p_overlap[2], 0,
                    "Darken: min(0, blue) should be 0, got {}",
                    p_overlap[2]
                );
            }
            _ => {}
        }
    }
}

#[test]
fn test_blending_method_linear_vs_srgb_off() {
    let gpu = GpuContext::init_headless()
        .block_on()
        .expect("GPU context initialization for headless rendering");

    let mut renderer =
        ZenoRenderer::new(&gpu.device, &gpu.queue, wgpu::TextureFormat::Rgba8UnormSrgb);

    // Render with Linear blending method
    let comp_linear = create_blend_test_composition(BlendMode::Normal, BlendingMethod::Linear);
    let _img_linear =
        renderer.render_to_image(&gpu.device, &gpu.queue, &comp_linear, 0.0, 200, 200);

    // Render with SrgbOff blending method
    let comp_srgb_off = create_blend_test_composition(BlendMode::Normal, BlendingMethod::SrgbOff);
    let _img_srgb_off =
        renderer.render_to_image(&gpu.device, &gpu.queue, &comp_srgb_off, 0.0, 200, 200);

    // In top-only region (175, 175):
    // Top layer has Color::rgba(1.0, 0.0, 0.0, 0.6)
    // In Linear: RGB is kept linear [1.0, 0.0, 0.0], premultiplied by 0.6 -> 0.6
    // In SrgbOff: RGB is de-linearized via pow(rgb, 1/2.2). For 1.0, 1.0^(1/2.2) is 1.0.
    //
    // Now let's test a midtone color where pow(x, 1/2.2) differs significantly:
    let make_midtone_comp = |method: BlendingMethod| {
        let mut comp = Composition::new(100, 100, 60.0, 1.0);
        let mut layer = Layer::new(
            LayerId::new(),
            "Midtone Layer",
            LayerContent::Shape(ShapeContent {
                geometry: ShapeGeometry::Rectangle {
                    size: Property::constant(Vec2::new(100.0, 100.0)),
                },
                fill: Some(Fill::Solid(Property::constant(Color::rgba(
                    0.5, 0.2, 0.8, 0.5,
                )))),
                stroke: None,
            }),
        );
        layer.transform.position = Property::constant(Vec3::new(0.0, 0.0, 0.0));
        layer.blending_method = method;
        comp.layers.push(layer);
        comp
    };

    let img_midtone_linear = renderer.render_to_image(
        &gpu.device,
        &gpu.queue,
        &make_midtone_comp(BlendingMethod::Linear),
        0.0,
        100,
        100,
    );
    let img_midtone_srgb_off = renderer.render_to_image(
        &gpu.device,
        &gpu.queue,
        &make_midtone_comp(BlendingMethod::SrgbOff),
        0.0,
        100,
        100,
    );

    let p_lin = img_midtone_linear.get_pixel(50, 50);
    let p_srgb = img_midtone_srgb_off.get_pixel(50, 50);

    // SrgbOff de-linearizes colors (pow(c, 1/2.2)) before alpha multiply, boosting midtones.
    // Linear red: 0.5 * 0.5 = 0.25 (approx 64 in 8-bit)
    // SrgbOff red: (0.5^(1/2.2)) * 0.5 = 0.7297 * 0.5 = 0.3649 (approx 93 in 8-bit)
    println!(
        "Midtone comparison: Linear RGBA = {:?}, SrgbOff RGBA = {:?}",
        p_lin, p_srgb
    );

    assert!(
        p_srgb[0] > p_lin[0] + 15,
        "SrgbOff red ({}) must be noticeably brighter than Linear red ({})",
        p_srgb[0],
        p_lin[0]
    );
    assert!(
        p_srgb[1] > p_lin[1] + 10,
        "SrgbOff green ({}) must be brighter than Linear green ({})",
        p_srgb[1],
        p_lin[1]
    );
    assert!(
        p_srgb[2] > p_lin[2] + 5,
        "SrgbOff blue ({}) must be brighter than Linear blue ({})",
        p_srgb[2],
        p_lin[2]
    );
}
