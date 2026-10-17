//! # zeno-engine
//!
//! Pure scene graph, animation evaluation, and render-list generation.
//!
//! In accordance with the engine dependency rule, this crate does not depend on
//! UI, windowing, device capture, or video encoding systems.

pub mod model;

// Re-exports
pub use model::{
    AnimationState, AssetKind, AssetRef, BlendMode, CameraSettings, Composition, Effect, Fill,
    GradientStop, Layer, LayerContent, Mask, MaskMode, ParticleEmitter, ShapeContent,
    ShapeGeometry, StatePlayback, Stroke, TextAlignment, TextContent, Transform3D,
};

#[cfg(test)]
mod tests {
    use super::*;
    use zeno_core::{Color, LayerId, Property, Vec3};

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
}
