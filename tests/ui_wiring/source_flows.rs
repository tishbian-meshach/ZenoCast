//! Integration tests driving real SourceRegistry and compositor for all 10 source types.

use std::sync::Arc;
use zeno_compositor::SceneCompositor;
use zeno_output::encoder::{
    AudioEncoderSettings, EncoderKind, EncoderPreset, EncoderProfile, RateControl,
    VideoEncoderSettings,
};
use zeno_output::service::OutputManager;
use zeno_ui::dock::create_default_live_layout;
use zeno_ui::project::{Autosaver, Project};
use zeno_ui::ZenocastApp;

fn setup_test_app(
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    mut comp: SceneCompositor,
) -> ZenocastApp {
    let mut project = Project::default();
    project.scenes.retain(|s| s.id == "scene_gameplay");
    project.scenes[0].sources.clear();

    let sc_id = zeno_ui::app::string_to_scene_id("scene_gameplay");
    let mut scene = zeno_compositor::Scene::new("Gameplay Scene");
    scene.id = sc_id;
    comp.add_scene(scene);
    comp.set_program(sc_id);
    comp.set_preview(sc_id);
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
        selected_scene_id: Some("scene_gameplay".into()),
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
        wizard: zeno_ui::FirstRunWizard::new(),
        license_client: Arc::new(zeno_license::LicenseClient::new_mock_valid()),
        license_key_input: String::new(),
        source_registry: Arc::new(parking_lot::RwLock::new(zeno_media::SourceRegistry::new())),
        compositor: Arc::new(parking_lot::RwLock::new(Some(comp))),
        source_dialog: zeno_ui::SourceDialogState::default(),
        preview_texture_id: None,
        program_texture_id: None,
        wgpu_device: Some(device),
        wgpu_queue: Some(queue),
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
pub fn test_add_each_source_type_and_verify_real_frame_output() {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: true,
    }))
    .expect("WGPU test adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("WiringTestSourceDevice"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .expect("WGPU test device");

    let device = Arc::new(device);
    let queue = Arc::new(queue);

    let comp = SceneCompositor::new(device.clone(), queue.clone(), 1920, 1080, 60.0);
    let mut app = setup_test_app(device.clone(), queue.clone(), comp);

    // 10 broadcast source kinds
    let ten_kinds = [
        (
            zeno_ui::SourceKind::DisplayCapture,
            "Display 0",
            "Main Display Capture",
        ),
        (
            zeno_ui::SourceKind::WindowCapture,
            "Zenocast Window",
            "App Window Capture",
        ),
        (
            zeno_ui::SourceKind::Color,
            "rgba(0.2,0.8,0.2,1.0)",
            "Solid Green Background",
        ),
        (
            zeno_ui::SourceKind::Text,
            "MATCH POINT - ROUND 24",
            "Live Lower Third Text",
        ),
        (
            zeno_ui::SourceKind::Image,
            "assets/test_overlay.png",
            "Branded Sponsor Logo",
        ),
        (
            zeno_ui::SourceKind::MediaFile,
            "assets/intro_video.mp4",
            "Tournament Intro Stinger",
        ),
        (
            zeno_ui::SourceKind::CameraFeed,
            "HD Pro Webcam C920",
            "Desk Host Camera",
        ),
        (
            zeno_ui::SourceKind::AudioInput,
            "Focusrite Scarlett Solo",
            "Shure SM7B Mic In",
        ),
        (
            zeno_ui::SourceKind::AudioOutput,
            "Speakers (Realtek Audio)",
            "Desktop Game Audio Loopback",
        ),
        (
            zeno_ui::SourceKind::Browser,
            "https://zenocast.live/overlay",
            "Dynamic HTML5 Leaderboard",
        ),
    ];

    let mut added_ids = Vec::new();

    for (idx, (kind, target, name)) in ten_kinds.iter().enumerate() {
        app.create_and_add_source(zeno_ui::SourceDialogAction::Create {
            name: (*name).to_string(),
            kind: *kind,
            target: (*target).to_string(),
            width: 1920,
            height: 1080,
            color_rgba: [0.2, 0.8, 0.2, 1.0],
            capture_cursor: true,
            file_path: (*target).to_string(),
            text_content: "MATCH POINT - ROUND 24".into(),
            font_size: 48.0,
            url: (*target).to_string(),
            looping: true,
            volume: 1.0,
            muted: false,
        });

        let source_id_str = app
            .selected_source_id
            .clone()
            .expect("Selected source ID must be set");
        let source_id = zeno_ui::app::string_to_source_id(&source_id_str);

        // Assert real Source object exists in SourceRegistry
        let reg = app.source_registry.read();
        assert_eq!(
            reg.count(),
            idx + 1,
            "Source registry must contain newly added source"
        );
        let src = reg
            .get(source_id)
            .expect("Source must exist in registry under its ID");
        assert!(!src.name().is_empty(), "Source name must not be empty");
        drop(reg);

        added_ids.push(source_id_str);
    }

    assert_eq!(app.source_registry.read().count(), 10);
    assert_eq!(app.project.scenes[0].sources.len(), 10);

    // Drive real GPU render tick
    app.source_registry
        .write()
        .update_all(&device, &queue, 1.0 / 60.0);
    let frame_views = app.source_registry.write().get_all_frame_views();
    assert!(
        !frame_views.is_empty(),
        "Registered sources must produce real GPU frame views"
    );

    // Compositor renders all 10 views without error
    let mut comp_lock = app.compositor.write();
    let render_res = comp_lock
        .as_mut()
        .unwrap()
        .render_frame(&frame_views, 1.0 / 60.0);
    assert!(
        render_res.is_ok(),
        "GPU scene compositor renders all 10 sources simultaneously"
    );
    drop(comp_lock);

    // Teardown all 10 sources and assert clean reclamation
    for id_str in &added_ids {
        app.remove_source_by_id(id_str);
    }

    assert_eq!(
        app.source_registry.read().count(),
        0,
        "SourceRegistry must be empty after complete teardown"
    );
    assert_eq!(
        app.project.scenes[0].sources.len(),
        0,
        "Project scene sources must be empty after teardown"
    );
}
