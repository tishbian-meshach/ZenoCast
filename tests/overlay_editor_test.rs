use tempfile::tempdir;
use zeno_core::{Color, Easing, Keyframe, Property, Vec2, Vec3};
use zeno_engine::{LayerContent, ShapeGeometry};
use zeno_ui::editor::{OverlayEditor, TimelineMarker};

#[test]
fn test_build_lower_third_and_save_reload_arx() {
    let mut editor = OverlayEditor::new(1920, 1080, 60.0, 5.0);

    // 1. Add background rectangle shape layer
    let _bg_id = editor.add_shape_layer(
        "Lower Third Background",
        Vec2::new(700.0, 100.0),
        Color::rgba(0.09, 0.1, 0.12, 0.95),
    );

    // 2. Add animated text layer
    let text_id = editor.add_text_layer(
        "Lower Third Headline",
        "GRAND FINALS: TEAM ALPHA VS TEAM BETA",
    );

    // Add position keyframes to text layer
    editor.selected_layer_id = Some(text_id);
    editor.clock.seek(0.0);
    editor.add_position_keyframe(Vec3::new(-300.0, 950.0, 0.0), Easing::ease());
    editor.clock.seek(1.0);
    editor.add_position_keyframe(Vec3::new(400.0, 950.0, 0.0), Easing::Linear);

    // Add opacity keyframes to text layer
    editor.clock.seek(0.0);
    editor.add_opacity_keyframe(0.0, Easing::ease_in_out());
    editor.clock.seek(0.5);
    editor.add_opacity_keyframe(100.0, Easing::Linear);

    // Add timeline marker at t=1.0 for intro finish
    editor.markers.push(TimelineMarker {
        time_seconds: 1.0,
        name: "Intro Finish".into(),
        comment: None,
    });

    assert_eq!(editor.composition.layers.len(), 2);

    // Save as .arx package
    let temp = tempdir().unwrap();
    let arx_path = temp.path().join("lower_third.arx");
    editor.save_arx(&arx_path).expect("Failed to save .arx");
    assert!(arx_path.exists());

    // Reload from .arx package
    let loaded_editor = OverlayEditor::load_arx(&arx_path).expect("Failed to load .arx");
    assert_eq!(loaded_editor.composition.width, 1920);
    assert_eq!(loaded_editor.composition.height, 1080);
    assert_eq!(loaded_editor.composition.frames_per_second, 60.0);
    assert_eq!(loaded_editor.composition.duration_seconds, 5.0);
    assert_eq!(loaded_editor.composition.layers.len(), 2);

    // Verify Shape Layer
    let loaded_bg = &loaded_editor.composition.layers[0];
    assert_eq!(loaded_bg.name, "Lower Third Background");
    if let LayerContent::Shape(sc) = &loaded_bg.content {
        if let ShapeGeometry::Rectangle { size } = &sc.geometry {
            assert_eq!(*size.fallback_value(), Vec2::new(700.0, 100.0));
        } else {
            panic!("Expected ShapeGeometry::Rectangle");
        }
    } else {
        panic!("Expected LayerContent::Shape");
    }

    // Verify Text Layer and Keyframes
    let loaded_text = &loaded_editor.composition.layers[1];
    assert_eq!(loaded_text.name, "Lower Third Headline");
    if let LayerContent::Text(tc) = &loaded_text.content {
        assert_eq!(
            *tc.text.fallback_value(),
            "GRAND FINALS: TEAM ALPHA VS TEAM BETA"
        );
    } else {
        panic!("Expected LayerContent::Text");
    }

    let pos_keys = loaded_text.transform.position.as_keyframes();
    assert_eq!(pos_keys.len(), 2);
    assert_eq!(pos_keys[0].time_seconds, 0.0);
    assert_eq!(pos_keys[0].value.x, -300.0);
    assert_eq!(pos_keys[1].time_seconds, 1.0);
    assert_eq!(pos_keys[1].value.x, 400.0);

    let op_keys = loaded_text.transform.opacity_percent.as_keyframes();
    assert_eq!(op_keys.len(), 2);
    assert_eq!(op_keys[0].value, 0.0);
    assert_eq!(op_keys[1].value, 100.0);
}

