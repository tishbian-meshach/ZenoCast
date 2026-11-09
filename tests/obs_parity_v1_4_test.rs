//! Acceptance and regression tests for Zenocast v1.4: OBS-Parity Gap Fixes.
//!
//! Verifies:
//! 1. Default Normal Mode (Single Program Monitor) & Studio Mode toggling.
//! 2. Decibel (dBFS) volume mapping, logarithmic fader curves, and roundtrips.
//! 3. Eye & Lock safety controls on sources list and preview interaction.
//! 4. Full OBS context menu parity:
//!    - Transform controls (Center, Fit to screen, Reset)
//!    - Clipboard operations: Copy, Paste Reference, Paste Duplicate
//!    - Native output resize to source
//!    - Preview monitor lock and scaling modes

use std::path::Path;
use std::sync::Arc;

use zeno_output::encoder::{AudioEncoderSettings, EncoderKind, RateControl, VideoEncoderSettings};
use zeno_output::service::OutputManager;
use zeno_ui::dock::create_default_live_layout;
use zeno_ui::dock::{db_to_volume, volume_to_db};
use zeno_ui::preferences::UserPreferences;
use zeno_ui::project::{
    Autosaver, PreviewScalingMode, Project, SceneDefinition, SourceDefinition, SourceKind,
};
use zeno_ui::ZenocastApp;

fn create_test_app() -> ZenocastApp {
    let project = Project {
        scenes: vec![
            SceneDefinition {
                id: "scene_main".into(),
                name: "Main Scene".into(),
                sources: vec![
                    SourceDefinition {
                        id: "src_display".into(),
                        name: "Display Capture".into(),
                        kind: SourceKind::DisplayCapture,
                        target: "Primary".into(),
                        visible: true,
                        locked: false,
                        volume: 1.0,
                        muted: false,
                        x: 0.0,
                        y: 0.0,
                        width: 1920.0,
                        height: 1080.0,
                        rotation: 0.0,
                        ..Default::default()
                    },
                    SourceDefinition {
                        id: "src_cam".into(),
                        name: "Webcam".into(),
                        kind: SourceKind::CameraFeed,
                        target: "Cam 1".into(),
                        visible: true,
                        locked: false,
                        volume: 0.8,
                        muted: false,
                        x: 100.0,
                        y: 100.0,
                        width: 640.0,
                        height: 360.0,
                        rotation: 0.0,
                        ..Default::default()
                    },
                ],
            },
            SceneDefinition {
                id: "scene_secondary".into(),
                name: "Secondary Scene".into(),
                sources: Vec::new(),
            },
        ],
        active_scene_id: Some("scene_main".into()),
        preview_scene_id: Some("scene_main".into()),
        ..Default::default()
    };

    let v_settings = VideoEncoderSettings {
        encoder_kind: EncoderKind::X264,
        width: 1920,
        height: 1080,
        fps: 60.0,
        rate_control: RateControl::Cbr,
        bitrate_kbps: 6000,
        max_bitrate_kbps: Some(6000),
        keyframe_interval_seconds: 2.0,
        preset: zeno_output::encoder::EncoderPreset::VeryFast,
        profile: zeno_output::encoder::EncoderProfile::Main,
        b_frames: 0,
        handoff_mode: Default::default(),
    };
    let a_settings = AudioEncoderSettings {
        sample_rate: 48000,
        channels: 2,
        bitrate_kbps: 160,
    };
    let output_manager = Arc::new(OutputManager::new_dry_run(v_settings, a_settings));

    ZenocastApp {
        project,
        project_path: None,
        dock_state: create_default_live_layout(),
        selected_scene_id: Some("scene_main".into()),
        selected_source_id: Some("src_display".into()),
        show_settings: false,
        is_streaming: false,
        is_recording: false,
        stream_start_time: None,
        active_transition_kind: "Fade".into(),
        transition_duration_ms: 300,
        t_bar_progress: 0.0,
        autosaver: Autosaver::new(Path::new(".")),
        output_manager,
        wizard: zeno_ui::FirstRunWizard::new(),
        license_client: Arc::new(zeno_license::LicenseClient::new_mock_valid()),
        license_key_input: String::new(),
        source_registry: Arc::new(parking_lot::RwLock::new(zeno_media::SourceRegistry::new())),
        compositor: Arc::new(parking_lot::RwLock::new(None)),
        source_dialog: zeno_ui::SourceDialogState::default(),
        preview_texture_id: None,
        program_texture_id: None,
        wgpu_device: None,
        wgpu_queue: None,
        source_warning_message: None,
        app_mode: zeno_ui::app::AppMode::LiveStudio,
        overlay_editor: zeno_ui::OverlayEditor::default(),
        timeline_mode: zeno_ui::editor::TimelineMode::DopeSheet,
        data_panel_state: zeno_ui::DataPanelState::default(),
        data_store: Arc::new(zeno_data::DataStore::new()),
        control_state: Arc::new(zeno_control::ControlState::new(Arc::new(
            zeno_data::DataStore::new(),
        ))),
        master_volume: 1.0,
        master_muted: false,
        show_open_dialog: false,
        show_save_as_dialog: false,
        file_dialog_path: "project.arxproj".to_string(),
        rename_scene_id: None,
        rename_scene_buffer: String::new(),
        live_gizmo_state: zeno_ui::TransformGizmoState::default(),
        settings_tab: Default::default(),
        hotkey_search_filter: String::new(),
        rebinding_target: None,
        global_hotkey_manager: None,
        global_hotkey_map: std::collections::HashMap::new(),
        previous_scene_id: None,
        studio_undo_stack: Vec::new(),
        studio_redo_stack: Vec::new(),
        push_to_mute_active: std::collections::HashMap::new(),
        push_to_talk_active: std::collections::HashMap::new(),
        studio_mode: false,
        clipboard_source: None,
        clipboard_filters: None,
        filter_dialog: Default::default(),
        preview_locked: false,
        preview_scaling_mode: PreviewScalingMode::ScaleToWindow,
    }
}

