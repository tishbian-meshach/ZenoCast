use std::time::{Duration, Instant};
use zeno_core::{Color, LayerId, Property};
use zeno_data::{apply_bindings_to_composition, DataStore, DataValue, StateMachine};
use zeno_engine::{Composition, Layer, LayerContent, TextContent};

#[test]
fn test_json_and_csv_loading_and_1000_keys_at_60hz() {
    let store = DataStore::new();

    // 1. Ingest JSON with nested objects
    let json_sample = r#"{
        "tournament": {
            "title": "International Championship",
            "season": 2026,
            "prize_pool": 1000000.50
        },
        "scoreboard": {
            "team1": { "name": "Alpha", "score": 12 },
            "team2": { "name": "Omega", "score": 9 }
        }
    }"#;

    let json_count = store.load_json_str(json_sample).expect("load JSON failed");
    assert_eq!(json_count, 7);
    assert_eq!(
        store.get("tournament.title"),
        Some(DataValue::String("International Championship".into()))
    );
    assert_eq!(
        store.get("scoreboard.team1.score"),
        Some(DataValue::Integer(12))
    );
    assert_eq!(
        store.get("scoreboard.team2.score"),
        Some(DataValue::Integer(9))
    );

    // 2. Ingest CSV file
    let csv_sample = "player_id,kills,deaths,assists,kda\nP1,14,2,8,11.0\nP2,8,4,12,5.0\n";
    let csv_count = store
        .load_csv_str(csv_sample, "player_id", None)
        .expect("load CSV failed");
    assert_eq!(csv_count, 8); // 2 rows * 4 fields
    assert_eq!(store.get("P1.kills"), Some(DataValue::Integer(14)));
    assert_eq!(store.get("P2.kda"), Some(DataValue::Float(5.0)));

    // 3. High throughput benchmark: 1,000 keys updated at 60 Hz without dropped frames
    // 60 Hz = ~16.6ms per tick. We simulate 60 frames (1 full second) of updates.
    // Each frame updates 1,000 distinct keys.
    let num_keys = 1000;
    let frames = 60;
    let frame_budget = Duration::from_millis(16); // 16.6ms budget

    let keys: Vec<String> = (0..num_keys)
        .map(|i| format!("telemetry.sensor_{}", i))
        .collect();

    for frame in 0..frames {
        let start = Instant::now();
        let batch: Vec<(String, DataValue)> = keys
            .iter()
            .enumerate()
            .map(|(idx, k)| {
                (
                    k.clone(),
                    DataValue::Float(frame as f64 * 1000.0 + idx as f64),
                )
            })
            .collect();

        store.batch_update(&batch);

        // Verification of real-time constraint:
        let elapsed = start.elapsed();
        assert!(
            elapsed < frame_budget,
            "Frame {} took {:?}, exceeding 16.6ms real-time budget!",
            frame,
            elapsed
        );
    }

    assert_eq!(store.len(), 7 + 8 + 1000);
    assert_eq!(
        store.get("telemetry.sensor_999"),
        Some(DataValue::Float(59.0 * 1000.0 + 999.0))
    );
}

