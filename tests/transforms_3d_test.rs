//! Integration tests for Milestone 13: 2.5D transforms, perspective camera,
//! depth sorting rules, and parallax motion.

use zeno_core::{Color, Keyframe, Property, Vec2, Vec3};
use zeno_engine::eval::evaluate_at;
use zeno_engine::{
    CameraSettings, Composition, Fill, Layer, LayerContent, ShapeContent, ShapeGeometry,
};

#[test]
fn test_card_flip_animation_3d() {
    let mut comp = Composition::new(1920, 1080, 60.0, 2.0);

    // 1. Add perspective camera layer
    let cam_id = zeno_core::LayerId::new();
    let cam_settings = CameraSettings::broadcast_default(1920, 1080);
    let mut cam_layer = Layer::new(cam_id, "Main Camera", LayerContent::Camera(cam_settings));
    cam_layer.is_3d = true;
    comp.layers.push(cam_layer);

    // 2. Add 3D card layer that flips 180 degrees on Y axis
    let card_id = zeno_core::LayerId::new();
    let mut card_layer = Layer::new(
        card_id,
        "Player Card",
        LayerContent::Shape(ShapeContent {
            geometry: ShapeGeometry::Rectangle {
                size: Property::constant(Vec2::new(400.0, 250.0)),
            },
            fill: Some(Fill::Solid(Property::constant(Color::rgba(
                0.2, 0.4, 0.8, 1.0,
            )))),
            stroke: None,
        }),
    );
    card_layer.is_3d = true;
    card_layer.transform.position = Property::constant(Vec3::new(960.0, 540.0, 0.0));
    card_layer.transform.anchor_point = Property::constant(Vec3::new(200.0, 125.0, 0.0));

    // Keyframe Y-rotation: 0 deg at t=0.0s -> 90 deg at t=0.5s -> 180 deg at t=1.0s
    let rot_keys = vec![
        Keyframe::linear(0.0, Vec3::new(0.0, 0.0, 0.0)),
        Keyframe::linear(0.5, Vec3::new(0.0, 90.0, 0.0)),
        Keyframe::linear(1.0, Vec3::new(0.0, 180.0, 0.0)),
    ];
    card_layer.transform.rotation_degrees = Property::keyframes(rot_keys);
    comp.layers.push(card_layer);

    // Evaluate at t = 0.0s (Facing front)
    let frame_0 = evaluate_at(&comp, 0.0);
    assert!(frame_0.is_perspective);
    let resolved_card_0 = frame_0.layers.iter().find(|l| l.id == card_id).unwrap();
    // At 0 deg, local X axis points along world +X
    let x_axis_0 = resolved_card_0.world_matrix.transform_vector3(Vec3::X);
    assert!((x_axis_0.x - 1.0).abs() < 1e-4);
    assert!(x_axis_0.z.abs() < 1e-4);

    // Evaluate at t = 0.5s (Edge-on: 90 degrees)
    let frame_half = evaluate_at(&comp, 0.5);
    let resolved_card_half = frame_half.layers.iter().find(|l| l.id == card_id).unwrap();
    // At 90 deg Y rotation, local X axis points along world Z axis (edge-on to camera)
    let x_axis_half = resolved_card_half.world_matrix.transform_vector3(Vec3::X);
    assert!(x_axis_half.x.abs() < 1e-4);
    assert!((x_axis_half.z.abs() - 1.0).abs() < 1e-4);

    // Evaluate at t = 1.0s (Flipped: 180 degrees)
    let frame_1 = evaluate_at(&comp, 1.0);
    let resolved_card_1 = frame_1.layers.iter().find(|l| l.id == card_id).unwrap();
    // At 180 deg Y rotation, local X axis points along world -X (inverted)
    let x_axis_1 = resolved_card_1.world_matrix.transform_vector3(Vec3::X);
    assert!((x_axis_1.x - (-1.0)).abs() < 1e-4);
}