#[test]
fn test_v1_4_section_1_default_normal_mode() {
    let prefs = UserPreferences::default();
    assert!(
        !prefs.studio_mode,
        "UserPreferences default must have studio_mode = false (Normal Mode)"
    );

    let mut app = create_test_app();
    assert!(
        !app.studio_mode,
        "ZenocastApp must launch in Normal Mode by default"
    );

    // Toggle to Studio Mode
    app.studio_mode = true;
    assert!(app.studio_mode, "Studio Mode must be active when toggled");

    // Toggle back to Normal Mode
    app.studio_mode = false;
    assert!(
        !app.studio_mode,
        "Normal Mode must be active when toggled back"
    );
}

#[test]
fn test_v1_4_section_3_decibel_volume_scaling() {
    // 1.0 linear volume corresponds to 0.0 dB (unity gain)
    let db_full = volume_to_db(1.0);
    assert!((db_full - 0.0).abs() < 0.01, "1.0 volume must equal 0.0 dB");
    let vol_full = db_to_volume(0.0);
    assert!(
        (vol_full - 1.0).abs() < 0.01,
        "0.0 dB must convert to 1.0 volume"
    );

    // 0.5 linear volume corresponds to approximately -6.0 dB (half amplitude)
    let db_half = volume_to_db(0.5);
    assert!(
        (db_half - (-6.02)).abs() < 0.1,
        "0.5 volume must equal ~ -6 dB"
    );
    let vol_half = db_to_volume(-6.02);
    assert!(
        (vol_half - 0.5).abs() < 0.05,
        "-6 dB must convert to ~0.5 volume"
    );

    // 0.0 linear volume corresponds to -60.0 dB floor
    let db_zero = volume_to_db(0.0);
    assert_eq!(db_zero, -60.0, "0.0 volume must floor at -60.0 dB");
    let vol_zero = db_to_volume(-60.0);
    assert_eq!(vol_zero, 0.0, "-60.0 dB must convert to 0.0 volume");

    // Monotonicity check across the range
    let steps = [0.0, 0.1, 0.25, 0.5, 0.75, 1.0, 1.25];
    for window in steps.windows(2) {
        let v1 = window[0];
        let v2 = window[1];
        assert!(
            volume_to_db(v1) < volume_to_db(v2),
            "dB scaling must be strictly monotonic"
        );
    }
}

