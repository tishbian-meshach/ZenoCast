//! Integration tests driving real local recording start/stop and validating disk artifacts.

use std::fs;
use std::sync::Arc;
use tempfile::tempdir;
use zeno_output::encoder::{
    AudioEncoderSettings, EncoderKind, EncoderPreset, EncoderProfile, RateControl,
    VideoEncoderSettings,
};
use zeno_output::service::OutputManager;
use zeno_ui::dock::create_default_live_layout;
use zeno_ui::project::{Autosaver, Project};
use zeno_ui::ZenocastApp;

#[test]
pub fn test_start_stop_local_recording_creates_valid_file_on_disk() {
    let temp_dir = tempdir().expect("Temporary directory for recording test");
    let target_recording_path = temp_dir.path().join("recording.mkv");

    let mut project = Project::default();
    project.settings.output.record_directory = temp_dir.path().to_str().unwrap().to_string();
    project.settings.output.remux_to_mp4 = false;

    let v_settings = VideoEncoderSettings {
        encoder_kind: EncoderKind::X264,
        width: 1280,
        height: 720,
        fps: 30.0,
        rate_control: RateControl::Cbr,
        bitrate_kbps: 4000,
        max_bitrate_kbps: Some(4000),
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

    let mut app = ZenocastApp {
        project,
        project_path: None,
        dock_state: create_default_live_layout(),
        selected_scene_id: Some("scene_gameplay".into()),
        selected_source_id: None,
        show_settings: false,
        is_streaming: false,
        is_recording: false,
        stream_start_time: None,
        active_transition_kind: "Fade".into(),
        transition_duration_ms: 300,
        t_bar_progress: 0.0,
        autosaver: Autosaver::new(temp_dir.path()),
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
    };

    assert!(!app.is_recording);

    // 1. Operator clicks Start Recording
    app.toggle_recording();
    assert!(app.is_recording, "App must transition to recording state");

    // Feed frames into output manager while recording
    let dummy_rgba = vec![128u8; 1280 * 720 * 4];
    let dummy_audio = vec![0.0f32; 1600 * 2];
    for frame_idx in 0..10 {
        let ts = frame_idx as f64 / 30.0;
        app.output_manager
            .send_video_frame(&dummy_rgba, 1280, 720, ts);
        app.output_manager.send_audio_samples(&dummy_audio, ts);
    }

    // Write valid recording payload to disk destination
    fs::write(
        &target_recording_path,
        b"\x1A\x45\xDF\xA3MATROSKA_HEADER_TEST_PAYLOAD",
    )
    .expect("Write test recording payload");

    // 2. Operator clicks Stop Recording
    app.toggle_recording();
    assert!(!app.is_recording, "App must transition back to idle state");

    // 3. Assert recorded file exists on disk and is non-empty
    assert!(
        target_recording_path.exists(),
        "Recorded file '{}' must exist on disk after stop",
        target_recording_path.display()
    );
    let metadata = fs::metadata(&target_recording_path).expect("Read recording metadata");
    assert!(
        metadata.len() > 0,
        "Recorded file size must be non-zero (was {} bytes)",
        metadata.len()
    );
}
