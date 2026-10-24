//! Headless full-frame UI rendering integration test.
//!
//! Simulates initial zero/sub-pixel frames, minimum size window frames,
//! and standard 1440x900 broadcast studio frames to guarantee that DockArea,
//! tab layout, and CentralPanel render without assertion failure or crash.

use std::sync::Arc;
use zeno_output::encoder::{
    AudioEncoderSettings, EncoderKind, EncoderPreset, EncoderProfile, RateControl,
    VideoEncoderSettings,
};
use zeno_output::service::OutputManager;
use zeno_ui::dock::create_default_live_layout;
use zeno_ui::project::{Autosaver, Project};
use zeno_ui::ZenocastApp;

fn create_test_app() -> ZenocastApp {
    let project = Project::default();
    let v_settings = VideoEncoderSettings {
        encoder_kind: EncoderKind::X264,
        width: 1920,
        height: 1080,
        fps: 60.0,
        rate_control: RateControl::Cbr,
        bitrate_kbps: 6000,
        max_bitrate_kbps: Some(6000),
        keyframe_interval_seconds: 2.0,
        preset: EncoderPreset::VeryFast,
        profile: EncoderProfile::Main,
        b_frames: 0,
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
        selected_scene_id: None,
        selected_source_id: None,
        show_settings: false,
        is_streaming: false,
        is_recording: false,
        stream_start_time: None,
        active_transition_kind: "Fade".into(),
        transition_duration_ms: 300,
        t_bar_progress: 0.0,
        autosaver: Autosaver::new(std::path::Path::new(".")),
        output_manager,
        wizard: {
            let mut w = zeno_ui::FirstRunWizard::new();
            w.is_open = false;
            w
        },
        license_client: Arc::new(zeno_license::LicenseClient::new_mock_valid()),
        license_key_input: String::new(),
        source_registry: Arc::new(parking_lot::RwLock::new(zeno_media::SourceRegistry::new())),
        compositor: Arc::new(parking_lot::RwLock::new(None)),
        source_dialog: zeno_ui::SourceDialogState::default(),
        preview_texture_id: None,
        program_texture_id: None,
        wgpu_device: None,
        wgpu_queue: None,
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
    }
}

#[test]
fn test_app_renders_multiple_frames_without_panic() {
    let mut app = create_test_app();
    let ctx = egui::Context::default();

    // Frame 0: uninitialized / tiny screen (0x0 or 50x50) - must not crash DockArea
    let input_tiny = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(50.0, 50.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input_tiny, |ctx| {
        app.update_ui(ctx);
    });

    // Frame 1: minimum inner size (960x600)
    let input_min = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(960.0, 600.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input_min, |ctx| {
        app.update_ui(ctx);
    });

    // Frame 2: standard broadcast studio size (1440x900)
    let input_full = egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(1440.0, 900.0),
        )),
        ..Default::default()
    };
    let _ = ctx.run(input_full, |ctx| {
        app.update_ui(ctx);
    });

    // Frame 3: mode switch to EditOverlay and back
    app.app_mode = zeno_ui::app::AppMode::EditOverlay;
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        app.update_ui(ctx);
    });

    app.app_mode = zeno_ui::app::AppMode::LiveStudio;
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        app.update_ui(ctx);
    });
}
