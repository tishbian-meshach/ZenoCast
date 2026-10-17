//! # zeno-engine
//!
//! Pure scene graph, animation evaluation, and render-list generation.
//!
//! In accordance with the engine dependency rule, this crate does not depend on
//! UI, windowing, device capture, or video encoding systems.

pub mod eval;
pub mod model;
pub mod render;

// Re-exports
pub use eval::{
    evaluate, evaluate_at, evaluate_with_clock, get_script_runtime, DataSnapshot,
    LifecycleController, LifecycleManager, PlaybackClock, PlaybackState, ResolvedContent,
    ResolvedFill, ResolvedFrame, ResolvedLayer, ResolvedShape, ResolvedShapeGeometry,
    ResolvedStroke, ResolvedText,
};
pub use model::{
    AnimationState, AssetKind, AssetRef, BlendMode, CameraSettings, Composition, Effect, Fill,
    GradientStop, Layer, LayerContent, Mask, MaskMode, ParticleEmitter, ShapeContent,
    ShapeGeometry, StatePlayback, Stroke, TextAlignment, TextContent, Transform3D,
};
pub use render::{
    CompositorPipeline, GpuContext, LayerUniforms, PooledTexture, RasterCache, TexturePool,
    ZenoRenderer,
};

#[cfg(test)]
mod tests {
    use super::*;
    use zeno_core::{Color, Keyframe, LayerId, Property, Vec2, Vec3};

    #[test]
    fn test_composition_serialization_roundtrip() {
        let mut comp = Composition::new(1920, 1080, 60.0, 5.0);
        let layer_id = LayerId::new();
        let mut layer = Layer::new(
            layer_id,
            "Title Text",
            LayerContent::Text(TextContent {
                text: Property::constant("Welcome to Zenocast".to_string()),
                font_family: "Inter".to_string(),
                font_size: Property::constant(48.0),
                color: Property::constant(Color::WHITE),
                ..Default::default()
            }),
        );
        layer.transform.position = Property::constant(Vec3::new(960.0, 540.0, 0.0));
        comp.layers.push(layer);

        let json = serde_json::to_string_pretty(&comp).expect("serialize composition");
        let deserialized: Composition =
            serde_json::from_str(&json).expect("deserialize composition");

        assert_eq!(comp.width, deserialized.width);
        assert_eq!(comp.height, deserialized.height);
        assert_eq!(comp.layers.len(), deserialized.layers.len());
        assert_eq!(comp.layers[0].name, deserialized.layers[0].name);
    }

    #[test]
    fn test_pure_evaluate_transform_keyframes() {
        let mut comp = Composition::new(1920, 1080, 60.0, 5.0);
        let layer_id = LayerId::new();
        let mut layer = Layer::new(
            layer_id,
            "Animated Box",
            LayerContent::Shape(ShapeContent {
                geometry: ShapeGeometry::Rectangle {
                    size: Property::constant(Vec2::new(100.0, 100.0)),
                },
                fill: Some(Fill::Solid(Property::constant(Color::WHITE))),
                stroke: None,
            }),
        );

        // Keyframe position from (0, 0, 0) at t=0s to (200, 400, 0) at t=2s
        layer.transform.position = Property::keyframes(vec![
            Keyframe::linear(0.0, Vec3::new(0.0, 0.0, 0.0)),
            Keyframe::linear(2.0, Vec3::new(200.0, 400.0, 0.0)),
        ]);
        // Keyframe opacity from 0% at t=0s to 100% at t=1s
        layer.transform.opacity_percent = Property::keyframes(vec![
            Keyframe::linear(0.0, 0.0f32),
            Keyframe::linear(1.0, 100.0f32),
        ]);

        comp.layers.push(layer);

        // Evaluate at t=0.0s
        let frame_0 = evaluate_at(&comp, 0.0);
        assert_eq!(frame_0.layers.len(), 1);
        assert_eq!(frame_0.layers[0].net_opacity, 0.0);

        // Evaluate at t=1.0s
        let frame_1 = evaluate_at(&comp, 1.0);
        assert_eq!(frame_1.layers[0].net_opacity, 1.0);
        // Translation in column 3 of world_matrix
        let col3_1 = frame_1.layers[0].world_matrix.col(3);
        assert!((col3_1.x - 100.0).abs() < 1e-4);
        assert!((col3_1.y - 200.0).abs() < 1e-4);

        // Evaluate at t=2.0s
        let frame_2 = evaluate_at(&comp, 2.0);
        let col3_2 = frame_2.layers[0].world_matrix.col(3);
        assert!((col3_2.x - 200.0).abs() < 1e-4);
        assert!((col3_2.y - 400.0).abs() < 1e-4);
    }

    #[test]
    fn test_pure_evaluate_data_binding() {
        let mut comp = Composition::new(1920, 1080, 60.0, 5.0);
        let layer_id = LayerId::new();
        let layer = Layer::new(
            layer_id,
            "Score Text",
            LayerContent::Text(TextContent {
                text: Property::binding("scoreboard.team1.name", "Default Team".to_string()),
                font_family: "Inter".to_string(),
                font_size: Property::constant(32.0),
                color: Property::binding("scoreboard.team1.color", Color::WHITE),
                ..Default::default()
            }),
        );
        comp.layers.push(layer);

        // 1. Without data snapshot -> fallback
        let frame_fallback = evaluate_at(&comp, 0.0);
        if let ResolvedContent::Text(ref text) = frame_fallback.layers[0].content {
            assert_eq!(text.text, "Default Team");
            assert_eq!(text.color, Color::WHITE);
        } else {
            panic!("Expected text layer content");
        }

        // 2. With data snapshot -> bound values
        let mut snapshot = DataSnapshot::new();
        snapshot.set("scoreboard.team1.name", "NAVI");
        snapshot.set("scoreboard.team1.color", "#FFCC00FF");

        let frame_bound = evaluate(&comp, Some(&snapshot), 0.0);
        if let ResolvedContent::Text(ref text) = frame_bound.layers[0].content {
            assert_eq!(text.text, "NAVI");
            assert_eq!(text.color, Color::from_hex("#FFCC00FF").unwrap());
        } else {
            panic!("Expected text layer content");
        }
    }

    #[test]
    fn test_playback_clock_scrubbing_and_looping() {
        let mut clock = PlaybackClock::new(5.0, 60.0);
        assert_eq!(clock.time(), 0.0);
        assert!(!clock.is_playing());

        // Scrubbing
        clock.set_time(2.5);
        assert_eq!(clock.time(), 2.5);

        // Stepping while paused does not advance
        clock.step(1.0);
        assert_eq!(clock.time(), 2.5);

        // Play and step
        clock.play();
        clock.step(1.0);
        assert_eq!(clock.time(), 3.5);

        // Loop boundary
        clock.set_loop_range(1.0, 4.0);
        clock.set_time(3.9);
        clock.step(0.2); // Should wrap to 1.1
        assert!((clock.time() - 1.1).abs() < 1e-4);

        // Step single frame forward and backward
        let prev = clock.time();
        clock.step_frame();
        assert!((clock.time() - (prev + 1.0 / 60.0)).abs() < 1e-4);
        clock.step_back_frame();
        assert!((clock.time() - prev).abs() < 1e-4);
    }
}
