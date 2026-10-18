//! Zenocast Studio desktop application shell (`eframe::App`).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use eframe::{App, Frame};
use egui::{vec2, CentralPanel, Context, TopBottomPanel, Window};
use egui_dock::{DockArea, DockState};
use tracing::info;

use crate::dock::{create_default_live_layout, StudioTab, StudioTabViewer};
use crate::project::{Autosaver, Project, SceneDefinition, SourceDefinition, SourceKind};
use crate::source_dialog::{SourceDialogAction, SourceDialogState};
use crate::theme::{apply_theme, AMBER_WARN, GREEN_OK, RED_LIVE, TEXT, TEXT_MUTED};
use crate::wizard::FirstRunWizard;
use zeno_compositor::{Scene, SceneCompositor, SceneItem, SceneItemSource};
use zeno_core::{SceneId, SourceId};
use zeno_license::{AntiTamperGuard, LicenseClient, LicenseState};
use zeno_media::{
    AudioCaptureSource, BrowserSource, CameraCaptureSource, ColorSource, DisplayCaptureSource,
    ImageSource, Source, SourceProperties, SourceRegistry, TextSource, VideoSource,
    WindowCaptureSource,
};
use zeno_output::service::OutputManager;

/// Deterministic conversion from string identifier to SceneId.
pub fn string_to_scene_id(s: &str) -> SceneId {
    use std::str::FromStr;
    if let Ok(id) = SceneId::from_str(s) {
        return id;
    }
    let mut bytes = [0u8; 16];
    for (i, &b) in s.as_bytes().iter().enumerate() {
        bytes[i % 16] ^= b.wrapping_add((i as u8).wrapping_mul(31));
    }
    SceneId(uuid::Uuid::from_bytes(bytes))
}

/// Deterministic conversion from string identifier to SourceId.
pub fn string_to_source_id(s: &str) -> SourceId {
    use std::str::FromStr;
    if let Ok(id) = SourceId::from_str(s) {
        return id;
    }
    let mut bytes = [0u8; 16];
    for (i, &b) in s.as_bytes().iter().enumerate() {
        bytes[i % 16] ^= b.wrapping_add((i as u8).wrapping_mul(37));
    }
    SourceId(uuid::Uuid::from_bytes(bytes))
}

/// Main studio application state.
pub struct ZenocastApp {
    /// Active project state
    pub project: Project,
    /// Project file path on disk if saved
    pub project_path: Option<PathBuf>,
    /// Dockable tabs layout
    pub dock_state: DockState<StudioTab>,
    /// Selected scene in Scenes panel
    pub selected_scene_id: Option<String>,
    /// Selected source in Sources panel
    pub selected_source_id: Option<String>,
    /// Whether settings modal is open
    pub show_settings: bool,
    /// Output streaming state
    pub is_streaming: bool,
    /// Output recording state
    pub is_recording: bool,
    /// Start timestamp for active stream
    pub stream_start_time: Option<Instant>,
    /// Active transition kind
    pub active_transition_kind: String,
    /// Active transition duration in milliseconds
    pub transition_duration_ms: u32,
    /// T-bar position (0.0 to 1.0)
    pub t_bar_progress: f32,
    /// Autosaver instance
    pub autosaver: Autosaver,
    /// Output manager
    pub output_manager: Arc<OutputManager>,
    /// First-run setup and hardware auto-configuration wizard
    pub wizard: FirstRunWizard,
    /// Client-side licensing and anti-tamper manager
    pub license_client: Arc<LicenseClient>,
    /// License key input in settings
    pub license_key_input: String,
    /// Registry owning live capture & media sources
    pub source_registry: Arc<parking_lot::RwLock<SourceRegistry>>,
    /// Live GPU Scene Compositor
    pub compositor: Arc<parking_lot::RwLock<Option<SceneCompositor>>>,
    /// Source modal dialog state
    pub source_dialog: SourceDialogState,
    /// Registered egui texture ID for live Preview monitor
    pub preview_texture_id: Option<egui::TextureId>,
    /// Registered egui texture ID for live Program monitor
    pub program_texture_id: Option<egui::TextureId>,
    /// Reference to WGPU device
    pub wgpu_device: Option<Arc<wgpu::Device>>,
    /// Reference to WGPU queue
    pub wgpu_queue: Option<Arc<wgpu::Queue>>,
}

