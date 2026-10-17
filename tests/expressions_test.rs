//! Acceptance test for Milestone M15: Sandboxed expression runtime.
//!
//! Validates:
//! 1. Wiggle built-in produces continuous dynamic values across time within amplitude bounds.
//! 2. Data-driven expressions access runtime snapshots (e.g. data("match.team1_score") * 2 + 10).
//! 3. Infinite loop (while true {}) is halted immediately by the instruction limit without blocking.
//! 4. Engine evaluation properly resolves Property::Expression on layers.

use std::collections::HashMap;
use zeno_core::{Color, LayerId, Property, Vec3};
use zeno_engine::eval::DataSnapshot;
use zeno_engine::evaluate;
use zeno_engine::model::{Composition, Layer, LayerContent, ShapeContent, ShapeGeometry};
use zeno_script::{ScriptError, ScriptRuntime};

#[test]
fn test_wiggle_evaluation_across_time() {
    let runtime = ScriptRuntime::new();
    let freq = 2.0;
    let amp = 20.0;
    let expr = format!("wiggle({freq}, {amp})");

    let val_0 = runtime
        .eval_f64(&expr, 0.0, 0, 0.0, None)
        .expect("eval at t=0");
    let val_1 = runtime
        .eval_f64(&expr, 0.25, 0, 0.0, None)
        .expect("eval at t=0.25");
    let val_2 = runtime
        .eval_f64(&expr, 0.5, 0, 0.0, None)
        .expect("eval at t=0.5");

    // Values should stay within [-amp, amp]
    assert!(val_0.abs() <= amp + 0.001, "val_0 {val_0} exceeded amp");
    assert!(val_1.abs() <= amp + 0.001, "val_1 {val_1} exceeded amp");
    assert!(val_2.abs() <= amp + 0.001, "val_2 {val_2} exceeded amp");

    // Dynamic variation across time
    assert!(
        (val_0 - val_1).abs() > 1e-4 || (val_1 - val_2).abs() > 1e-4,
        "wiggle should vary across time steps"
    );
}

#[test]
fn test_data_driven_expressions() {
    let runtime = ScriptRuntime::new();
    let mut data = HashMap::new();
    data.insert(
        "match.team1_score".to_string(),
        serde_json::Value::Number(serde_json::Number::from(7)),
    );
    data.insert(
        "team.tag".to_string(),
        serde_json::Value::String("SEN".to_string()),
    );

    // Numeric calculation using data
    let score_calc = runtime
        .eval_f64(
            "data(\"match.team1_score\") * 2.0 + 10.0",
            0.0,
            0,
            0.0,
            Some(&data),
        )
        .expect("eval score_calc");
    assert_eq!(score_calc, 24.0);

    // String interpolation using data
    let tag_str = runtime
        .eval_string("\"TEAM: \" + data(\"team.tag\")", 0.0, 0, "", Some(&data))
        .expect("eval tag_str");
    assert_eq!(tag_str, "TEAM: SEN");
}

#[test]
fn test_infinite_loop_sandboxing_timeout() {
    let runtime = ScriptRuntime::new();
    let start = std::time::Instant::now();

    // Expression containing an infinite loop
    let result = runtime.eval_f64("while true {}", 0.0, 0, 42.0, None);

    let elapsed = start.elapsed();
    assert!(
        elapsed.as_millis() < 500,
        "Infinite loop must be terminated within sub-second limit, took {elapsed:?}"
    );

    match result {
        Err(ScriptError::InstructionLimitExceeded(_msg)) => {}
        other => panic!("Expected InstructionLimitExceeded error, got: {other:?}"),
    }
}

#[test]
fn test_engine_property_expression_evaluation() {
    let mut comp = Composition::new(1920, 1080, 60.0, 5.0);
    let mut layer = Layer::new(
        LayerId::new(),
        "AnimatedBox",
        LayerContent::Shape(ShapeContent {
            geometry: ShapeGeometry::Rectangle {
                size: Property::constant(zeno_core::Vec2::new(100.0, 100.0)),
            },
            fill: Some(zeno_engine::model::Fill::Solid(Property::constant(
                Color::WHITE,
            ))),
            stroke: None,
        }),
    );

    // Opacity driven by expression
    layer.transform.opacity_percent = Property::expression("50.0 + 25.0", 100.0);
    // Position driven by expression with data
    layer.transform.position =
        Property::expression("[data(\"hero.x\"), 200.0, 0.0]", Vec3::new(0.0, 0.0, 0.0));

    comp.layers.push(layer);

    let mut snapshot = DataSnapshot::new();
    snapshot.set("hero.x", 450.0);

    let frame = evaluate(&comp, Some(&snapshot), 1.0);
    assert_eq!(frame.layers.len(), 1);

    let resolved = &frame.layers[0];
    // Opacity should be (50 + 25) / 100 = 0.75
    assert!((resolved.net_opacity - 0.75).abs() < 1e-4);

    // Transform position x should be 450.0
    let trans = resolved.world_matrix;
    assert!((trans.w_axis.x - 450.0).abs() < 1e-3);
    assert!((trans.w_axis.y - 200.0).abs() < 1e-3);
}