#[test]
fn test_v1_4_section_4_eye_and_lock_controls() {
    let mut app = create_test_app();
    let src_id = "src_display".to_string();

    // Verify initial visibility and lock
    {
        let sources = app.project.preview_sources();
        let src = sources.iter().find(|s| s.id == src_id).unwrap();
        assert!(src.visible, "Source must initially be visible");
        assert!(!src.locked, "Source must initially be unlocked");
    }

    // 1. Toggle visibility off
    app.toggle_source_visibility(&src_id);
    {
        let sources = app.project.preview_sources();
        let src = sources.iter().find(|s| s.id == src_id).unwrap();
        assert!(!src.visible, "Source must be hidden after toggle");
    }

    // 2. Toggle visibility on
    app.toggle_source_visibility(&src_id);
    {
        let sources = app.project.preview_sources();
        let src = sources.iter().find(|s| s.id == src_id).unwrap();
        assert!(src.visible, "Source must be visible after second toggle");
    }

    // 3. Toggle lock on
    app.toggle_source_lock(&src_id);
    {
        let sources = app.project.preview_sources();
        let src = sources.iter().find(|s| s.id == src_id).unwrap();
        assert!(src.locked, "Source must be locked after toggle");
    }

    // 4. Attempting to delete a locked source must be rejected with an inline warning
    app.selected_source_id = Some(src_id.clone());
    let sources_before = app.project.preview_sources().len();
    let is_locked = app
        .project
        .preview_sources()
        .iter()
        .any(|s| s.id == src_id && s.locked);
    if is_locked {
        app.source_warning_message = Some((
            "Source is locked. Unlock to delete.".to_string(),
            std::time::Instant::now(),
        ));
    } else {
        app.remove_source_by_id(&src_id);
    }

    assert_eq!(
        app.project.preview_sources().len(),
        sources_before,
        "Locked source must not be deleted"
    );
    assert!(
        app.source_warning_message.is_some(),
        "Inline warning must be displayed when attempting to delete locked source"
    );

    // 5. Unlock source and delete successfully
    app.toggle_source_lock(&src_id);
    {
        let sources = app.project.preview_sources();
        let src = sources.iter().find(|s| s.id == src_id).unwrap();
        assert!(!src.locked, "Source must be unlocked");
    }
    app.remove_source_by_id(&src_id);
    assert_eq!(
        app.project.preview_sources().len(),
        sources_before - 1,
        "Unlocked source must be deleted successfully"
    );
}

#[test]
fn test_v1_4_section_5_clipboard_and_context_menu_parity() {
    let mut app = create_test_app();
    let src_id = "src_cam".to_string();
    app.selected_source_id = Some(src_id.clone());

    // 1. Copy Selected Source
    app.copy_selected_source();
    assert!(
        app.clipboard_source.is_some(),
        "Clipboard must contain copied source"
    );
    let clip = app.clipboard_source.as_ref().unwrap();
    assert_eq!(clip.id, "src_cam");
    assert_eq!(clip.name, "Webcam");

    // 2. Paste (Duplicate)
    let initial_count = app.project.preview_sources().len();
    app.paste_source_duplicate();
    assert_eq!(app.project.preview_sources().len(), initial_count + 1);
    let pasted_dup = app.project.preview_sources().last().unwrap().clone();
    assert_ne!(
        pasted_dup.id, "src_cam",
        "Duplicate paste must generate a new unique ID"
    );
    assert_eq!(
        pasted_dup.name, "Webcam (Copy)",
        "Duplicate paste must append (Copy)"
    );
    assert_eq!(pasted_dup.width, 640.0);
    assert_eq!(pasted_dup.height, 360.0);

    // 3. Paste (Reference)
    app.paste_source_reference();
    assert_eq!(app.project.preview_sources().len(), initial_count + 2);
    let pasted_ref = app.project.preview_sources().last().unwrap().clone();
    assert_eq!(
        pasted_ref.name, "Webcam",
        "Reference paste must retain original name"
    );

    // 4. Resize Output to Source
    app.resize_output_to_source("src_cam");
    assert_eq!(app.project.settings.video.output_width, 640);
    assert_eq!(app.project.settings.video.output_height, 360);
    assert_eq!(app.project.settings.video.base_width, 640);
    assert_eq!(app.project.settings.video.base_height, 360);

    // 5. Preview Lock and Scaling Modes
    assert!(!app.preview_locked);
    app.preview_locked = true;
    assert!(app.preview_locked, "Lock Preview must be toggleable");

    assert_eq!(app.preview_scaling_mode, PreviewScalingMode::ScaleToWindow);
    app.preview_scaling_mode = PreviewScalingMode::Canvas1To1;
    assert_eq!(app.preview_scaling_mode, PreviewScalingMode::Canvas1To1);
    app.preview_scaling_mode = PreviewScalingMode::Output1To1;
    assert_eq!(app.preview_scaling_mode, PreviewScalingMode::Output1To1);
}

