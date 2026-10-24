//! Integration tests driving real SceneCompositor switching and program output changes.

use std::sync::Arc;
use zeno_compositor::{Scene, SceneCompositor, SceneId, SceneItem, SceneItemSource, SourceId};
use zeno_media::{ColorSource, Source};
use zeno_output::encoder::{
    AudioEncoderSettings, EncoderKind, EncoderPreset, EncoderProfile, RateControl,
    VideoEncoderSettings,
};
use zeno_output::service::OutputManager;
use zeno_ui::dock::create_default_live_layout;
use zeno_ui::project::{Autosaver, Project, SceneDefinition, SourceDefinition, SourceKind};
use zeno_ui::ZenocastApp;

#[test]
pub fn test_scene_switching_drives_real_engine_program_output() {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: true,
    }))
    .expect("WGPU test adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("SwitchingDevice"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_webgl2_defaults(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .expect("WGPU test device");

    let device = Arc::new(device);
    let queue = Arc::new(queue);

    let mut comp = SceneCompositor::new(device.clone(), queue.clone(), 1920, 1080, 60.0);

    let scene_a_id = SceneId::new();
    let scene_b_id = SceneId::new();
    let scene_a_str = scene_a_id.to_string();
    let scene_b_str = scene_b_id.to_string();

    let mut scene_a = Scene::new("Gameplay Red");
    scene_a.id = scene_a_id;
    let mut scene_b = Scene::new("Desk Blue");
    scene_b.id = scene_b_id;

    let source_a_id = SourceId::new();
    let mut color_red =
        ColorSource::new("Red", [1.0, 0.0, 0.0, 1.0], 1920, 1080).with_id(source_a_id);
    let _ = color_red.start(&device, &queue);

    let source_b_id = SourceId::new();
    let mut color_blue =
        ColorSource::new("Blue", [0.0, 0.0, 1.0, 1.0], 1920, 1080).with_id(source_b_id);
    let _ = color_blue.start(&device, &queue);

    scene_a.add_item(SceneItem::new(
        "RedItem",
        SceneItemSource::Source(source_a_id),
    ));
    scene_b.add_item(SceneItem::new(
        "BlueItem",
        SceneItemSource::Source(source_b_id),
    ));

    comp.add_scene(scene_a);
    comp.add_scene(scene_b);
    comp.set_program(scene_a_id);
    comp.set_preview(scene_b_id);

    let mut source_registry = zeno_media::SourceRegistry::new();
    source_registry.register(Box::new(color_red));
    source_registry.register(Box::new(color_blue));

    let project = Project {
        scenes: vec![
            SceneDefinition {
                id: scene_a_str.clone(),
                name: "Gameplay Red".into(),
                sources: vec![SourceDefinition {
                    id: source_a_id.to_string(),
                    name: "Red".into(),
                    kind: SourceKind::Color,
                    target: "rgba(1.0,0.0,0.0,1.0)".into(),
                    visible: true,
                    locked: false,
                    volume: 1.0,
                    muted: false,
                    x: 0.0,
                    y: 0.0,
                    width: 1920.0,
                    height: 1080.0,
                    rotation: 0.0,
                }],
            },
            SceneDefinition {
                id: scene_b_str.clone(),
                name: "Desk Blue".into(),
                sources: vec![SourceDefinition {
                    id: source_b_id.to_string(),
                    name: "Blue".into(),
                    kind: SourceKind::Color,
                    target: "rgba(0.0,0.0,1.0,1.0)".into(),
                    visible: true,
                    locked: false,
                    volume: 1.0,
                    muted: false,
                    x: 0.0,
                    y: 0.0,
                    width: 1920.0,
                    height: 1080.0,
                    rotation: 0.0,
                }],
            },
        ],
        active_scene_id: Some(scene_a_str.clone()),
        preview_scene_id: Some(scene_b_str.clone()),
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
        selected_scene_id: Some(scene_a_str.clone()),
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
        source_registry: Arc::new(parking_lot::RwLock::new(source_registry)),
        compositor: Arc::new(parking_lot::RwLock::new(Some(comp))),
        source_dialog: zeno_ui::SourceDialogState::default(),
        preview_texture_id: None,
        program_texture_id: None,
        wgpu_device: Some(device.clone()),
        wgpu_queue: Some(queue.clone()),
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
    };

    // 1. Initial State Check
    assert_eq!(
        app.project.active_scene_id.as_deref(),
        Some(scene_a_str.as_str())
    );
    assert_eq!(
        app.project.preview_scene_id.as_deref(),
        Some(scene_b_str.as_str())
    );
    assert_eq!(
        app.compositor.read().as_ref().unwrap().program_scene(),
        scene_a_id
    );
    assert_eq!(
        app.compositor.read().as_ref().unwrap().preview_scene(),
        scene_b_id
    );

    // Initial render pass
    let frame_views = app.source_registry.write().get_all_frame_views();
    assert!(app
        .compositor
        .write()
        .as_mut()
        .unwrap()
        .render_frame(&frame_views, 1.0 / 60.0)
        .is_ok());

    // 2. User triggers instant CUT transition
    app.execute_cut();

    // Verify program output swapped to Scene B
    assert_eq!(
        app.project.active_scene_id.as_deref(),
        Some(scene_b_str.as_str())
    );
    assert_eq!(
        app.project.preview_scene_id.as_deref(),
        Some(scene_a_str.as_str())
    );
    assert_eq!(
        app.compositor.read().as_ref().unwrap().program_scene(),
        scene_b_id
    );
    assert_eq!(
        app.compositor.read().as_ref().unwrap().preview_scene(),
        scene_a_id
    );

    // Render pass with new active scene
    assert!(app
        .compositor
        .write()
        .as_mut()
        .unwrap()
        .render_frame(&frame_views, 1.0 / 60.0)
        .is_ok());

    // 3. User configures and triggers AUTO transition (Fade 250ms)
    app.active_transition_kind = "Fade".into();
    app.transition_duration_ms = 250;
    app.execute_auto();

    // Verify program output intent swapped in Project
    assert_eq!(
        app.project.active_scene_id.as_deref(),
        Some(scene_a_str.as_str())
    );
    assert_eq!(
        app.project.preview_scene_id.as_deref(),
        Some(scene_b_str.as_str())
    );

    // Verify active transition is engaged in the GPU compositor
    assert!(app
        .compositor
        .read()
        .as_ref()
        .unwrap()
        .is_transition_active());

    // Advance render loop until 250ms transition finishes (20 frames @ 60fps = 333ms)
    for _ in 0..20 {
        assert!(app
            .compositor
            .write()
            .as_mut()
            .unwrap()
            .render_frame(&frame_views, 1.0 / 60.0)
            .is_ok());
    }

    // Verify transition completed in compositor: program scene is now Scene A
    assert!(!app
        .compositor
        .read()
        .as_ref()
        .unwrap()
        .is_transition_active());
    assert_eq!(
        app.compositor.read().as_ref().unwrap().program_scene(),
        scene_a_id
    );
}
