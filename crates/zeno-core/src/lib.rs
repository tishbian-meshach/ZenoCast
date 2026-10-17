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
pub use property::{evaluate_keyframes, Easing, Interpolate, Keyframe, Property};
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

    #[test]
    fn test_interpolate_primitives() {
        assert_eq!(10.0f32.interpolate(&20.0f32, 0.5), 15.0);
        assert_eq!(
            Vec2::new(0.0, 10.0).interpolate(&Vec2::new(10.0, 20.0), 0.5),
            Vec2::new(5.0, 15.0)
        );
        assert_eq!(
            Vec3::new(0.0, 0.0, 0.0).interpolate(&Vec3::new(10.0, 20.0, 30.0), 0.5),
            Vec3::new(5.0, 10.0, 15.0)
        );

        let col1 = Color::BLACK;
        let col2 = Color::WHITE;
        let col_mid = col1.interpolate(&col2, 0.5);
        assert!((col_mid.r - 0.5).abs() < 1e-4);

        assert!(!false.interpolate(&true, 0.4));
        assert!(false.interpolate(&true, 1.0));

        assert_eq!(
            "Alpha".to_string().interpolate(&"Beta".to_string(), 0.5),
            "Alpha"
        );
        assert_eq!(
            "Alpha".to_string().interpolate(&"Beta".to_string(), 1.0),
            "Beta"
        );
    }

    #[test]
    fn test_keyframe_interpolation_linear() {
        let prop = Property::keyframes(vec![
            Keyframe::linear(1.0, 100.0f32),
            Keyframe::linear(3.0, 200.0f32),
        ]);

        assert_eq!(prop.evaluate_at(0.0), 100.0);
        assert_eq!(prop.evaluate_at(1.0), 100.0);
        assert_eq!(prop.evaluate_at(2.0), 150.0);
        assert_eq!(prop.evaluate_at(3.0), 200.0);
        assert_eq!(prop.evaluate_at(4.0), 200.0);
    }

    #[test]
    fn test_keyframe_interpolation_hold() {
        let prop = Property::keyframes(vec![
            Keyframe::hold(1.0, 10.0f32),
            Keyframe::linear(2.0, 20.0f32),
        ]);

        assert_eq!(prop.evaluate_at(1.0), 10.0);
        assert_eq!(prop.evaluate_at(1.99), 10.0);
        assert_eq!(prop.evaluate_at(2.0), 20.0);
    }

    #[test]
    fn test_keyframe_interpolation_bezier() {
        let prop = Property::keyframes(vec![
            Keyframe::new(0.0, 0.0f32, Easing::ease_in()),
            Keyframe::linear(1.0, 100.0f32),
        ]);

        let mid = prop.evaluate_at(0.5);
        // Ease-in accelerates slowly, so mid should be less than 50.0
        assert!(mid < 50.0 && mid > 0.0);
    }
}
