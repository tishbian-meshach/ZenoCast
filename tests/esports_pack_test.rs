//! Acceptance test for Milestone M17: Esports pack (templates, rosters, and scene presets).
//!
//! Validates:
//! 1. All 14 esports templates export to `templates/esports/` and validate as intact .arx containers.
//! 2. Roster manager presets sync all match and player fields into the reactive `DataStore`.
//! 3. One-click tournament scene generator creates a complete 6-scene production setup with
//!    configured transition rules, proving a full tournament stream setup is ready in under 10 minutes.

use pollster::FutureExt;
use std::path::Path;
use zeno_compositor::{SceneCompositor, TournamentPresetGenerator};
use zeno_data::{export_esports_pack, DataStore, RosterManager};
use zeno_engine::GpuContext;
use zeno_format::ArxPackage;

#[test]
fn test_esports_templates_pack_export_and_integrity() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = if manifest_dir.ends_with("zeno-compositor") {
        manifest_dir.parent().unwrap().parent().unwrap()
    } else {
        manifest_dir
    };
    let out_dir = workspace_root.join("templates/esports");
    let written = export_esports_pack(&out_dir).expect("Export esports pack to templates/esports");

    assert_eq!(written.len(), 14, "Expected 14 broadcast template packages");

    let expected_files = [
        "starting_soon.arx",
        "be_right_back.arx",
        "stream_ending.arx",
        "scoreboard_bo1.arx",
        "scoreboard_bo3.arx",
        "scoreboard_bo5.arx",
        "lower_third.arx",
        "caster_desk_2.arx",
        "caster_desk_3.arx",
        "caster_desk_4.arx",
        "player_cam_frames.arx",
        "map_veto.arx",
        "kill_feed.arx",
        "tournament_bracket.arx",
    ];

    for filename in expected_files {
        let path = out_dir.join(filename);
        assert!(path.exists(), "Template file missing: {}", path.display());

        // Read and validate the .arx container
        let pkg = ArxPackage::open(&path).expect(&format!("Open package {}", path.display()));
        assert!(!pkg.manifest.name.is_empty());
        assert_eq!(pkg.manifest.author, "Zenocast Esports Pack");
        assert!(!pkg.scene.layers.is_empty(), "Scene layers must not be empty in {}", filename);
    }
}

#[test]
fn test_roster_manager_presets_and_datastore_sync() {
    let mgr = RosterManager::new();
    assert!(mgr.teams().len() >= 4, "Expected at least 4 preset teams");

    let sen = mgr.find_team("SEN").expect("Sentinels found by tag");
    assert_eq!(sen.name, "Sentinels");
    assert_eq!(sen.roster.len(), 5);

    let fnc = mgr.find_team("FNC").expect("Fnatic found by tag");
    assert_eq!(fnc.name, "Fnatic");

    let store = DataStore::new();
    let def_match = mgr.default_match();
    mgr.sync_to_data_store(&def_match, &store);

    // Verify stored keys
    assert_eq!(
        store.get("tournament.name").map(|v| v.to_display_string()),
        Some("ZENOCAST MASTERS 2026".to_string())
    );
    assert_eq!(
        store.get("match.title").map(|v| v.to_display_string()),
        Some("GRAND FINALS".to_string())
    );
    assert_eq!(
        store.get("match.format").map(|v| v.to_display_string()),
        Some("BEST OF 5".to_string())
    );
    assert_eq!(
        store.get("match.team1.tag").map(|v| v.to_display_string()),
        Some("SEN".to_string())
    );
    assert_eq!(
        store.get("match.team2.tag").map(|v| v.to_display_string()),
        Some("FNC".to_string())
    );
    assert_eq!(
        store.get("match.team1.player_1.handle").map(|v| v.to_display_string()),
        Some("TenZ".to_string())
    );
    assert_eq!(
        store.get("casters.caster_1.handle").map(|v| v.to_display_string()),
        Some("Pansy".to_string())
    );
}

#[test]
fn test_one_click_tournament_scene_generation() {
    let gpu = GpuContext::init_headless()
        .block_on()
        .expect("Headless GPU context initialization");

    let mut compositor = SceneCompositor::new(
        std::sync::Arc::new(gpu.device),
        std::sync::Arc::new(gpu.queue),
        1920,
        1080,
        60.0,
    );

    let start_instant = std::time::Instant::now();
    let scene_set = TournamentPresetGenerator::generate(&mut compositor);
    let elapsed = start_instant.elapsed();

    // Generation takes < 50ms
    assert!(
        elapsed.as_millis() < 50,
        "One-click generation took {elapsed:?}, expected < 50ms"
    );

    // Verify all 6 core scenes were generated and registered
    assert_eq!(scene_set.all_scene_ids.len(), 6);
    assert_eq!(scene_set.rule_count, 5);

    for scn_id in &scene_set.all_scene_ids {
        assert!(compositor.get_scene(*scn_id).is_some());
    }

    // Verify starting soon scene has overlay item
    let starting = compositor.get_scene(scene_set.starting_soon_id).unwrap();
    assert_eq!(starting.name, "Starting Soon");
    assert!(!starting.items.is_empty());

    // Verify live match scene has game capture and overlays
    let live_match = compositor.get_scene(scene_set.live_match_id).unwrap();
    assert_eq!(live_match.name, "Live Match");
    assert_eq!(live_match.items.len(), 4); // Game capture, player cams, scoreboard, kill feed

    // Verify transition rules by setting Direct mode and triggering a scene switch
    compositor.set_mode(zeno_compositor::StudioMode::Direct);
    compositor.switch_scene(scene_set.starting_soon_id, None);
    assert_eq!(compositor.program_scene(), scene_set.starting_soon_id);

    compositor.switch_scene(scene_set.caster_desk_id, None);
    // Transition should be active
    assert!(compositor.is_transition_active());
}