#[test]
fn test_bezier_curve_tangents_and_evaluation() {
    // Cubic bezier evaluation check with custom tangent control handles:
    // P1 = (0.5, 0.1), P2 = (0.5, 0.6)
    let y1 = 0.1_f32;
    let y2 = 0.6_f32;
    let easing = Easing::CubicBezier {
        control_1_x: 0.5,
        control_1_y: y1,
        control_2_x: 0.5,
        control_2_y: y2,
    };

    // Evaluate property with this easing from 0 to 100
    let prop = Property::keyframes(vec![
        Keyframe {
            time_seconds: 0.0,
            value: 0.0_f32,
            easing_to_next: easing,
        },
        Keyframe {
            time_seconds: 1.0,
            value: 100.0_f32,
            easing_to_next: Easing::Linear,
        },
    ]);

    let val_at_half = prop.evaluate_at(0.5);

    // Manual cubic formula at parameter t=0.5:
    // B_y(t) = 3*(1-t)^2*t*y1 + 3*(1-t)*t^2*y2 + t^3
    let t = 0.5_f32;
    let u = 1.0 - t;
    let manual_cubic_y = 3.0 * u * u * t * y1 + 3.0 * u * t * t * y2 + t * t * t;
    let expected = manual_cubic_y * 100.0;

    assert!(
        (val_at_half - expected).abs() < 1e-4,
        "Expected value {:?} to match manual cubic formula {:?}",
        val_at_half,
        expected
    );
}

#[test]
fn test_undo_redo_50_operations() {
    let mut editor = OverlayEditor::new(1920, 1080, 60.0, 10.0);

    // Initial base layer
    let layer_id = editor.add_text_layer("Dynamic Layer", "Initial");
    editor.selected_layer_id = Some(layer_id);

    // Perform 50 sequential modify text commands
    for i in 1..=50 {
        let ok = editor.modify_selected_text(&format!("Revision {}", i));
        assert!(ok, "modify_selected_text failed at step {}", i);
    }

    assert_eq!(editor.history.undo_count(), 51);
    assert_eq!(editor.history.redo_count(), 0);

    // Verify current text is Revision 50
    if let LayerContent::Text(ref tc) = editor.composition.layers[0].content {
        assert_eq!(*tc.text.fallback_value(), "Revision 50");
    } else {
        panic!("Expected text content");
    }

    // Step-by-step undo all 50 text modifications
    for i in (1..=50).rev() {
        let expected_text = if i == 1 {
            "Initial".to_string()
        } else {
            format!("Revision {}", i - 1)
        };
        editor.undo();
        if let LayerContent::Text(ref tc) = editor.composition.layers[0].content {
            assert_eq!(*tc.text.fallback_value(), expected_text);
        }
    }

    assert_eq!(editor.history.redo_count(), 50);

    // Step-by-step redo all 50 operations back to Revision 50
    for i in 1..=50 {
        editor.redo();
        if let LayerContent::Text(ref tc) = editor.composition.layers[0].content {
            assert_eq!(*tc.text.fallback_value(), format!("Revision {}", i));
        }
    }

    assert_eq!(editor.history.redo_count(), 0);
}

#[test]
fn test_editor_clipboard_copy_paste() {
    let mut editor = OverlayEditor::new(1920, 1080, 60.0, 10.0);
    let original_id = editor.add_shape_layer(
        "Copyable Layer",
        Vec2::new(100.0, 50.0),
        Color::rgb(1.0, 0.0, 0.0),
    );
    editor.selected_layer_id = Some(original_id);

    // Copy selected layer
    assert!(editor.copy_selected());

    // Paste copied layer
    let pasted_id = editor.paste().expect("Paste should succeed");
    assert_ne!(pasted_id, original_id);
    assert_eq!(editor.composition.layers.len(), 2);
    assert_eq!(editor.composition.layers[1].name, "Copyable Layer (Copy)");
    assert_eq!(editor.selected_layer_id, Some(pasted_id));
}
