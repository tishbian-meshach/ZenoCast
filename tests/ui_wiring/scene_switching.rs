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

    // Verify program output updated to Scene B, preview remains on Scene B
    assert_eq!(
        app.project.active_scene_id.as_deref(),
        Some(scene_b_str.as_str())
    );
    assert_eq!(
        app.project.preview_scene_id.as_deref(),
        Some(scene_b_str.as_str())
    );
    assert_eq!(
        app.compositor.read().as_ref().unwrap().program_scene(),
        scene_b_id
    );
    assert_eq!(
        app.compositor.read().as_ref().unwrap().preview_scene(),
        scene_b_id
    );

    // Render pass with new active scene
    assert!(app
        .compositor
        .write()
        .as_mut()
        .unwrap()
        .render_frame(&frame_views, 1.0 / 60.0)
        .is_ok());

    // 3. User selects Scene A into Preview, configures and triggers AUTO transition (Fade 250ms)
    app.project.preview_scene_id = Some(scene_a_str.clone());
    app.compositor
        .write()
        .as_mut()
        .unwrap()
        .set_preview(scene_a_id);
    app.active_transition_kind = "Fade".into();
    app.transition_duration_ms = 250;
    app.execute_auto();

    // Verify preview is Scene A, program is Scene B while transitioning
    assert_eq!(
        app.project.preview_scene_id.as_deref(),
        Some(scene_a_str.as_str())
    );
    assert_eq!(
        app.project.active_scene_id.as_deref(),
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

#[test]
pub fn test_independent_preview_and_program_buses() {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: true,
    }))
    .expect("WGPU test adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("BusTestDevice"),
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

    let scene_id = SceneId::new();
    let scene_str = scene_id.to_string();
    let source_id = SourceId::new();

    let color_red = ColorSource::new("Red", [1.0, 0.0, 0.0, 1.0], 1920, 1080).with_id(source_id);

    let mut scene = Scene::new("Live Scene");
    scene.id = scene_id;
    let item = SceneItem::new("Red", SceneItemSource::Source(source_id))
        .with_position(960.0, 540.0)
        .with_scale(1.0, 1.0);
    scene.add_item(item);

    comp.add_scene(scene);
    comp.set_program(scene_id);
    comp.set_preview(scene_id);

    let mut source_registry = zeno_media::SourceRegistry::new();
    source_registry.register(Box::new(color_red));

    let project = Project {
        scenes: vec![SceneDefinition {
            id: scene_str.clone(),
            name: "Live Scene".into(),
            sources: vec![SourceDefinition {
                id: source_id.to_string(),
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
        }],
        active_scene_id: Some(scene_str.clone()),
        preview_scene_id: Some(scene_str.clone()),
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
        selected_scene_id: Some(scene_str.clone()),
        selected_source_id: Some(source_id.to_string()),
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
        file_dialog_path: String::new(),
        rename_scene_id: None,
        rename_scene_buffer: String::new(),
        live_gizmo_state: zeno_ui::TransformGizmoState::default(),
    };

    // 1. Move and scale source in Preview only
    app.sync_source_transform_to_compositor(
        &source_id.to_string(),
        100.0,
        100.0,
        800.0,
        600.0,
        45.0,
    );
    if let Some(sources) = app.project.preview_working_sources_mut() {
        if let Some(src) = sources.iter_mut().find(|s| s.id == source_id.to_string()) {
            src.x = 100.0;
            src.y = 100.0;
            src.width = 800.0;
            src.height = 600.0;
            src.rotation = 45.0;
        }
    }

    // Verify Preview working scene updated in compositor, while Program canonical scene did NOT change
    let comp_lock = app.compositor.read();
    let comp_ref = comp_lock.as_ref().unwrap();

    let preview_item = &comp_ref.preview_working_scene().unwrap().items[0];
    assert_eq!(preview_item.transform.position.x, 500.0); // 100 + 800/2
    assert_eq!(preview_item.transform.position.y, 400.0); // 100 + 600/2
    assert_eq!(preview_item.transform.rotation_degrees, 45.0);

    let program_item = &comp_ref.get_scene(scene_id).unwrap().items[0];
    assert_eq!(program_item.transform.position.x, 960.0); // Untouched
    assert_eq!(program_item.transform.position.y, 540.0);
    assert_eq!(program_item.transform.rotation_degrees, 0.0);

    // Canonical project scene is untouched
    assert_eq!(app.project.scenes[0].sources[0].x, 0.0);
    drop(comp_lock);

    // 2. Drag T-bar halfway: blends between Program and Preview
    app.t_bar_progress = 0.5;
    let mut reg_lock = app.source_registry.write();
    let frame_views = reg_lock.get_all_frame_views();
    let def = zeno_compositor::TransitionDefinition::fade(0.3);
    assert!(app
        .compositor
        .write()
        .as_mut()
        .unwrap()
        .render_transition_preview(scene_id, scene_id, &def, 0.5, &frame_views)
        .is_ok());
    drop(reg_lock);

    // 3. Trigger CUT: commits preview working edits into real scene and program bus
    app.execute_cut();

    assert_eq!(app.project.scenes[0].sources[0].x, 100.0);
    assert_eq!(app.project.scenes[0].sources[0].width, 800.0);

    let comp_lock = app.compositor.read();
    let comp_ref = comp_lock.as_ref().unwrap();
    let program_item_after_cut = &comp_ref.get_scene(scene_id).unwrap().items[0];
    assert_eq!(program_item_after_cut.transform.position.x, 500.0);
    assert_eq!(program_item_after_cut.transform.rotation_degrees, 45.0);
    assert_eq!(comp_ref.program_scene(), scene_id);
    assert_eq!(comp_ref.preview_scene(), scene_id);
}

#[test]
pub fn test_source_changes_refresh_preview_immediately_no_scene_switch() {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: true,
    }))
    .expect("WGPU test adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("RefreshTestDevice"),
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

    let scene_id = SceneId::new();
    let scene_str = scene_id.to_string();
    let source_1_id = SourceId::new();

    let color_red = ColorSource::new("Red", [1.0, 0.0, 0.0, 1.0], 1920, 1080).with_id(source_1_id);

    let mut scene = Scene::new("Test Scene");
    scene.id = scene_id;
    scene.add_item(SceneItem::new("Red", SceneItemSource::Source(source_1_id)));

    comp.add_scene(scene);
    comp.set_program(scene_id);
    comp.set_preview(scene_id);

    let mut source_registry = zeno_media::SourceRegistry::new();
    source_registry.register(Box::new(color_red));

    let project = Project {
        scenes: vec![SceneDefinition {
            id: scene_str.clone(),
            name: "Test Scene".into(),
            sources: vec![SourceDefinition {
                id: source_1_id.to_string(),
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
        }],
        active_scene_id: Some(scene_str.clone()),
        preview_scene_id: Some(scene_str.clone()),
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
        selected_scene_id: Some(scene_str.clone()),
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
        file_dialog_path: String::new(),
        rename_scene_id: None,
        rename_scene_buffer: String::new(),
        live_gizmo_state: zeno_ui::TransformGizmoState::default(),
    };

    // Initial state: 1 source
    assert_eq!(app.project.preview_sources().len(), 1);

    // Operator adds a new Color source
    app.create_and_add_source(zeno_ui::SourceDialogAction::Create {
        name: "Blue Source".into(),
        kind: SourceKind::Color,
        target: "rgba(0.0,0.0,1.0,1.0)".into(),
        width: 1280,
        height: 720,
        color_rgba: [0.0, 0.0, 1.0, 1.0],
        capture_cursor: false,
        file_path: String::new(),
        text_content: String::new(),
        font_size: 32.0,
        url: String::new(),
        looping: false,
        volume: 1.0,
        muted: false,
    });

    // Verification: IMMEDIATELY visible in Preview without scene-switch
    assert_eq!(app.project.preview_sources().len(), 2);
    let comp_lock = app.compositor.read();
    let comp_ref = comp_lock.as_ref().unwrap();
    assert_eq!(comp_ref.preview_working_scene().unwrap().items.len(), 2);
    assert_eq!(comp_ref.get_scene(scene_id).unwrap().items.len(), 2);
    assert_eq!(app.project.scenes[0].sources.len(), 2);
    drop(comp_lock);

    // Render frame to ensure pipeline succeeds
    let mut reg_lock = app.source_registry.write();
    let frame_views = reg_lock.get_all_frame_views();
    assert!(app
        .compositor
        .write()
        .as_mut()
        .unwrap()
        .render_frame(&frame_views, 1.0 / 60.0)
        .is_ok());
    drop(reg_lock);

    // Verify preview renders successfully without error and without needing a scene switch
    assert_eq!(app.project.preview_sources().len(), 2);
}
