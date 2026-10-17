//! M10c Acceptance Test: Spreadsheet-linked data sources
//!
//! Verifies:
//! 1. Editing a cell in a spreadsheet updates bound scoreboard text and fires `score_change` transition.
//! 2. Missing or locked files keep the last good value in the data store and show warning badges.
//! 3. A 10-row range binds to and fills a Repeater layer with 10 instances and proper positional offsets.

use std::fs::File;
use std::io::Write;
use tempfile::tempdir;

use zeno_core::{LayerId, Property, Vec2};
use zeno_data::sheet::{
    CellBinding, CellTarget, SheetLink, SheetLinkStatus, SheetSource, SheetSyncEngine,
    SpreadsheetTable, VariableKind,
};
use zeno_data::{apply_bindings_to_composition, DataStore, StateMachine};
use zeno_engine::eval::evaluate;
use zeno_engine::{Composition, Layer, LayerContent, TextContent};

/// Test 1: Editing a cell updates bound scoreboard text and fires `score_change` trigger.
#[test]
fn test_spreadsheet_cell_edit_updates_bound_text_and_fires_score_change() {
    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("match_scoreboard.csv");

    // Initial CSV content: A1=ColA, B1=ScoreHeader; A2=TeamA, B2=10; A3=TeamB, B3=20
    let mut f = File::create(&csv_path).unwrap();
    writeln!(f, "Team,Score").unwrap();
    writeln!(f, "Sentinels,10").unwrap();
    writeln!(f, "Fnatic,20").unwrap();
    drop(f);

    let store = DataStore::new();
    let mut sync_engine = SheetSyncEngine::new();

    // Link B2 (Sentinels score) to scoreboard.team1.score
    let mut link = SheetLink::new(
        SheetSource::LocalFile {
            path: csv_path.clone(),
        },
        "Sheet1",
    );
    link.add_binding(CellBinding::new(
        "scoreboard.team1.score",
        CellTarget::Cell("B2".to_string()),
        VariableKind::Number,
    ));
    let link_idx = sync_engine.add_link(link);

    // Setup StateMachine with reactive DataTriggerRule on scoreboard.team1.score
    let mut sm = StateMachine::new("Idle");
    sm.add_transition("Idle", "score_change", "pulse_animation");
    sm.add_transition("score_change", "Idle", "reset");
    sm.add_data_trigger(
        "scoreboard.team1.score",
        "score_change",
        std::time::Duration::from_millis(0),
    );

    let sm_ref = std::sync::Arc::new(parking_lot::Mutex::new(sm));
    let sm_clone = sm_ref.clone();
    store.add_change_listener(move |key, val| {
        sm_clone.lock().on_data_change(key, val);
    });

    // Initial sync
    let changes = sync_engine.sync_link(link_idx, &store).unwrap();
    assert_eq!(changes, 1);
    assert_eq!(
        store.get("scoreboard.team1.score").unwrap().as_i64(),
        Some(10)
    );
    assert_eq!(sm_ref.lock().current_state(), "score_change");

    // Reset state machine back to Idle
    let _ = sm_ref.lock().transition_to("Idle");
    assert_eq!(sm_ref.lock().current_state(), "Idle");

    // Second sync with no file edits: 0 changes, no state machine trigger
    let changes_no_op = sync_engine.sync_link(link_idx, &store).unwrap();
    assert_eq!(changes_no_op, 0);
    assert_eq!(sm_ref.lock().current_state(), "Idle");

    // Simulate scorekeeper editing cell B2 in the spreadsheet to 11
    let mut f_edit = File::create(&csv_path).unwrap();
    writeln!(f_edit, "Team,Score").unwrap();
    writeln!(f_edit, "Sentinels,11").unwrap();
    writeln!(f_edit, "Fnatic,20").unwrap();
    drop(f_edit);

    // Sync poll
    let changes_edited = sync_engine.sync_link(link_idx, &store).unwrap();
    assert_eq!(changes_edited, 1);
    assert_eq!(
        store.get("scoreboard.team1.score").unwrap().as_i64(),
        Some(11)
    );

    // Assert state machine transitioned to score_change
    assert_eq!(sm_ref.lock().current_state(), "score_change");
    assert_eq!(sm_ref.lock().action_history().len(), 3);

    // Assert bound composition text reflects updated score
    let mut comp = Composition::new(1920, 1080, 60.0, 10.0);
    let text_layer = Layer::new(
        LayerId::new(),
        "Team1ScoreText",
        LayerContent::Text(TextContent {
            text: Property::constant("{scoreboard.team1.score}".to_string()),
            font_family: "Inter".to_string(),
            font_size: Property::constant(48.0),
            color: Property::constant(zeno_core::Color::WHITE),
            ..Default::default()
        }),
    );
    comp.layers.push(text_layer);

    apply_bindings_to_composition(&mut comp, &store);
    if let LayerContent::Text(tc) = &comp.layers[0].content {
        assert_eq!(*tc.text.fallback_value(), "11");
    } else {
        panic!("Expected text layer");
    }
}