#[test]
fn test_text_layer_template_binding_updates_within_one_frame() {
    let store = DataStore::new();
    store.set("match.team1.name", "Cloud9");
    store.set("match.team2.name", "Fnatic");
    store.set("match.team1.score", 15);
    store.set("match.team2.score", 14);
    store.set("match.game_time", 185.0); // 3 minutes 5 seconds

    // Composition with a text layer containing string template bindings
    let mut comp = Composition::new(1920, 1080, 60.0, 10.0);
    let layer_id = LayerId::new();
    let text_layer = Layer::new(
        layer_id,
        "Scoreboard Overlay",
        LayerContent::Text(TextContent {
            text: Property::constant(
                "{match.team1.name} {match.team1.score} - {match.team2.score} {match.team2.name} [{match.game_time | duration:mm:ss}]".into(),
            ),
            font_family: "Inter".into(),
            font_size: Property::constant(36.0),
            color: Property::constant(Color::WHITE),
            ..Default::default()
        }),
    );
    comp.layers.push(text_layer);

    // Apply bindings at Frame 1
    let frame_start = Instant::now();
    apply_bindings_to_composition(&mut comp, &store);
    let frame_elapsed = frame_start.elapsed();

    // Must resolve within 1 frame (< 1ms execution)
    assert!(
        frame_elapsed < Duration::from_millis(16),
        "Binding resolution took too long: {:?}",
        frame_elapsed
    );

    if let LayerContent::Text(ref tc) = comp.layers[0].content {
        assert_eq!(*tc.text.fallback_value(), "Cloud9 15 - 14 Fnatic [03:05]");
    } else {
        panic!("Expected LayerContent::Text");
    }

    // Live update occurs
    store.set("match.team1.score", 16);
    store.set("match.game_time", 195.0);
    comp.layers[0].content = LayerContent::Text(TextContent {
        text: Property::constant(
            "{match.team1.name} {match.team1.score} - {match.team2.score} {match.team2.name} [{match.game_time | duration:mm:ss}]".into(),
        ),
        ..Default::default()
    });

    apply_bindings_to_composition(&mut comp, &store);

    if let LayerContent::Text(ref tc) = comp.layers[0].content {
        assert_eq!(*tc.text.fallback_value(), "Cloud9 16 - 14 Fnatic [03:15]");
    } else {
        panic!("Expected LayerContent::Text");
    }
}

#[test]
fn test_state_machine_idle_in_active_out_transitions_and_actions() {
    let mut sm = StateMachine::new("Idle");

    // Define state transitions: Idle -> In -> Active -> Out
    sm.add_transition("Idle", "In", "play_intro_animation");
    sm.add_transition("In", "Active", "start_loop_playback");
    sm.add_transition("Active", "Out", "play_outro_transition");
    sm.add_transition("Out", "Idle", "hide_overlay");

    assert_eq!(sm.current_state(), "Idle");

    // 1. Transition: Idle -> In
    let act1 = sm.transition_to("In").expect("Transition to In failed");
    assert_eq!(act1.from_state, "Idle");
    assert_eq!(act1.to_state, "In");
    assert_eq!(act1.action_name, "play_intro_animation");
    assert_eq!(sm.current_state(), "In");

    // 2. Transition: In -> Active
    let act2 = sm
        .transition_to("Active")
        .expect("Transition to Active failed");
    assert_eq!(act2.from_state, "In");
    assert_eq!(act2.to_state, "Active");
    assert_eq!(act2.action_name, "start_loop_playback");
    assert_eq!(sm.current_state(), "Active");

    // 3. Transition: Active -> Out
    let act3 = sm.transition_to("Out").expect("Transition to Out failed");
    assert_eq!(act3.from_state, "Active");
    assert_eq!(act3.to_state, "Out");
    assert_eq!(act3.action_name, "play_outro_transition");
    assert_eq!(sm.current_state(), "Out");

    // 4. Transition: Out -> Idle
    let act4 = sm.transition_to("Idle").expect("Transition to Idle failed");
    assert_eq!(act4.from_state, "Out");
    assert_eq!(act4.to_state, "Idle");
    assert_eq!(act4.action_name, "hide_overlay");
    assert_eq!(sm.current_state(), "Idle");

    // Verify all 4 actions logged in exact sequence
    assert_eq!(sm.transition_count(), 4);
    assert_eq!(sm.action_history().len(), 4);
    assert_eq!(sm.action_history()[0].action_name, "play_intro_animation");
    assert_eq!(sm.action_history()[1].action_name, "start_loop_playback");
    assert_eq!(sm.action_history()[2].action_name, "play_outro_transition");
    assert_eq!(sm.action_history()[3].action_name, "hide_overlay");
}

#[test]
fn test_data_triggered_state_transitions() {
    let mut sm = StateMachine::new("Active");
    sm.add_transition("Active", "ScoreHighlight", "trigger_score_flash");

    // Configure data trigger: when "match.goal" changes, trigger "ScoreHighlight"
    sm.add_data_trigger("match.goal", "ScoreHighlight", Duration::from_millis(50));

    let action = sm.on_data_change("match.goal", &DataValue::Boolean(true));
    assert!(action.is_some());
    assert_eq!(action.unwrap().action_name, "trigger_score_flash");
    assert_eq!(sm.current_state(), "ScoreHighlight");
}
