//! # zeno-core
//!
//! Core primitives, mathematical structures, property wrappers, time handling, and identifiers for Zenocast.

pub mod color;
pub mod error;
pub mod id;
pub mod lifecycle;
pub mod property;
pub mod time;
pub mod variable;

// Re-exports
pub use color::Color;
pub use error::CoreError;
pub use id::{AssetId, CompositionId, LayerId, LinkId, SceneId, TransitionId};
pub use lifecycle::{InterruptBehavior, Lifecycle, LifecyclePhase};
pub use property::{Easing, Keyframe, Property};
pub use time::{TimeRange, Timecode};
pub use variable::{VariableDeclaration, VariableKind, VariableValue};

// Re-export glam math types
pub use glam::{Mat4, Quat, Vec2, Vec3, Vec4};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_constant() {
        let prop = Property::constant(42.0f32);
        assert_eq!(*prop.fallback_value(), 42.0);
    }

    #[test]
    fn test_easing_linear() {
        let easing = Easing::Linear;
        assert_eq!(easing.evaluate(0.0), 0.0);
        assert_eq!(easing.evaluate(0.5), 0.5);
        assert_eq!(easing.evaluate(1.0), 1.0);
    }

    #[test]
    fn test_easing_hold() {
        let easing = Easing::Hold;
        assert_eq!(easing.evaluate(0.0), 0.0);
        assert_eq!(easing.evaluate(0.99), 0.0);
        assert_eq!(easing.evaluate(1.0), 1.0);
    }

    #[test]
    fn test_easing_bezier_endpoints() {
        let easing = Easing::ease_in_out();
        assert!((easing.evaluate(0.0) - 0.0).abs() < 1e-4);
        assert!((easing.evaluate(1.0) - 1.0).abs() < 1e-4);
        assert!(easing.evaluate(0.5) > 0.0 && easing.evaluate(0.5) < 1.0);
    }

    #[test]
    fn test_color_hex_roundtrip() {
        let c = Color::from_hex("#FF8800FF").expect("valid hex");
        assert_eq!(c.to_u8_array(), [255, 136, 0, 255]);
        let hex = c.to_hex();
        assert_eq!(hex, "#FF8800FF");
    }

    #[test]
    fn test_timecode_conversions() {
        let tc = Timecode::from_seconds(65.5, 30.0);
        assert_eq!(tc.minutes, 1);
        assert_eq!(tc.seconds, 5);
        assert_eq!(tc.frames, 15);
    }
}