/// Test 2: Missing or locked file keeps last good value and shows warning status.
#[test]
fn test_missing_or_locked_file_preserves_last_values_and_shows_warning() {
    let dir = tempdir().unwrap();
    let csv_path = dir.path().join("live_data.csv");

    // 1. Write initial good data
    let mut f = File::create(&csv_path).unwrap();
    writeln!(f, "Player,Kills").unwrap();
    writeln!(f, "TenZ,25").unwrap();
    drop(f);

    let store = DataStore::new();
    let mut sync_engine = SheetSyncEngine::new();

    let mut link = SheetLink::new(
        SheetSource::LocalFile {
            path: csv_path.clone(),
        },
        "Sheet1",
    );
    link.add_binding(CellBinding::new(
        "stats.player.kills",
        CellTarget::Cell("B2".to_string()),
        VariableKind::Number,
    ));
    let link_idx = sync_engine.add_link(link);

    // Initial successful sync
    let count = sync_engine.sync_link(link_idx, &store).unwrap();
    assert_eq!(count, 1);
    assert_eq!(store.get("stats.player.kills").unwrap().as_i64(), Some(25));
    assert_eq!(sync_engine.links()[link_idx].status, SheetLinkStatus::Ok);

    // 2. Simulate file missing / moved
    std::fs::remove_file(&csv_path).unwrap();

    // Sync poll should fail gracefully
    let err_res = sync_engine.sync_link(link_idx, &store);
    assert!(err_res.is_err());

    // CRITICAL ACCEPTANCE CHECK: DataStore keeps the last good value!
    assert_eq!(
        store.get("stats.player.kills").unwrap().as_i64(),
        Some(25),
        "DataStore must keep last good value when file is missing"
    );

    // CRITICAL ACCEPTANCE CHECK: Status shows warning badge
    match &sync_engine.links()[link_idx].status {
        SheetLinkStatus::Warning(msg) => {
            assert!(msg.contains("not found"));
        }
        _ => panic!("Expected SheetLinkStatus::Warning for missing file"),
    }

    // 3. Re-create file and verify it recovers cleanly to Ok
    let mut f_recover = File::create(&csv_path).unwrap();
    writeln!(f_recover, "Player,Kills").unwrap();
    writeln!(f_recover, "TenZ,26").unwrap();
    drop(f_recover);

    let recover_count = sync_engine.sync_link(link_idx, &store).unwrap();
    assert_eq!(recover_count, 1);
    assert_eq!(store.get("stats.player.kills").unwrap().as_i64(), Some(26));
    assert_eq!(sync_engine.links()[link_idx].status, SheetLinkStatus::Ok);
}

/// Test 3: A 10-row range fills a Repeater layer.
#[test]
fn test_ten_row_range_fills_repeater() {
    let store = DataStore::new();
    let mut sync_engine = SheetSyncEngine::new();

    // Create a 10-row table of standings
    let mut rows = Vec::new();
    rows.push(vec![
        zeno_data::DataValue::String("Rank".to_string()),
        zeno_data::DataValue::String("Team".to_string()),
        zeno_data::DataValue::String("Points".to_string()),
    ]);
    for i in 1..=10 {
        rows.push(vec![
            zeno_data::DataValue::Integer(i as i64),
            zeno_data::DataValue::String(format!("Team_{}", i)),
            zeno_data::DataValue::Integer((100 - i * 5) as i64),
        ]);
    }
    let table = SpreadsheetTable::new("Standings", rows);

    // Register mock table
    sync_engine.register_mock_table("google_sheet_tournament_123", table);

    let mut link = SheetLink::new(
        SheetSource::GoogleSheet {
            spreadsheet_id: "google_sheet_tournament_123".to_string(),
        },
        "Standings",
    );
    // Bind 10 rows: A1 to C10
    link.add_binding(CellBinding::new(
        "tournament.standings",
        CellTarget::Range("A1:C10".to_string()),
        VariableKind::Text,
    ));
    let link_idx = sync_engine.add_link(link);

    // Sync
    let changed = sync_engine.sync_link(link_idx, &store).unwrap();
    assert!(changed > 0);

    // Check store table data
    let standings_val = store.get("tournament.standings").unwrap();
    if let zeno_data::DataValue::Table(grid) = standings_val {
        assert_eq!(grid.len(), 10, "Range A1:C10 contains 10 rows");
    } else {
        panic!("Expected DataValue::Table");
    }

    // Setup composition with a template row layer and a Repeater layer
    let mut comp = Composition::new(1920, 1080, 60.0, 10.0);

    let template_layer_id = LayerId::new();
    let template_layer = Layer::new(template_layer_id, "RowTemplate", LayerContent::Null);
    comp.layers.push(template_layer);

    let repeater_layer_id = LayerId::new();
    let repeater_layer = Layer::new(
        repeater_layer_id,
        "StandingsRepeater",
        LayerContent::Repeater {
            template_group: template_layer_id,
            data_path: "tournament.standings".to_string(),
            max_rows: 10,
            row_offset: Property::constant(Vec2::new(0.0, 42.0)),
        },
    );
    comp.layers.push(repeater_layer);

    // Evaluate frame with snapshot from store
    let snapshot = store.to_snapshot();
    let frame = evaluate(&comp, Some(&snapshot), 0.0);

    // Find all instanced layers created for the repeater
    let instances: Vec<_> = frame
        .layers
        .iter()
        .filter(|l| l.name.starts_with("RowTemplate_row"))
        .collect();

    assert_eq!(
        instances.len(),
        10,
        "A 10-row range must fill exactly 10 repeater row instances"
    );

    // Check that vertical offsets increase monotonically by row_offset (42.0 px per row)
    for (i, inst) in instances.iter().enumerate() {
        assert_eq!(inst.name, format!("RowTemplate_row{}", i));
        let expected_y = (i as f32) * 42.0;
        let trans_y = inst.world_matrix.w_axis.y;
        assert!(
            (trans_y - expected_y).abs() < 1e-4,
            "Instance {} should have y-offset close to {}, got {}",
            i,
            expected_y,
            trans_y
        );
    }
}