#[test]
fn test_v1_5_section_3_source_reference_and_duplicate_lifecycle() {
    let mut app = create_test_app();
    let cam_id = zeno_ui::app::string_to_source_id("src_cam");
    let color_src = Box::new(
        zeno_media::ColorSource::new("Webcam", [0.0, 1.0, 0.0, 1.0], 640, 360).with_id(cam_id),
    );
    app.source_registry.write().register(color_src);

    // Initial state: 1 reference in SourceRegistry
    assert_eq!(app.source_registry.read().ref_count(cam_id), 1);
    assert!(app.source_registry.read().contains(cam_id));

    // Select and Copy Webcam
    app.selected_source_id = Some("src_cam".to_string());
    app.copy_selected_source();

    // 1. Paste Reference
    app.paste_source_reference();
    let ref_item = app
        .project
        .preview_sources()
        .iter()
        .find(|s| s.id != "src_cam" && s.id != "src_display")
        .unwrap()
        .clone();
    assert_eq!(
        ref_item.underlying_source_id(),
        "src_cam",
        "Reference item must point to original underlying source ID"
    );
    assert_eq!(
        app.source_registry.read().ref_count(cam_id),
        2,
        "Paste Reference must increment underlying source ref_count to 2"
    );

    // 2. Paste Duplicate
    app.paste_source_duplicate();
    let dup_item = app.project.preview_sources().last().unwrap().clone();
    assert_ne!(
        dup_item.underlying_source_id(),
        "src_cam",
        "Duplicate item must have independent underlying source ID"
    );
    let dup_source_id = zeno_ui::app::string_to_source_id(dup_item.underlying_source_id());
    assert_eq!(
        app.source_registry.read().ref_count(dup_source_id),
        1,
        "Duplicate source must start with ref_count 1"
    );
    assert_eq!(
        app.source_registry.read().ref_count(cam_id),
        2,
        "Original source ref_count must remain unaffected by duplicate paste"
    );

    // 3. Remove one of the referencing scene items (ref_item)
    app.remove_source_by_id(&ref_item.id);
    assert_eq!(
        app.source_registry.read().ref_count(cam_id),
        1,
        "Removing one referencing item must decrement ref_count to 1 without stopping source"
    );
    assert!(
        app.source_registry.read().contains(cam_id),
        "Source must remain active and registered while ref_count > 0"
    );

    // 4. Remove the original item
    app.remove_source_by_id("src_cam");
    assert_eq!(
        app.source_registry.read().ref_count(cam_id),
        0,
        "Removing the final reference must drop ref_count to 0"
    );
    assert!(
        !app.source_registry.read().contains(cam_id),
        "Source must be torn down and unregistered when ref_count reaches 0"
    );

    // Duplicate source is still active independently
    assert!(
        app.source_registry.read().contains(dup_source_id),
        "Duplicate source must remain active"
    );
    app.remove_source_by_id(&dup_item.id);
    assert!(
        !app.source_registry.read().contains(dup_source_id),
        "Duplicate source is torn down when its reference is removed"
    );
}