impl ZenocastApp {
    /// Initializes the studio shell with a default project and layout.
    pub fn new(cc: &eframe::CreationContext) -> Self {
        apply_theme(&cc.egui_ctx);

        let project = Project::default();
        let selected_scene_id = project.active_scene_id.clone();
        let dock_state = create_default_live_layout();

        let v_settings = zeno_output::encoder::VideoEncoderSettings {
            encoder_kind: zeno_output::encoder::EncoderKind::X264,
            width: project.settings.video.output_width,
            height: project.settings.video.output_height,
            fps: project.settings.video.fps,
            rate_control: zeno_output::encoder::RateControl::Cbr,
            bitrate_kbps: project.settings.output.video_bitrate_kbps,
            max_bitrate_kbps: Some(project.settings.output.video_bitrate_kbps),
            keyframe_interval_seconds: project.settings.output.keyframe_interval_seconds,
            preset: zeno_output::encoder::EncoderPreset::VeryFast,
            profile: zeno_output::encoder::EncoderProfile::Main,
            b_frames: 0,
        };

        let a_settings = zeno_output::encoder::AudioEncoderSettings {
            sample_rate: project.settings.audio.sample_rate,
            channels: project.settings.audio.channels,
            bitrate_kbps: project.settings.audio.bitrate_kbps,
        };

        let output_manager = Arc::new(OutputManager::new_dry_run(v_settings, a_settings));

        let source_registry = Arc::new(parking_lot::RwLock::new(SourceRegistry::new()));
        let mut compositor_opt = None;
        let mut preview_texture_id = None;
        let mut program_texture_id = None;
        let mut wgpu_device = None;
        let mut wgpu_queue = None;

        if let Some(ref rstate) = cc.wgpu_render_state {
            let device = Arc::new(rstate.device.clone());
            let queue = Arc::new(rstate.queue.clone());
            let mut comp = SceneCompositor::new(
                device.clone(),
                queue.clone(),
                project.settings.video.output_width,
                project.settings.video.output_height,
                project.settings.video.fps,
            );

            // Register Preview and Program texture views with egui's renderer
            let p_tex_id = rstate.renderer.write().register_native_texture(
                &device,
                comp.preview_view(),
                wgpu::FilterMode::Linear,
            );
            let prog_tex_id = rstate.renderer.write().register_native_texture(
                &device,
                comp.program_view(),
                wgpu::FilterMode::Linear,
            );

            // Populate initial scenes into compositor and default sources
            for sc in &project.scenes {
                let sc_id = string_to_scene_id(&sc.id);
                let mut scene = Scene::new(&sc.name);
                scene.id = sc_id;

                for src_def in &sc.sources {
                    let s_id = string_to_source_id(&src_def.id);
                    let color = match src_def.kind {
                        SourceKind::CameraFeed => [0.85, 0.35, 0.25, 1.0],
                        SourceKind::ArxOverlay => [0.95, 0.65, 0.15, 1.0],
                        _ => [0.15, 0.25, 0.45, 1.0],
                    };
                    let mut color_src = ColorSource::new(
                        &src_def.name,
                        color,
                        src_def.width as u32,
                        src_def.height as u32,
                    )
                    .with_id(s_id);
                    let _ = color_src.start(&device, &queue);
                    source_registry.write().register(Box::new(color_src));

                    let canvas_w = comp.width() as f32;
                    let canvas_h = comp.height() as f32;
                    let item = SceneItem::new(&src_def.name, SceneItemSource::Source(s_id))
                        .with_position(
                            src_def.x + src_def.width / 2.0,
                            src_def.y + src_def.height / 2.0,
                        )
                        .with_scale(src_def.width / canvas_w, src_def.height / canvas_h);
                    scene.add_item(item);
                }

                comp.add_scene(scene);
            }

            if let Some(ref prev) = project.preview_scene_id {
                comp.set_preview(string_to_scene_id(prev));
            }
            if let Some(ref act) = project.active_scene_id {
                comp.set_program(string_to_scene_id(act));
            }

            compositor_opt = Some(comp);
            preview_texture_id = Some(p_tex_id);
            program_texture_id = Some(prog_tex_id);
            wgpu_device = Some(device);
            wgpu_queue = Some(queue);
        }

        Self {
            project,
            project_path: None,
            dock_state,
            selected_scene_id,
            selected_source_id: None,
            show_settings: false,
            is_streaming: false,
            is_recording: false,
            stream_start_time: None,
            active_transition_kind: "Fade".into(),
            transition_duration_ms: 300,
            t_bar_progress: 0.0,
            autosaver: Autosaver::new(
                &std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            ),
            output_manager,
            wizard: {
                let mut w = FirstRunWizard::new();
                w.is_open = false;
                w
            },
            license_client: Arc::new(LicenseClient::new_mock_valid()),
            license_key_input: String::new(),
            source_registry,
            compositor: Arc::new(parking_lot::RwLock::new(compositor_opt)),
            source_dialog: SourceDialogState::default(),
            preview_texture_id,
            program_texture_id,
            wgpu_device,
            wgpu_queue,
        }
    }