#[test]
fn test_parallax_lower_third_with_camera_move() {
    let mut comp = Composition::new(1920, 1080, 60.0, 2.0);

    // Broadcast default camera distance: d = (1080 / 2) / tan(30 deg) ≈ 935.3 px
    // Lateral camera move from X = 960 to X = 1160 over 2.0 seconds
    let cam_id = zeno_core::LayerId::new();
    let mut cam_settings = CameraSettings::broadcast_default(1920, 1080);
    let cam_keys = vec![
        Keyframe::linear(0.0, Vec3::new(960.0, 540.0, -935.3074)),
        Keyframe::linear(2.0, Vec3::new(1160.0, 540.0, -935.3074)),
    ];
    let tgt_keys = vec![
        Keyframe::linear(0.0, Vec3::new(960.0, 540.0, 0.0)),
        Keyframe::linear(2.0, Vec3::new(1160.0, 540.0, 0.0)),
    ];
    cam_settings.position = Property::keyframes(cam_keys);
    cam_settings.target = Property::keyframes(tgt_keys);

    let mut cam_layer = Layer::new(cam_id, "Camera", LayerContent::Camera(cam_settings));
    cam_layer.is_3d = true;
    comp.layers.push(cam_layer);

    // 1. Far background layer at Z = +1000.0
    let bg_id = zeno_core::LayerId::new();
    let mut bg = Layer::new(
        bg_id,
        "Stadium Background",
        LayerContent::Shape(ShapeContent {
            geometry: ShapeGeometry::Rectangle {
                size: Property::constant(Vec2::new(1920.0, 1080.0)),
            },
            fill: Some(Fill::Solid(Property::constant(Color::rgba(
                0.5, 0.5, 0.5, 1.0,
            )))),
            stroke: None,
        }),
    );
    bg.is_3d = true;
    bg.transform.position = Property::constant(Vec3::new(960.0, 540.0, 1000.0));
    comp.layers.push(bg);

    // 2. Midground Lower-Third Bar at Z = 0.0
    let bar_id = zeno_core::LayerId::new();
    let mut bar = Layer::new(
        bar_id,
        "Lower Third Base",
        LayerContent::Shape(ShapeContent {
            geometry: ShapeGeometry::Rectangle {
                size: Property::constant(Vec2::new(800.0, 120.0)),
            },
            fill: Some(Fill::Solid(Property::constant(Color::rgba(
                0.0, 0.2, 0.8, 1.0,
            )))),
            stroke: None,
        }),
    );
    bar.is_3d = true;
    bar.transform.position = Property::constant(Vec3::new(960.0, 800.0, 0.0));
    comp.layers.push(bar);

    // 3. Foreground Badge at Z = -400.0 (closer to camera)
    let fg_id = zeno_core::LayerId::new();
    let mut fg = Layer::new(
        fg_id,
        "Team Badge Popout",
        LayerContent::Shape(ShapeContent {
            geometry: ShapeGeometry::Rectangle {
                size: Property::constant(Vec2::new(120.0, 120.0)),
            },
            fill: Some(Fill::Solid(Property::constant(Color::rgba(
                0.9, 0.1, 0.1, 1.0,
            )))),
            stroke: None,
        }),
    );
    fg.is_3d = true;
    fg.transform.position = Property::constant(Vec3::new(960.0, 800.0, -400.0));
    comp.layers.push(fg);

    // Evaluate at t = 0.0s
    let frame_0 = evaluate_at(&comp, 0.0);
    assert_eq!(frame_0.layers.len(), 4); // camera + 3 shapes

    // Verify depth sorting: back-to-front rendering order
    // Background (Z=+1000, dist~1935px) -> Midground (Z=0, dist~935px) -> Foreground (Z=-400, dist~535px)
    let visible_layers: Vec<_> = frame_0.layers.iter().filter(|l| l.id != cam_id).collect();

    assert_eq!(visible_layers[0].id, bg_id, "Background must render first");
    assert_eq!(visible_layers[1].id, bar_id, "Midground must render second");
    assert_eq!(visible_layers[2].id, fg_id, "Foreground must render on top");

    // Evaluate at t = 2.0s (Camera shifted laterally by +200 px)
    let frame_2 = evaluate_at(&comp, 2.0);
    let view_0 = frame_0.camera_view.unwrap();
    let view_2 = frame_2.camera_view.unwrap();

    // Compute apparent screen displacement in view space
    let bg_pos = Vec3::new(960.0, 540.0, 1000.0);
    let fg_pos = Vec3::new(960.0, 800.0, -400.0);

    let bg_cam_0 = view_0.transform_point3(bg_pos);
    let bg_cam_2 = view_2.transform_point3(bg_pos);
    let bg_disp = (bg_cam_2.x - bg_cam_0.x).abs();

    let fg_cam_0 = view_0.transform_point3(fg_pos);
    let fg_cam_2 = view_2.transform_point3(fg_pos);
    let fg_disp = (fg_cam_2.x - fg_cam_0.x).abs();

    // Perspective projection foreshortens distant objects:
    // Projected X on screen = X_cam / (Z_cam * tan(fov/2))
    let bg_screen_disp = bg_disp / bg_cam_0.z;
    let fg_screen_disp = fg_disp / fg_cam_0.z;

    // Foreground badge moves significantly more across the screen than the far stadium background
    assert!(
        fg_screen_disp > bg_screen_disp * 2.5,
        "Foreground parallax displacement ({:.4}) must be significantly larger than background ({:.4})",
        fg_screen_disp, bg_screen_disp
    );
}

#[test]
fn test_depth_sorting_rules_with_tie_breaker() {
    let mut comp = Composition::new(1920, 1080, 60.0, 1.0);

    // Two 2D layers at identical depth (Z = 0)
    let id_1 = zeno_core::LayerId::new();
    let mut layer_1 = Layer::new(id_1, "Layer 1 Bottom", LayerContent::Null);
    layer_1.is_3d = true;
    layer_1.transform.position = Property::constant(Vec3::new(100.0, 100.0, 0.0));
    comp.layers.push(layer_1);

    let id_2 = zeno_core::LayerId::new();
    let mut layer_2 = Layer::new(id_2, "Layer 2 Top", LayerContent::Null);
    layer_2.is_3d = true;
    layer_2.transform.position = Property::constant(Vec3::new(100.0, 100.0, 0.0));
    comp.layers.push(layer_2);

    // Layer 3 behind both at Z = 50.0
    let id_3 = zeno_core::LayerId::new();
    let mut layer_3 = Layer::new(id_3, "Layer 3 Behind", LayerContent::Null);
    layer_3.is_3d = true;
    layer_3.transform.position = Property::constant(Vec3::new(100.0, 100.0, 50.0));
    comp.layers.push(layer_3);

    let frame = evaluate_at(&comp, 0.0);

    // Depth sorting order:
    // 1. Layer 3 (Z = 50.0, farthest)
    // 2. Layer 1 (Z = 0.0, original index 0)
    // 3. Layer 2 (Z = 0.0, original index 1, tie broken by composition layer order)
    assert_eq!(frame.layers[0].id, id_3);
    assert_eq!(frame.layers[1].id, id_1);
    assert_eq!(frame.layers[2].id, id_2);
}