    /// Handles instant Cut transition between Preview and Program.
    pub fn execute_cut(&mut self) {
        let temp = self.project.active_scene_id.clone();
        self.project.active_scene_id = self.project.preview_scene_id.clone();
        self.project.preview_scene_id = temp;
        self.t_bar_progress = 0.0;
        if let Some(ref mut comp) = *self.compositor.write() {
            comp.cut();
        }
        info!(
            "Executed Cut transition: live='{:?}'",
            self.project.active_scene_id
        );
    }

    /// Handles Auto transition between Preview and Program.
    pub fn execute_auto(&mut self) {
        let temp = self.project.active_scene_id.clone();
        self.project.active_scene_id = self.project.preview_scene_id.clone();
        self.project.preview_scene_id = temp;
        self.t_bar_progress = 0.0;
        if let Some(ref mut comp) = *self.compositor.write() {
            comp.auto_transition(None);
        }
    }

    /// Creates and adds a new source from dialog action into registry and compositor.
    pub fn create_and_add_source(&mut self, action: SourceDialogAction) {
        match action {
            SourceDialogAction::Create {
                name,
                kind,
                target,
                width,
                height,
                color_rgba,
                capture_cursor,
                file_path,
                text_content,
                font_size,
                url,
                looping,
                volume,
                muted,
            } => {
                let Some(ref sel_scene_id) = self.selected_scene_id else {
                    return;
                };
                let source_id = SourceId::new();
                let source_id_str = source_id.to_string();

                // 1. Construct real backend Source trait object
                let source_obj: Option<Box<dyn zeno_media::Source>> = match kind {
                    SourceKind::Color => Some(Box::new(
                        ColorSource::new(&name, color_rgba, width, height).with_id(source_id),
                    )),
                    SourceKind::DisplayCapture => {
                        let disp_idx = if let Some(stripped) = target.strip_prefix("Display ") {
                            stripped.parse::<usize>().unwrap_or(0)
                        } else {
                            0
                        };
                        if let Ok(mut disp) = DisplayCaptureSource::from_index(disp_idx) {
                            disp.set_capture_cursor(capture_cursor);
                            Some(Box::new(disp.with_id(source_id)))
                        } else if let Ok(mut disp) = DisplayCaptureSource::primary() {
                            disp.set_capture_cursor(capture_cursor);
                            Some(Box::new(disp.with_id(source_id)))
                        } else {
                            Some(Box::new(
                                ColorSource::new(&name, [0.1, 0.3, 0.8, 1.0], width, height)
                                    .with_id(source_id),
                            ))
                        }
                    }
                    SourceKind::WindowCapture => {
                        let mut win = if target.is_empty() || target == "Select Window..." {
                            WindowCaptureSource::default_or_fallback()
                        } else {
                            WindowCaptureSource::from_title(&target)
                        };
                        win.set_capture_cursor(capture_cursor);
                        Some(Box::new(win.with_id(source_id)))
                    }
                    SourceKind::CameraFeed => {
                        let cam = CameraCaptureSource::from_name_or_id(&target).with_id(source_id);
                        Some(Box::new(cam))
                    }
                    SourceKind::Text => Some(Box::new(
                        TextSource::new(&name, text_content, font_size, color_rgba, width, height)
                            .with_id(source_id),
                    )),
                    SourceKind::Image => Some(Box::new(
                        ImageSource::new(&name, file_path).with_id(source_id),
                    )),
                    SourceKind::MediaFile => {
                        let mut vid = VideoSource::new(&name, file_path).with_id(source_id);
                        vid.set_looping(looping);
                        Some(Box::new(vid))
                    }
                    SourceKind::AudioInput => {
                        let mut aud =
                            AudioCaptureSource::new_input(&name, &target).with_id(source_id);
                        aud.set_volume(volume);
                        aud.set_muted(muted);
                        Some(Box::new(aud))
                    }
                    SourceKind::AudioOutput => {
                        let mut aud =
                            AudioCaptureSource::new_output(&name, &target).with_id(source_id);
                        aud.set_volume(volume);
                        aud.set_muted(muted);
                        Some(Box::new(aud))
                    }
                    SourceKind::Browser => Some(Box::new(
                        BrowserSource::new(&url, width, height).with_id(source_id),
                    )),
                    _ => Some(Box::new(
                        ColorSource::new(&name, color_rgba, width, height).with_id(source_id),
                    )),
                };

                if let Some(mut src) = source_obj {
                    if let (Some(dev), Some(q)) = (&self.wgpu_device, &self.wgpu_queue) {
                        let _ = src.start(dev, q);
                    }
                    self.source_registry.write().register(src);
                }

                // 2. Add to project scene definition
                if let Some(scene) = self
                    .project
                    .scenes
                    .iter_mut()
                    .find(|s| &s.id == sel_scene_id)
                {
                    scene.sources.push(SourceDefinition {
                        id: source_id_str.clone(),
                        name: name.clone(),
                        kind,
                        target,
                        visible: true,
                        locked: false,
                        volume,
                        muted,
                        x: 0.0,
                        y: 0.0,
                        width: width as f32,
                        height: height as f32,
                    });
                }

                // 3. Add to Compositor scene items
                if let Some(ref mut comp) = *self.compositor.write() {
                    let canvas_w = comp.width() as f32;
                    let canvas_h = comp.height() as f32;
                    let sc_id = string_to_scene_id(sel_scene_id);
                    if let Some(scene) = comp.get_scene_mut(sc_id) {
                        let item = SceneItem::new(name, SceneItemSource::Source(source_id))
                            .with_position(canvas_w / 2.0, canvas_h / 2.0)
                            .with_scale((width as f32) / canvas_w, (height as f32) / canvas_h);
                        scene.add_item(item);
                    }
                }

                self.selected_source_id = Some(source_id_str);
            }
            SourceDialogAction::Update {
                id,
                name,
                target,
                width,
                height,
                color_rgba,
                capture_cursor,
                file_path,
                text_content,
                font_size,
                url,
                looping,
                volume,
                muted,
            } => {
                let src_id = string_to_source_id(&id);
                let Some(ref sel_scene_id) = self.selected_scene_id else {
                    return;
                };

                // Update Project
                if let Some(scene) = self
                    .project
                    .scenes
                    .iter_mut()
                    .find(|s| &s.id == sel_scene_id)
                {
                    if let Some(src) = scene.sources.iter_mut().find(|s| s.id == id) {
                        src.name = name.clone();
                        src.target = target;
                        src.width = width as f32;
                        src.height = height as f32;
                        src.volume = volume;
                        src.muted = muted;
                    }
                }

                // Update Backend Source properties
                if let Some(src) = self.source_registry.write().get_mut(src_id) {
                    src.set_name(name.clone());
                    let props = match src.kind() {
                        zeno_media::SourceKind::Color => SourceProperties::Color {
                            color: color_rgba,
                            width,
                            height,
                        },
                        zeno_media::SourceKind::DisplayCapture => SourceProperties::Display {
                            monitor_index: 0,
                            cursor_visible: capture_cursor,
                        },
                        zeno_media::SourceKind::WindowCapture => SourceProperties::Window {
                            window_title: name.clone(),
                            cursor_visible: capture_cursor,
                        },
                        zeno_media::SourceKind::CameraCapture => SourceProperties::Camera {
                            device_name: name.clone(),
                            format_index: 0,
                            resolution: (width, height),
                        },
                        zeno_media::SourceKind::Browser => SourceProperties::Browser {
                            url: url.clone(),
                            width,
                            height,
                            custom_css: String::new(),
                        },
                        zeno_media::SourceKind::Text => SourceProperties::Text {
                            text: text_content,
                            font_family: "Inter".into(),
                            font_size,
                            color: color_rgba,
                        },
                        zeno_media::SourceKind::Image => SourceProperties::Image {
                            path: file_path,
                            scale_mode: "Fit".into(),
                        },
                        zeno_media::SourceKind::MediaFile => SourceProperties::Video {
                            path: file_path,
                            looping,
                        },
                        zeno_media::SourceKind::AudioInput => SourceProperties::AudioInput {
                            device_name: name.clone(),
                            volume,
                            muted,
                        },
                        zeno_media::SourceKind::AudioOutput => SourceProperties::AudioOutput {
                            device_name: name.clone(),
                            volume,
                            muted,
                        },
                        _ => src.properties(),
                    };
                    if let (Some(dev), Some(q)) = (&self.wgpu_device, &self.wgpu_queue) {
                        let _ = src.update_properties(&props, dev, q);
                    }
                }

                // Update Compositor SceneItem
                if let Some(ref mut comp) = *self.compositor.write() {
                    let sc_id = string_to_scene_id(sel_scene_id);
                    if let Some(scene) = comp.get_scene_mut(sc_id) {
                        for item in &mut scene.items {
                            if let SceneItemSource::Source(id) = item.source {
                                if id == src_id {
                                    item.name = name.clone();
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Removes a source from project, compositor, and stops it in the registry.
    pub fn remove_source_by_id(&mut self, source_id_str: &str) {
        let src_id = string_to_source_id(source_id_str);
        if let Some(ref sel_scene_id) = self.selected_scene_id {
            if let Some(scene) = self
                .project
                .scenes
                .iter_mut()
                .find(|s| &s.id == sel_scene_id)
            {
                scene.sources.retain(|s| s.id != source_id_str);
            }
            if let Some(ref mut comp) = *self.compositor.write() {
                let sc_id = string_to_scene_id(sel_scene_id);
                if let Some(scene) = comp.get_scene_mut(sc_id) {
                    scene.items.retain(|item| {
                        if let SceneItemSource::Source(id) = item.source {
                            id != src_id
                        } else {
                            true
                        }
                    });
                }
            }
        }
        self.source_registry.write().unregister(src_id);
        if self.selected_source_id.as_deref() == Some(source_id_str) {
            self.selected_source_id = None;
        }
    }

    /// Toggles visibility of a source in project and compositor.
    pub fn toggle_source_visibility(&mut self, source_id_str: &str) {
        let src_id = string_to_source_id(source_id_str);
        if let Some(ref sel_scene_id) = self.selected_scene_id {
            let mut new_vis = true;
            if let Some(scene) = self
                .project
                .scenes
                .iter_mut()
                .find(|s| &s.id == sel_scene_id)
            {
                if let Some(src) = scene.sources.iter_mut().find(|s| s.id == source_id_str) {
                    src.visible = !src.visible;
                    new_vis = src.visible;
                }
            }
            if let Some(ref mut comp) = *self.compositor.write() {
                let sc_id = string_to_scene_id(sel_scene_id);
                if let Some(scene) = comp.get_scene_mut(sc_id) {
                    for item in &mut scene.items {
                        if let SceneItemSource::Source(id) = item.source {
                            if id == src_id {
                                item.visible = new_vis;
                            }
                        }
                    }
                }
            }
        }
    }

    /// Toggles lock state of a source.
    pub fn toggle_source_lock(&mut self, source_id_str: &str) {
        if let Some(ref sel_scene_id) = self.selected_scene_id {
            if let Some(scene) = self
                .project
                .scenes
                .iter_mut()
                .find(|s| &s.id == sel_scene_id)
            {
                if let Some(src) = scene.sources.iter_mut().find(|s| s.id == source_id_str) {
                    src.locked = !src.locked;
                }
            }
        }
    }

    /// Adds a new scene to project and compositor.
    pub fn add_new_scene(&mut self) {
        let count = self.project.scenes.len() + 1;
        let id = format!("scene_{}", count);
        let name = format!("Scene {}", count);
        self.project.scenes.push(SceneDefinition {
            id: id.clone(),
            name: name.clone(),
            sources: Vec::new(),
        });
        let sc_id = string_to_scene_id(&id);
        if let Some(ref mut comp) = *self.compositor.write() {
            let mut s = Scene::new(&name);
            s.id = sc_id;
            comp.add_scene(s);
        }
        self.selected_scene_id = Some(id);
    }

    /// Removes a scene by ID from project and compositor.
    pub fn remove_scene_by_id(&mut self, scene_id_str: &str) {
        self.project.scenes.retain(|s| s.id != scene_id_str);
        if self.selected_scene_id.as_deref() == Some(scene_id_str) {
            self.selected_scene_id = self.project.scenes.first().map(|s| s.id.clone());
        }
        if self.project.preview_scene_id.as_deref() == Some(scene_id_str) {
            self.project.preview_scene_id = self.project.scenes.first().map(|s| s.id.clone());
        }
        if self.project.active_scene_id.as_deref() == Some(scene_id_str) {
            self.project.active_scene_id = self.project.scenes.first().map(|s| s.id.clone());
        }
    }

    /// Toggles live streaming output.
    pub fn toggle_streaming(&mut self) {
        if self.is_streaming {
            self.output_manager.stop_streaming();
            self.is_streaming = false;
            self.stream_start_time = None;
            info!("Stopped live streaming");
        } else {
            let now_sec = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let license_state = self.license_client.get_state(now_sec);
            if !AntiTamperGuard::can_broadcast_output(&license_state) {
                tracing::warn!("Streaming refused: restricted license mode or tamper detected");
                return;
            }

            let url = format!(
                "{}/{}",
                self.project
                    .settings
                    .output
                    .stream_url
                    .trim_end_matches('/'),
                self.project.settings.output.stream_key
            );
            if self.output_manager.start_streaming(&url).is_ok() {
                self.is_streaming = true;
                self.stream_start_time = Some(Instant::now());
                info!("Started live streaming to '{}'", url);
            }
        }
    }

    /// Toggles local recording output.
    pub fn toggle_recording(&mut self) {
        if self.is_recording {
            let _ = self
                .output_manager
                .stop_recording(self.project.settings.output.remux_to_mp4);
            self.is_recording = false;
            info!("Stopped recording");
        } else {
            let now_sec = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            let license_state = self.license_client.get_state(now_sec);
            if !AntiTamperGuard::can_broadcast_output(&license_state) {
                tracing::warn!("Recording refused: restricted license mode or tamper detected");
                return;
            }

            let path =
                PathBuf::from(&self.project.settings.output.record_directory).join("recording.mkv");
            if self.output_manager.start_recording(path).is_ok() {
                self.is_recording = true;
                info!("Started recording");
            }
        }
    }

    /// Formats stream elapsed time string.
    pub fn stream_duration_string(&self) -> String {
        if let Some(start) = self.stream_start_time {
            let secs = start.elapsed().as_secs();
            format!(
                "{:02}:{:02}:{:02}",
                secs / 3600,
                (secs % 3600) / 60,
                secs % 60
            )
        } else {
            "00:00:00".into()
        }
    }
}

impl App for ZenocastApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        // Render tick for live sources and compositor
        if let (Some(device), Some(queue)) = (&self.wgpu_device, &self.wgpu_queue) {
            let now_sec = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs_f64();

            let mut registry = self.source_registry.write();
            registry.update_all(device, queue, now_sec);
            let frame_views = registry.get_all_frame_views();
            drop(registry);

            if let Some(ref mut comp) = *self.compositor.write() {
                if let Some(ref prv) = self.project.preview_scene_id {
                    comp.set_preview(string_to_scene_id(prv));
                }
                if let Some(ref act) = self.project.active_scene_id {
                    comp.set_program(string_to_scene_id(act));
                }

                if self.t_bar_progress > 0.0 && self.t_bar_progress < 1.0 {
                    let from = comp.program_scene();
                    let to = comp.preview_scene();
                    let def = comp.resolve_transition(from, to);
                    let _ = comp.render_transition_preview(
                        from,
                        to,
                        &def,
                        self.t_bar_progress,
                        &frame_views,
                    );
                } else {
                    let _ = comp.render_frame(&frame_views, 1.0 / 60.0);
                }
            }
            ctx.request_repaint();
        }

        // 30-second periodic autosave check
        let _ = self.autosaver.check_and_save(&mut self.project);

        // Top Menu Bar
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project").clicked() {
                        self.project = Project::default();
                        self.project_path = None;
                        ui.close_menu();
                    }
                    if ui.button("Save Project").clicked() {
                        if let Some(ref path) = self.project_path {
                            let _ = self.project.save_to_file(path);
                        } else {
                            let path = PathBuf::from("project.arxproj");
                            let _ = self.project.save_to_file(&path);
                            self.project_path = Some(path);
                        }
                        ui.close_menu();
                    }
                    if ui.button("Settings").clicked() {
                        self.show_settings = true;
                        ui.close_menu();
                    }
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Mode", |ui| {
                    let _ = ui.selectable_label(true, "Live Studio");
                    let _ = ui.selectable_label(false, "Edit Overlay");
                    let _ = ui.selectable_label(false, "Data Control");
                });

                ui.menu_button("Tools", |ui| {
                    if ui.button("Auto-Configuration Wizard...").clicked() {
                        self.wizard.is_open = true;
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.is_streaming {
                        ui.colored_label(RED_LIVE, "● LIVE");
                    }
                    if self.is_recording {
                        ui.colored_label(RED_LIVE, "● REC");
                    }
                });
            });
        });

        // Bottom Status Bar
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.colored_label(
                    TEXT_MUTED,
                    format!(
                        "Zenocast v0.1.0 | 1080p60 | {}",
                        self.stream_duration_string()
                    ),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(TEXT_MUTED, "Hardware NVENC/AMF Ready");
                });
            });
        });

        // Trigger flags for TabViewer actions
        let mut trigger_cut = false;
        let mut trigger_auto = false;
        let mut trigger_toggle_stream = false;
        let mut trigger_toggle_record = false;
        let mut trigger_open_add_source = false;
        let mut trigger_open_edit_source: Option<String> = None;
        let mut trigger_remove_source: Option<String> = None;
        let mut trigger_toggle_source_visibility: Option<String> = None;
        let mut trigger_toggle_source_lock: Option<String> = None;
        let mut trigger_add_scene = false;
        let mut trigger_remove_scene: Option<String> = None;

        let bitrate_kbps = self.project.settings.output.video_bitrate_kbps;
        let stream_time = self.stream_duration_string();

        // Central Dock Area
        CentralPanel::default().show(ctx, |ui| {
            let mut viewer = StudioTabViewer {
                project: &mut self.project,
                selected_scene_id: &mut self.selected_scene_id,
                selected_source_id: &mut self.selected_source_id,
                is_streaming: self.is_streaming,
                is_recording: self.is_recording,
                fps: 60.0,
                dropped_percent: 0.0,
                bitrate_kbps,
                stream_time,
                active_transition_kind: &mut self.active_transition_kind,
                transition_duration_ms: &mut self.transition_duration_ms,
                t_bar_progress: &mut self.t_bar_progress,
                trigger_cut: &mut trigger_cut,
                trigger_auto: &mut trigger_auto,
                trigger_toggle_stream: &mut trigger_toggle_stream,
                trigger_toggle_record: &mut trigger_toggle_record,
                preview_texture_id: self.preview_texture_id,
                program_texture_id: self.program_texture_id,
                trigger_open_add_source: &mut trigger_open_add_source,
                trigger_open_edit_source: &mut trigger_open_edit_source,
                trigger_remove_source: &mut trigger_remove_source,
                trigger_toggle_source_visibility: &mut trigger_toggle_source_visibility,
                trigger_toggle_source_lock: &mut trigger_toggle_source_lock,
                trigger_add_scene: &mut trigger_add_scene,
                trigger_remove_scene: &mut trigger_remove_scene,
            };

            DockArea::new(&mut self.dock_state)
                .style(egui_dock::Style::from_egui(ui.style().as_ref()))
                .show_inside(ui, &mut viewer);
        });

        // Handle trigger events
        if trigger_cut {
            self.execute_cut();
        }
        if trigger_auto {
            self.execute_auto();
        }
        if trigger_toggle_stream {
            self.toggle_streaming();
        }
        if trigger_toggle_record {
            self.toggle_recording();
        }
        if trigger_open_add_source {
            self.source_dialog.open_add();
        }
        if let Some(ref edit_id) = trigger_open_edit_source {
            if let Some(ref sel_scene) = self.selected_scene_id {
                if let Some(scene) = self.project.scenes.iter().find(|s| &s.id == sel_scene) {
                    if let Some(src) = scene.sources.iter().find(|s| &s.id == edit_id) {
                        self.source_dialog.open_edit(
                            &src.id,
                            &src.name,
                            src.kind,
                            &src.target,
                            src.width,
                            src.height,
                            src.volume,
                            src.muted,
                        );
                    }
                }
            }
        }
        if let Some(ref rem_id) = trigger_remove_source {
            self.remove_source_by_id(rem_id);
        }
        if let Some(ref vis_id) = trigger_toggle_source_visibility {
            self.toggle_source_visibility(vis_id);
        }
        if let Some(ref lock_id) = trigger_toggle_source_lock {
            self.toggle_source_lock(lock_id);
        }
        if trigger_add_scene {
            self.add_new_scene();
        }
        if let Some(ref rem_sc) = trigger_remove_scene {
            self.remove_scene_by_id(rem_sc);
        }

        // Render Source Modal Dialog if open
        if let Some(action) = self.source_dialog.ui(ctx) {
            self.create_and_add_source(action);
        }

        // Settings Dialog Window
        if self.show_settings {
            Window::new("Studio Settings")
                .collapsible(false)
                .resizable(false)
                .default_size(vec2(450.0, 350.0))
                .show(ctx, |ui| {
                    ui.heading("Video Settings");
                    ui.horizontal(|ui| {
                        ui.label("Resolution:");
                        ui.colored_label(
                            TEXT,
                            format!(
                                "{}x{}",
                                self.project.settings.video.output_width,
                                self.project.settings.video.output_height
                            ),
                        );
                        ui.label("FPS:");
                        ui.colored_label(TEXT, format!("{:.0}", self.project.settings.video.fps));
                    });

                    ui.separator();
                    ui.heading("Audio Settings");
                    ui.horizontal(|ui| {
                        ui.label("Sample Rate:");
                        ui.colored_label(
                            TEXT,
                            format!("{} Hz", self.project.settings.audio.sample_rate),
                        );
                        ui.label("Channels:");
                        ui.colored_label(TEXT, format!("{}", self.project.settings.audio.channels));
                    });

                    ui.separator();
                    ui.heading("Output & Stream Settings");
                    ui.horizontal(|ui| {
                        ui.label("Service:");
                        ui.text_edit_singleline(&mut self.project.settings.output.stream_service);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Server URL:");
                        ui.text_edit_singleline(&mut self.project.settings.output.stream_url);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Stream Key:");
                        ui.text_edit_singleline(&mut self.project.settings.output.stream_key);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Bitrate (kbps):");
                        ui.add(egui::Slider::new(
                            &mut self.project.settings.output.video_bitrate_kbps,
                            1000..=20000,
                        ));
                    });

                    ui.separator();
                    ui.heading("Licensing & Device Activation");
                    let now_sec = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    let license_state = self.license_client.get_state(now_sec);
                    match &license_state {
                        LicenseState::Valid { certificate } => {
                            ui.horizontal(|ui| {
                                ui.label("Status:");
                                ui.colored_label(GREEN_OK, "Licensed & Active");
                            });
                            ui.horizontal(|ui| {
                                ui.label("Edition:");
                                ui.colored_label(TEXT, certificate.edition.display_name());
                            });
                        }
                        LicenseState::GracePeriod {
                            offline_since_sec, ..
                        } => {
                            ui.horizontal(|ui| {
                                ui.label("Status:");
                                ui.colored_label(AMBER_WARN, "Offline Grace Period Active");
                            });
                            let days_offline = (now_sec.saturating_sub(*offline_since_sec)) / 86400;
                            let days_left = 14u64.saturating_sub(days_offline);
                            ui.label(format!(
                                "Offline for {} days ({} days remaining)",
                                days_offline, days_left
                            ));
                        }
                        LicenseState::Restricted { reason } => {
                            ui.horizontal(|ui| {
                                ui.label("Status:");
                                ui.colored_label(RED_LIVE, "Restricted Mode (Output Gated)");
                            });
                            ui.colored_label(RED_LIVE, reason.message());
                        }
                    }

                    if let Some(wm) = AntiTamperGuard::get_watermark_text(&license_state) {
                        ui.colored_label(AMBER_WARN, format!("Watermark Active: '{}'", wm));
                    }

                    ui.horizontal(|ui| {
                        ui.label("Device Fingerprint:");
                        let fp = zeno_license::get_device_fingerprint();
                        let short_fp = if fp.len() >= 16 { &fp[..16] } else { &fp };
                        ui.colored_label(TEXT_MUTED, format!("{}...", short_fp));
                    });

                    ui.horizontal(|ui| {
                        ui.label("License Key:");
                        ui.text_edit_singleline(&mut self.license_key_input);
                        if ui.button("Deactivate This Device").clicked() {
                            self.license_client.clear_certificate();
                        }
                    });

                    ui.hyperlink_to(
                        "Manage Seats & Renewals Online",
                        "https://zenocast.live/portal",
                    );

                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Close").clicked() {
                            self.show_settings = false;
                        }
                    });
                });
        }

        // Render First-Run Setup Wizard if open
        self.wizard.ui(ctx, &mut self.project.settings);
    }
}
