//! Zenocast Studio desktop application shell (`eframe::App`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use eframe::{App, Frame};
use egui::{vec2, CentralPanel, Context, TopBottomPanel, Window};
use egui_dock::{DockArea, DockState};
use tracing::info;

use crate::data_panel::{ui_esports_data_panel, DataPanelState};
use crate::dock::{create_default_live_layout, StudioTab, StudioTabViewer};
use crate::editor::{
    ui_assets_panel, ui_canvas, ui_layers_panel, ui_properties_panel, ui_timeline, LeftTab,
    OverlayEditor, TimelineMode,
};
use crate::hotkey_registry::{ActionCategory, ActionId, KeyCombo, RebindTarget};
use crate::project::{Autosaver, Project, SceneDefinition, SourceDefinition, SourceKind};
use crate::source_dialog::{SourceDialogAction, SourceDialogState};
use crate::theme::{apply_theme, ACCENT, AMBER_WARN, RED_LIVE, TEXT, TEXT_MUTED};
use crate::transform_gizmo::TransformGizmoState;
use crate::wizard::FirstRunWizard;
use zeno_compositor::{
    Scene, SceneCompositor, SceneItem, SceneItemSource, SlideDirection, TransitionDefinition,
    WipeDirection,
};
use zeno_control::ControlState;
use zeno_core::{SceneId, SourceId};
use zeno_data::DataStore;
use zeno_license::{AntiTamperGuard, LicenseClient, LicenseState};
use zeno_media::{
    AudioCaptureSource, BrowserSource, CameraCaptureSource, ColorSource, DisplayCaptureSource,
    ImageSource, Source, SourceProperties, SourceRegistry, TextSource, VideoSource,
    WindowCaptureSource,
};
use zeno_output::service::OutputManager;

/// Studio operation modes selectable from the top Mode menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Standard live production multiview with scenes, sources, mixer, transitions, and streaming.
    LiveStudio,
    /// Motion-graphics overlay composition editor with timeline, layers, and bezier keyframes.
    EditOverlay,
    /// Rapid esports tournament data control panel for scores, teams, rosters, and quick switching.
    DataControl,
}

/// Tabs in the Studio Settings window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    #[default]
    Video,
    Audio,
    Output,
    Hotkeys,
    Licensing,
}

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
    /// Active application layout / mode
    pub app_mode: AppMode,
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
    /// Transient inline warning message for sources (e.g. "Unlock to delete")
    pub source_warning_message: Option<(String, Instant)>,
    /// Whether Studio Mode (dual Preview/Program monitors) is enabled, or Normal Mode (single Program monitor).
    pub studio_mode: bool,
    /// Motion graphics overlay editor state (Edit Overlay mode)
    pub overlay_editor: OverlayEditor,
    /// Timeline mode (DopeSheet vs CurveEditor)
    pub timeline_mode: TimelineMode,
    /// Esports fast data control state (Data Control mode)
    pub data_panel_state: DataPanelState,
    /// Shared live data store
    pub data_store: Arc<DataStore>,
    /// Shared control server state
    pub control_state: Arc<ControlState>,
    /// Master audio output volume multiplier [0.0..1.5]
    pub master_volume: f32,
    /// Master audio mute toggle
    pub master_muted: bool,
    /// Whether Open Project modal dialog is open
    pub show_open_dialog: bool,
    /// Whether Save As modal dialog is open
    pub show_save_as_dialog: bool,
    /// File path buffer for Open / Save As dialogs
    pub file_dialog_path: String,
    /// Active scene being renamed, if any
    pub rename_scene_id: Option<String>,
    /// Text buffer for scene renaming
    pub rename_scene_buffer: String,
    /// Live Studio on-canvas transform gizmo state
    pub live_gizmo_state: TransformGizmoState,
    /// Active tab in Studio Settings window
    pub settings_tab: SettingsTab,
    /// Hotkey search query in Settings -> Hotkeys
    pub hotkey_search_filter: String,
    /// Active interactive hotkey rebinding target
    pub rebinding_target: Option<crate::hotkey_registry::RebindTarget>,
    /// Global hotkey manager owning background OS keyboard hooks
    pub global_hotkey_manager: Option<global_hotkey::GlobalHotKeyManager>,
    /// Map from global hotkey ID to action target
    pub global_hotkey_map: HashMap<u32, crate::hotkey_registry::RebindTarget>,
    /// Previous active scene ID (for OBS push-to-talk-style scene switching)
    pub previous_scene_id: Option<String>,
    /// Live Studio undo snapshots
    pub studio_undo_stack: Vec<Project>,
    /// Live Studio redo snapshots
    pub studio_redo_stack: Vec<Project>,
    /// Sources actively held in push-to-mute state
    pub push_to_mute_active: HashMap<String, bool>,
    /// Sources actively held in push-to-talk state
    pub push_to_talk_active: HashMap<String, bool>,
    /// Internal clipboard for copied scene source item (OBS Copy / Paste Reference / Paste Duplicate)
    pub clipboard_source: Option<SourceDefinition>,
    /// Internal clipboard for copied filter chain (OBS Copy / Paste Filters)
    pub clipboard_filters: Option<zeno_core::FilterChain>,
    /// Filter modal dialog state
    pub filter_dialog: crate::filter_dialog::FilterDialogState,
    /// Whether Preview monitor interaction is locked (OBS Lock Preview)
    pub preview_locked: bool,
    /// Preview scaling mode (OBS Preview Scaling)
    pub preview_scaling_mode: crate::project::PreviewScalingMode,
}

impl ZenocastApp {
    /// Initializes the studio shell with a default project and layout.
    pub fn new(cc: &eframe::CreationContext) -> Self {
        apply_theme(&cc.egui_ctx);

        let user_prefs = crate::preferences::UserPreferences::load();
        let studio_mode = user_prefs.studio_mode;

        let mut project = Project::default();
        let selected_scene_id = if studio_mode {
            project
                .preview_scene_id
                .clone()
                .or_else(|| project.active_scene_id.clone())
        } else {
            project.preview_scene_id = project.active_scene_id.clone();
            project.active_scene_id.clone()
        };
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
                    let mut item = SceneItem::new(&src_def.name, SceneItemSource::Source(s_id))
                        .with_position(
                            src_def.x + src_def.width / 2.0,
                            src_def.y + src_def.height / 2.0,
                        )
                        .with_scale(src_def.width / canvas_w, src_def.height / canvas_h);
                    item.visible = src_def.visible;
                    item.locked = src_def.locked;
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

        let mut app = Self {
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
            source_warning_message: None,
            studio_mode,
            clipboard_source: None,
            clipboard_filters: None,
            filter_dialog: crate::filter_dialog::FilterDialogState::default(),
            preview_locked: false,
            preview_scaling_mode: crate::project::PreviewScalingMode::ScaleToWindow,
            app_mode: AppMode::LiveStudio,
            overlay_editor: OverlayEditor::default(),
            timeline_mode: TimelineMode::DopeSheet,
            data_panel_state: DataPanelState::default(),
            data_store: Arc::new(DataStore::new()),
            control_state: {
                let s = Arc::new(DataStore::new());
                Arc::new(ControlState::new(s))
            },
            master_volume: 1.0,
            master_muted: false,
            show_open_dialog: false,
            show_save_as_dialog: false,
            file_dialog_path: "project.arxproj".to_string(),
            rename_scene_id: None,
            rename_scene_buffer: String::new(),
            live_gizmo_state: TransformGizmoState::default(),
            settings_tab: SettingsTab::default(),
            hotkey_search_filter: String::new(),
            rebinding_target: None,
            global_hotkey_manager: global_hotkey::GlobalHotKeyManager::new().ok(),
            global_hotkey_map: HashMap::new(),
            previous_scene_id: None,
            studio_undo_stack: Vec::new(),
            studio_redo_stack: Vec::new(),
            push_to_mute_active: HashMap::new(),
            push_to_talk_active: HashMap::new(),
        };

        app.sync_global_hotkeys();

        if let Ok(proj_path) = std::env::var("ZENOCAST_DEV_PROJECT") {
            let path = PathBuf::from(proj_path);
            if path.exists() {
                let _ = app.open_project_from_path(path);
            }
        }

        app
    }

    /// Initializes a headless studio instance without GPU surface bindings.
    pub fn new_headless() -> Self {
        let project = Project::default();
        let selected_scene_id = project
            .preview_scene_id
            .clone()
            .or_else(|| project.active_scene_id.clone());
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
            compositor: Arc::new(parking_lot::RwLock::new(None)),
            source_dialog: SourceDialogState::default(),
            preview_texture_id: None,
            program_texture_id: None,
            wgpu_device: None,
            wgpu_queue: None,
            source_warning_message: None,
            studio_mode: false,
            clipboard_source: None,
            clipboard_filters: None,
            filter_dialog: crate::filter_dialog::FilterDialogState::default(),
            preview_locked: false,
            preview_scaling_mode: crate::project::PreviewScalingMode::ScaleToWindow,
            app_mode: AppMode::LiveStudio,
            overlay_editor: OverlayEditor::default(),
            timeline_mode: TimelineMode::DopeSheet,
            data_panel_state: DataPanelState::default(),
            data_store: Arc::new(DataStore::new()),
            control_state: {
                let s = Arc::new(DataStore::new());
                Arc::new(ControlState::new(s))
            },
            master_volume: 1.0,
            master_muted: false,
            show_open_dialog: false,
            show_save_as_dialog: false,
            file_dialog_path: "project.arxproj".to_string(),
            rename_scene_id: None,
            rename_scene_buffer: String::new(),
            live_gizmo_state: TransformGizmoState::default(),
            settings_tab: SettingsTab::default(),
            hotkey_search_filter: String::new(),
            rebinding_target: None,
            global_hotkey_manager: None,
            global_hotkey_map: HashMap::new(),
            previous_scene_id: None,
            studio_undo_stack: Vec::new(),
            studio_redo_stack: Vec::new(),
            push_to_mute_active: HashMap::new(),
            push_to_talk_active: HashMap::new(),
        }
    }

    /// Handles instant Cut transition between Preview and Program.
    pub fn execute_cut(&mut self) {
        self.project.commit_preview_working_sources();
        self.project.active_scene_id = self.project.preview_scene_id.clone();
        self.t_bar_progress = 0.0;
        if let Some(ref mut comp) = *self.compositor.write() {
            comp.cut();
        }
        info!(
            "Executed Cut transition: live='{:?}'",
            self.project.active_scene_id
        );
    }

    /// Handles Auto transition between Preview and Program using active transition settings.
    pub fn execute_auto(&mut self) {
        self.t_bar_progress = 0.0;
        if let Some(ref mut comp) = *self.compositor.write() {
            let dur_sec = (self.transition_duration_ms as f64) / 1000.0;
            let def = match self.active_transition_kind.as_str() {
                "Cut" => TransitionDefinition::cut(),
                "Fade" => TransitionDefinition::fade(dur_sec),
                "Slide" => TransitionDefinition::slide(SlideDirection::Left, true, dur_sec),
                "Wipe" => TransitionDefinition::wipe(WipeDirection::Horizontal, 0.1, dur_sec),
                _ => TransitionDefinition::fade(dur_sec),
            };
            comp.auto_transition(Some(def));
        }
        info!(
            "Executed Auto transition ({}ms {}): live='{:?}'",
            self.transition_duration_ms, self.active_transition_kind, self.project.active_scene_id
        );
    }

    /// Directly switches live Program output to the designated scene (used in Normal Mode and double-clicks).
    pub fn switch_to_scene_direct(&mut self, scene_id_str: &str) {
        self.project.commit_preview_working_sources();
        self.selected_scene_id = Some(scene_id_str.to_string());
        self.project.preview_scene_id = Some(scene_id_str.to_string());
        if let Some(scene) = self.project.scenes.iter().find(|s| s.id == scene_id_str) {
            self.project.preview_working_sources =
                Some((scene_id_str.to_string(), scene.sources.clone()));
        }
        if let Some(ref mut comp) = *self.compositor.write() {
            let sc_id = string_to_scene_id(scene_id_str);
            comp.set_preview(sc_id);
            if self.active_transition_kind == "Cut" || self.transition_duration_ms == 0 {
                comp.set_program(sc_id);
                comp.cut();
            } else {
                let dur_sec = (self.transition_duration_ms as f64) / 1000.0;
                let def = match self.active_transition_kind.as_str() {
                    "Fade" => TransitionDefinition::fade(dur_sec),
                    "Slide" => TransitionDefinition::slide(SlideDirection::Left, true, dur_sec),
                    "Wipe" => TransitionDefinition::wipe(WipeDirection::Horizontal, 0.1, dur_sec),
                    _ => TransitionDefinition::fade(dur_sec),
                };
                comp.auto_transition(Some(def));
            }
        }
        self.project.active_scene_id = Some(scene_id_str.to_string());
        info!("Switched live output directly to scene '{}'", scene_id_str);
    }

    /// Toggles between Normal Mode (single Program monitor) and Studio Mode (dual monitors).
    pub fn toggle_studio_mode(&mut self) {
        self.studio_mode = !self.studio_mode;
        if self.studio_mode {
            // Switching into Studio Mode duplicates current single monitor into Preview+Program
            if self.project.preview_scene_id.is_none() {
                self.project.preview_scene_id = self.project.active_scene_id.clone();
            }
            if let Some(ref sc_id) = self.project.preview_scene_id {
                if let Some(scene) = self.project.scenes.iter().find(|s| &s.id == sc_id) {
                    self.project.preview_working_sources =
                        Some((sc_id.clone(), scene.sources.clone()));
                }
            }
        } else {
            // Switching back to Normal Mode collapses to Program only:
            // Auto-commit unsaved edits and keep Program as the single visible feed
            self.project.commit_preview_working_sources();
            self.selected_scene_id = self.project.active_scene_id.clone();
            self.project.preview_scene_id = self.project.active_scene_id.clone();
            if let Some(ref sc_id) = self.project.active_scene_id {
                if let Some(scene) = self.project.scenes.iter().find(|s| &s.id == sc_id) {
                    self.project.preview_working_sources =
                        Some((sc_id.clone(), scene.sources.clone()));
                }
            }
        }
        let prefs = crate::preferences::UserPreferences {
            studio_mode: self.studio_mode,
        };
        prefs.save();
    }

    /// Resets project to a fresh default state.
    pub fn reset_to_new_project(&mut self) {
        self.project = Project::default();
        self.project_path = None;
        self.selected_scene_id = self
            .project
            .preview_scene_id
            .clone()
            .or_else(|| self.project.active_scene_id.clone());
        self.source_registry.write().clear();

        if let Some(ref mut comp) = *self.compositor.write() {
            let canvas_w = comp.width() as f32;
            let canvas_h = comp.height() as f32;
            for scene_def in &self.project.scenes {
                let sc_id = string_to_scene_id(&scene_def.id);
                let mut scene = Scene::new(&scene_def.name);
                scene.id = sc_id;
                for src_def in &scene_def.sources {
                    let src_id = string_to_source_id(&src_def.id);
                    let mut item = SceneItem::new(&src_def.name, SceneItemSource::Source(src_id))
                        .with_position(canvas_w / 2.0, canvas_h / 2.0)
                        .with_scale(src_def.width / canvas_w, src_def.height / canvas_h);
                    item.visible = src_def.visible;
                    item.locked = src_def.locked;
                    scene.add_item(item);
                }
                comp.add_scene(scene);
            }
            if let Some(ref prev) = self.project.preview_scene_id {
                comp.set_preview(string_to_scene_id(prev));
            }
            if let Some(ref act) = self.project.active_scene_id {
                comp.set_program(string_to_scene_id(act));
            }
        }
        info!("Reset to new project");
    }

    /// Loads project from disk and synchronizes compositor and sources.
    pub fn open_project_from_path(
        &mut self,
        path: PathBuf,
    ) -> Result<(), crate::project::ProjectError> {
        let loaded = Project::load_from_file(&path)?;
        self.project = loaded;
        self.project_path = Some(path.clone());
        self.selected_scene_id = self
            .project
            .preview_scene_id
            .clone()
            .or_else(|| self.project.active_scene_id.clone());
        self.source_registry.write().clear();

        if let Some(ref mut comp) = *self.compositor.write() {
            let canvas_w = comp.width() as f32;
            let canvas_h = comp.height() as f32;
            for scene_def in &self.project.scenes {
                let sc_id = string_to_scene_id(&scene_def.id);
                let mut scene = Scene::new(&scene_def.name);
                scene.id = sc_id;
                for src_def in &scene_def.sources {
                    let src_id = string_to_source_id(&src_def.id);
                    let mut item = SceneItem::new(&src_def.name, SceneItemSource::Source(src_id))
                        .with_position(canvas_w / 2.0, canvas_h / 2.0)
                        .with_scale(src_def.width / canvas_w, src_def.height / canvas_h);
                    item.visible = src_def.visible;
                    item.locked = src_def.locked;
                    scene.add_item(item);
                }
                comp.add_scene(scene);
            }
            if let Some(ref prev) = self.project.preview_scene_id {
                comp.set_preview(string_to_scene_id(prev));
            }
            if let Some(ref act) = self.project.active_scene_id {
                comp.set_program(string_to_scene_id(act));
            }
        }
        info!("Opened project from '{}'", path.display());
        Ok(())
    }

    /// Saves project to disk at specified path.
    pub fn save_project_to_path(
        &mut self,
        path: PathBuf,
    ) -> Result<(), crate::project::ProjectError> {
        self.project.save_to_file(&path)?;
        self.project_path = Some(path.clone());
        info!("Saved project to '{}'", path.display());
        Ok(())
    }

    /// Renames an existing scene in project and compositor.
    pub fn rename_scene(&mut self, scene_id_str: &str, new_name: &str) {
        if let Some(scene) = self
            .project
            .scenes
            .iter_mut()
            .find(|s| s.id == scene_id_str)
        {
            scene.name = new_name.to_string();
        }
        if let Some(ref mut comp) = *self.compositor.write() {
            let sc_id = string_to_scene_id(scene_id_str);
            if let Some(scene) = comp.get_scene_mut(sc_id) {
                scene.name = new_name.to_string();
            }
        }
    }

    /// Moves a scene up in the list order.
    pub fn move_scene_up(&mut self, scene_id_str: &str) {
        if let Some(idx) = self
            .project
            .scenes
            .iter()
            .position(|s| s.id == scene_id_str)
        {
            if idx > 0 {
                self.project.scenes.swap(idx, idx - 1);
            }
        }
    }

    /// Moves a scene down in the list order.
    pub fn move_scene_down(&mut self, scene_id_str: &str) {
        if let Some(idx) = self
            .project
            .scenes
            .iter()
            .position(|s| s.id == scene_id_str)
        {
            if idx + 1 < self.project.scenes.len() {
                self.project.scenes.swap(idx, idx + 1);
            }
        }
    }

    /// Moves a source forward (closer to foreground / up in UI list) and syncs compositor.
    pub fn move_source_forward(&mut self, scene_id_str: &str, source_id_str: &str) {
        if let Some(scene) = self
            .project
            .scenes
            .iter_mut()
            .find(|s| s.id == scene_id_str)
        {
            if let Some(idx) = scene.sources.iter().position(|s| s.id == source_id_str) {
                if idx + 1 < scene.sources.len() {
                    scene.sources.swap(idx, idx + 1);
                }
            }
        }
        self.sync_scene_sources_order_to_compositor(scene_id_str);
    }

    /// Moves a source backward (closer to background / down in UI list) and syncs compositor.
    pub fn move_source_backward(&mut self, scene_id_str: &str, source_id_str: &str) {
        if let Some(scene) = self
            .project
            .scenes
            .iter_mut()
            .find(|s| s.id == scene_id_str)
        {
            if let Some(idx) = scene.sources.iter().position(|s| s.id == source_id_str) {
                if idx > 0 {
                    scene.sources.swap(idx, idx - 1);
                }
            }
        }
        self.sync_scene_sources_order_to_compositor(scene_id_str);
    }

    /// Moves a source to the front (topmost / foreground) and syncs compositor.
    pub fn move_source_to_front(&mut self, scene_id_str: &str, source_id_str: &str) {
        if let Some(scene) = self
            .project
            .scenes
            .iter_mut()
            .find(|s| s.id == scene_id_str)
        {
            if let Some(idx) = scene.sources.iter().position(|s| s.id == source_id_str) {
                if idx + 1 < scene.sources.len() {
                    let s = scene.sources.remove(idx);
                    scene.sources.push(s);
                }
            }
        }
        self.sync_scene_sources_order_to_compositor(scene_id_str);
    }

    /// Moves a source to the back (bottommost / background) and syncs compositor.
    pub fn move_source_to_back(&mut self, scene_id_str: &str, source_id_str: &str) {
        if let Some(scene) = self
            .project
            .scenes
            .iter_mut()
            .find(|s| s.id == scene_id_str)
        {
            if let Some(idx) = scene.sources.iter().position(|s| s.id == source_id_str) {
                if idx > 0 {
                    let s = scene.sources.remove(idx);
                    scene.sources.insert(0, s);
                }
            }
        }
        self.sync_scene_sources_order_to_compositor(scene_id_str);
    }

    /// Reorders a source from `from_idx` to `to_idx` in a scene and syncs compositor.
    pub fn reorder_source(&mut self, scene_id_str: &str, from_idx: usize, to_idx: usize) {
        if let Some(scene) = self
            .project
            .scenes
            .iter_mut()
            .find(|s| s.id == scene_id_str)
        {
            if from_idx < scene.sources.len() && to_idx < scene.sources.len() && from_idx != to_idx
            {
                let s = scene.sources.remove(from_idx);
                scene.sources.insert(to_idx, s);
            }
        }
        self.sync_scene_sources_order_to_compositor(scene_id_str);
    }

    /// Synchronizes compositor Scene items order to match the project scene definition sources order.
    pub fn sync_scene_sources_order_to_compositor(&mut self, scene_id_str: &str) {
        let source_ids: Vec<SourceId> = self
            .project
            .preview_sources()
            .iter()
            .map(|s| string_to_source_id(&s.id))
            .collect();
        if let Some(ref mut comp) = *self.compositor.write() {
            comp.sync_preview_items_order(&source_ids);
        }
        if let Some(scene_def) = self.project.scenes.iter().find(|s| s.id == scene_id_str) {
            if let Some(ref mut comp) = *self.compositor.write() {
                let sc_id = string_to_scene_id(scene_id_str);
                if let Some(comp_scene) = comp.get_scene_mut(sc_id) {
                    let mut reordered = Vec::with_capacity(comp_scene.items.len());
                    for src_def in &scene_def.sources {
                        let s_id = string_to_source_id(&src_def.id);
                        if let Some(pos) = comp_scene.items.iter().position(|it| {
                            if let SceneItemSource::Source(id) = it.source {
                                id == s_id
                            } else {
                                false
                            }
                        }) {
                            reordered.push(comp_scene.items.remove(pos));
                        }
                    }
                    reordered.append(&mut comp_scene.items);
                    comp_scene.items = reordered;
                }
            }
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
                        let ext = std::path::Path::new(&file_path)
                            .extension()
                            .and_then(|e| e.to_str())
                            .map(|e| e.to_ascii_lowercase())
                            .unwrap_or_default();
                        let is_video = matches!(ext.as_str(), "mp4" | "mov" | "mkv" | "webm");
                        if is_video {
                            let mut vid = VideoSource::new(&name, file_path).with_id(source_id);
                            vid.set_looping(looping);
                            Some(Box::new(vid))
                        } else {
                            Some(Box::new(
                                ImageSource::new(&name, file_path).with_id(source_id),
                            ))
                        }
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

                // 2. Add to project scene definition and preview working sources
                let src_def = SourceDefinition {
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
                    rotation: 0.0,
                    ..Default::default()
                };
                if let Some(scene) = self
                    .project
                    .scenes
                    .iter_mut()
                    .find(|s| &s.id == sel_scene_id)
                {
                    scene.sources.push(src_def.clone());
                }
                if let Some(sources) = self.project.preview_working_sources_mut() {
                    if !sources.iter().any(|s| s.id == source_id_str) {
                        sources.push(src_def);
                    }
                }

                // 3. Add to Compositor scene items and Preview working scene
                if let Some(ref mut comp) = *self.compositor.write() {
                    let canvas_w = comp.width() as f32;
                    let canvas_h = comp.height() as f32;
                    let sc_id = string_to_scene_id(sel_scene_id);
                    if let Some(scene) = comp.get_scene_mut(sc_id) {
                        let item = SceneItem::new(name.clone(), SceneItemSource::Source(source_id))
                            .with_position(canvas_w / 2.0, canvas_h / 2.0)
                            .with_scale((width as f32) / canvas_w, (height as f32) / canvas_h);
                        scene.add_item(item);
                    }
                    let item = SceneItem::new(name, SceneItemSource::Source(source_id))
                        .with_position(canvas_w / 2.0, canvas_h / 2.0)
                        .with_scale((width as f32) / canvas_w, (height as f32) / canvas_h);
                    comp.add_preview_item(item);
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
                let src_def_opt = self
                    .project
                    .scenes
                    .iter()
                    .flat_map(|sc| &sc.sources)
                    .find(|s| s.id == id);
                let underlying_str = src_def_opt
                    .map(|s| s.underlying_source_id().to_string())
                    .unwrap_or_else(|| id.clone());
                let src_id = string_to_source_id(&underlying_str);
                if self.selected_scene_id.is_none() {
                    return;
                }

                // Update Project / Preview working sources across all referencing instances
                if let Some(sources) = self.project.preview_working_sources_mut() {
                    for src in sources
                        .iter_mut()
                        .filter(|s| s.id == id || s.underlying_source_id() == underlying_str)
                    {
                        src.name = name.clone();
                        src.target = target.clone();
                        src.width = width as f32;
                        src.height = height as f32;
                        src.volume = volume;
                        src.muted = muted;
                    }
                }
                for scene in self.project.scenes.iter_mut() {
                    for src in scene
                        .sources
                        .iter_mut()
                        .filter(|s| s.id == id || s.underlying_source_id() == underlying_str)
                    {
                        src.name = name.clone();
                        src.target = target.clone();
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

                // Update Compositor Preview SceneItem
                if let Some(ref mut comp) = *self.compositor.write() {
                    if let Some(scene) = comp.preview_working_scene_mut() {
                        for item in &mut scene.items {
                            if let SceneItemSource::Source(sid) = item.source {
                                if sid == src_id {
                                    item.name = name.clone();
                                }
                            }
                        }
                    }
                }
            }
            SourceDialogAction::OpenFilters { id } => {
                self.open_source_filters(&id);
            }
        }
    }

    /// Removes a source from project, compositor, and stops it in the registry.
    pub fn remove_source_by_id(&mut self, source_id_str: &str) {
        let is_locked = self
            .project
            .preview_sources()
            .iter()
            .find(|s| s.id == source_id_str)
            .map(|s| s.locked)
            .unwrap_or(false);
        if is_locked {
            self.source_warning_message = Some(("Unlock to delete".to_string(), Instant::now()));
            return;
        }

        let underlying_str = self
            .project
            .preview_sources()
            .iter()
            .find(|s| s.id == source_id_str)
            .map(|s| s.underlying_source_id().to_string())
            .unwrap_or_else(|| {
                self.project
                    .scenes
                    .iter()
                    .flat_map(|sc| &sc.sources)
                    .find(|s| s.id == source_id_str)
                    .map(|s| s.underlying_source_id().to_string())
                    .unwrap_or_else(|| source_id_str.to_string())
            });
        let underlying_id = string_to_source_id(&underlying_str);
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
                        item.id != src_id.0
                            && match item.source {
                                SceneItemSource::Source(id) => id != src_id,
                                _ => true,
                            }
                    });
                }
            }
        }
        if let Some(sources) = self.project.preview_working_sources_mut() {
            sources.retain(|s| s.id != source_id_str);
        }
        if let Some(ref mut comp) = *self.compositor.write() {
            if let Some(scene) = comp.preview_working_scene_mut() {
                scene.items.retain(|item| {
                    item.id != src_id.0
                        && match item.source {
                            SceneItemSource::Source(id) => id != src_id,
                            _ => true,
                        }
                });
            }
        }
        self.source_registry.write().release_ref(underlying_id);
        if self.selected_source_id.as_deref() == Some(source_id_str) {
            self.selected_source_id = None;
        }
    }

    /// Toggles visibility of a source in project and compositor.
    pub fn toggle_source_visibility(&mut self, source_id_str: &str) {
        let src_id = string_to_source_id(source_id_str);
        let current_vis = self
            .project
            .preview_sources()
            .iter()
            .find(|s| s.id == source_id_str)
            .map(|s| s.visible)
            .unwrap_or_else(|| {
                self.project
                    .scenes
                    .iter()
                    .flat_map(|sc| &sc.sources)
                    .find(|s| s.id == source_id_str)
                    .map(|s| s.visible)
                    .unwrap_or(true)
            });
        let new_vis = !current_vis;

        // 1. Update project saved scenes
        for scene in &mut self.project.scenes {
            if let Some(src) = scene.sources.iter_mut().find(|s| s.id == source_id_str) {
                src.visible = new_vis;
            }
        }

        // 2. Update preview working sources if active
        if let Some(sources) = self.project.preview_working_sources_mut() {
            if let Some(src) = sources.iter_mut().find(|s| s.id == source_id_str) {
                src.visible = new_vis;
            }
        }

        // 3. Update compositor scenes (immediately affects Program output)
        if let Some(ref mut comp) = *self.compositor.write() {
            for scene_def in &self.project.scenes {
                let sc_id = string_to_scene_id(&scene_def.id);
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
            // 4. Update preview working scene (immediately affects Preview output)
            comp.update_preview_item_visibility(src_id, new_vis);
        }
    }

    /// Toggles lock state of a source in project and compositor.
    pub fn toggle_source_lock(&mut self, source_id_str: &str) {
        let src_id = string_to_source_id(source_id_str);
        let current_lock = self
            .project
            .preview_sources()
            .iter()
            .find(|s| s.id == source_id_str)
            .map(|s| s.locked)
            .unwrap_or_else(|| {
                self.project
                    .scenes
                    .iter()
                    .flat_map(|sc| &sc.sources)
                    .find(|s| s.id == source_id_str)
                    .map(|s| s.locked)
                    .unwrap_or(false)
            });
        let new_lock = !current_lock;

        // 1. Update project saved scenes
        for scene in &mut self.project.scenes {
            if let Some(src) = scene.sources.iter_mut().find(|s| s.id == source_id_str) {
                src.locked = new_lock;
            }
        }

        // 2. Update preview working sources if active
        if let Some(sources) = self.project.preview_working_sources_mut() {
            if let Some(src) = sources.iter_mut().find(|s| s.id == source_id_str) {
                src.locked = new_lock;
            }
        }

        // 3. Update compositor scenes
        if let Some(ref mut comp) = *self.compositor.write() {
            for scene_def in &self.project.scenes {
                let sc_id = string_to_scene_id(&scene_def.id);
                if let Some(scene) = comp.get_scene_mut(sc_id) {
                    for item in &mut scene.items {
                        if let SceneItemSource::Source(id) = item.source {
                            if id == src_id {
                                item.locked = new_lock;
                            }
                        }
                    }
                }
            }
            // 4. Update preview working scene
            comp.update_preview_item_lock(src_id, new_lock);
        }
    }

    /// Synchronizes a source's position, size, and rotation from project to compositor SceneItem.
    pub fn sync_source_transform_to_compositor(
        &mut self,
        source_id_str: &str,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        rotation: f32,
    ) {
        if let Some(ref mut comp) = *self.compositor.write() {
            let s_id = string_to_source_id(source_id_str);
            let canvas_w = comp.width() as f32;
            let canvas_h = comp.height() as f32;
            comp.update_preview_item_transform(
                s_id,
                x + width / 2.0,
                y + height / 2.0,
                width / canvas_w,
                height / canvas_h,
                rotation,
            );
        }
    }

    /// Copies the currently selected source to the internal clipboard.
    pub fn copy_selected_source(&mut self) {
        if let Some(ref sel_id) = self.selected_source_id {
            if let Some(src) = self
                .project
                .preview_sources()
                .iter()
                .find(|s| &s.id == sel_id)
            {
                self.clipboard_source = Some(src.clone());
            }
        }
    }

    /// Pastes the clipboard source as a reference (linked instance).
    pub fn paste_source_reference(&mut self) {
        let Some(clip) = self.clipboard_source.clone() else {
            return;
        };
        self.studio_snapshot_for_undo();
        let mut new_item = clip.clone();
        new_item.id = uuid::Uuid::new_v4().to_string();
        new_item.x += 20.0;
        new_item.y += 20.0;
        let underlying_str = clip.underlying_source_id().to_string();
        new_item.source_id = Some(underlying_str.clone());
        let new_id = new_item.id.clone();
        let underlying_source_id = string_to_source_id(&underlying_str);

        // Increment reference count in SourceRegistry
        self.source_registry.write().add_ref(underlying_source_id);

        if let Some(srcs) = self.project.preview_working_sources_mut() {
            srcs.push(new_item.clone());
        }
        if let Some(ref sel_scene_id) = self.selected_scene_id {
            if let Some(scene) = self
                .project
                .scenes
                .iter_mut()
                .find(|s| &s.id == sel_scene_id)
            {
                scene.sources.push(new_item.clone());
            }
        }
        if let Some(ref mut comp) = *self.compositor.write() {
            let canvas_w = comp.width() as f32;
            let canvas_h = comp.height() as f32;
            let mut item = SceneItem::new(
                new_item.name.clone(),
                SceneItemSource::Source(underlying_source_id),
            )
            .with_position(
                new_item.x + new_item.width / 2.0,
                new_item.y + new_item.height / 2.0,
            )
            .with_scale(new_item.width / canvas_w, new_item.height / canvas_h);
            item.id = string_to_source_id(&new_id).0;
            item.visible = new_item.visible;
            item.locked = new_item.locked;
            comp.add_preview_item(item);
        }
        self.selected_source_id = Some(new_id);
    }

    /// Pastes the clipboard source as a duplicate (independent clone).
    pub fn paste_source_duplicate(&mut self) {
        let Some(clip) = self.clipboard_source.clone() else {
            return;
        };
        self.studio_snapshot_for_undo();
        let mut new_item = clip.clone();
        new_item.id = uuid::Uuid::new_v4().to_string();
        new_item.name = format!("{} (Copy)", new_item.name);
        new_item.x += 20.0;
        new_item.y += 20.0;
        let new_id = new_item.id.clone();
        let orig_underlying_str = clip.underlying_source_id();
        let orig_underlying_id = string_to_source_id(orig_underlying_str);

        let dev = self.wgpu_device.as_deref();
        let q = self.wgpu_queue.as_deref();
        let new_src_id = self.source_registry.write().duplicate_source(
            orig_underlying_id,
            Some(new_item.name.clone()),
            dev,
            q,
        );
        let new_underlying_id = new_src_id.unwrap_or_else(|| string_to_source_id(&new_id));
        new_item.source_id = Some(new_underlying_id.to_string());

        if let Some(srcs) = self.project.preview_working_sources_mut() {
            srcs.push(new_item.clone());
        }
        if let Some(ref sel_scene_id) = self.selected_scene_id {
            if let Some(scene) = self
                .project
                .scenes
                .iter_mut()
                .find(|s| &s.id == sel_scene_id)
            {
                scene.sources.push(new_item.clone());
            }
        }
        if let Some(ref mut comp) = *self.compositor.write() {
            let canvas_w = comp.width() as f32;
            let canvas_h = comp.height() as f32;
            let mut item = SceneItem::new(
                new_item.name.clone(),
                SceneItemSource::Source(new_underlying_id),
            )
            .with_position(
                new_item.x + new_item.width / 2.0,
                new_item.y + new_item.height / 2.0,
            )
            .with_scale(new_item.width / canvas_w, new_item.height / canvas_h);
            item.id = string_to_source_id(&new_id).0;
            item.visible = new_item.visible;
            item.locked = new_item.locked;
            comp.add_preview_item(item);
        }
        self.selected_source_id = Some(new_id);
    }

    /// Saves a screenshot of the preview monitor or a specific source to disk.
    pub fn save_screenshot(&mut self, source_id: Option<&str>, _is_preview: bool) {
        let name_prefix = source_id.unwrap_or("Screenshot");
        let default_name = format!("{}.png", name_prefix);
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(&default_name)
            .add_filter("PNG Image", &["png"])
            .save_file()
        {
            self.source_warning_message = Some((
                format!("Saved screenshot to {}", path.display()),
                std::time::Instant::now(),
            ));
            tracing::info!("Screenshot saved to {:?}", path);
        }
    }

    /// Resizes base and output canvas to match the selected source's native resolution.
    pub fn resize_output_to_source(&mut self, source_id: &str) {
        if let Some(src) = self
            .project
            .preview_sources()
            .iter()
            .find(|s| s.id == source_id)
        {
            let w = src.width.round() as u32;
            let h = src.height.round() as u32;
            if w > 0 && h > 0 {
                self.studio_snapshot_for_undo();
                self.project.settings.video.base_width = w;
                self.project.settings.video.base_height = h;
                self.project.settings.video.output_width = w;
                self.project.settings.video.output_height = h;
                self.source_warning_message = Some((
                    format!("Output resized to {}x{}", w, h),
                    std::time::Instant::now(),
                ));
            }
        }
    }

    /// Opens the filter dialog modal for the given source ID.
    pub fn open_source_filters(&mut self, source_id_str: &str) {
        if let Some(src) = self
            .project
            .preview_sources()
            .into_iter()
            .find(|s| s.id == source_id_str)
        {
            self.filter_dialog
                .open(src.id.clone(), src.name.clone(), src.filters.clone());
        }
    }

    /// Copies the filters of the specified source to the internal clipboard.
    pub fn copy_source_filters(&mut self, source_id_str: &str) {
        if let Some(src) = self
            .project
            .preview_sources()
            .into_iter()
            .find(|s| s.id == source_id_str)
        {
            self.clipboard_filters = Some(src.filters.clone());
            self.source_warning_message = Some((
                "Filters copied to clipboard".to_string(),
                std::time::Instant::now(),
            ));
        }
    }

    /// Pastes the clipboard filters onto the specified source.
    pub fn paste_source_filters(&mut self, source_id_str: &str) {
        let clip_opt = self.clipboard_filters.clone();
        if let Some(clip) = clip_opt {
            self.studio_snapshot_for_undo();
            let filters = clip.clone();
            self.sync_source_filters(source_id_str, filters.clone());
            if self.filter_dialog.is_open()
                && self.filter_dialog.source_id.as_deref() == Some(source_id_str)
            {
                self.filter_dialog.filters = filters;
            }
            self.source_warning_message = Some((
                "Filters pasted successfully".to_string(),
                std::time::Instant::now(),
            ));
        }
    }

    /// Synchronizes a modified filter chain to project data and live compositor SceneItems.
    pub fn sync_source_filters(&mut self, source_id_str: &str, filters: zeno_core::FilterChain) {
        // 1. Update in preview working sources
        if let Some(srcs) = self.project.preview_working_sources_mut() {
            if let Some(s) = srcs.iter_mut().find(|s| s.id == source_id_str) {
                s.filters = filters.clone();
            }
        }

        // 2. Update in project canonical scenes
        for sc in &mut self.project.scenes {
            if let Some(s) = sc.sources.iter_mut().find(|s| s.id == source_id_str) {
                s.filters = filters.clone();
            }
        }

        // 3. Update in SceneCompositor preview & program scene items
        let s_id = string_to_source_id(source_id_str);
        if let Some(ref mut comp) = *self.compositor.write() {
            comp.update_item_filters(s_id, filters);
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
        if let Some(ref mut comp) = *self.compositor.write() {
            comp.remove_scene(string_to_scene_id(scene_id_str));
        }
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

    /// Synchronizes OS-level global hotkeys using `global-hotkey`.
    pub fn sync_global_hotkeys(&mut self) {
        let Some(ref manager) = self.global_hotkey_manager else {
            return;
        };
        let mut new_map = HashMap::new();

        let reg = &self.project.settings.hotkeys.registry;

        // Standard actions
        for meta in crate::hotkey_registry::MASTER_ACTION_DEFINITIONS {
            if meta.global_eligible && reg.is_global(meta.id) {
                if let Some(combo) = reg.get_combo(meta.id) {
                    if let Some(hk) = combo.to_global_hotkey() {
                        let id = hk.id();
                        let _ = manager.register(hk);
                        new_map.insert(id, crate::hotkey_registry::RebindTarget::Action(meta.id));
                    }
                }
            }
        }

        // Per-scene global hotkeys
        for (sc_id, sc_hk) in &reg.scene_hotkeys {
            if sc_hk.switch_to_global {
                if let Some(ref s) = sc_hk.switch_to {
                    if let Some(combo) = crate::hotkey_registry::KeyCombo::parse(s) {
                        if let Some(hk) = combo.to_global_hotkey() {
                            let id = hk.id();
                            let _ = manager.register(hk);
                            new_map.insert(
                                id,
                                crate::hotkey_registry::RebindTarget::SceneSwitch(sc_id.clone()),
                            );
                        }
                    }
                }
            }
            if sc_hk.switch_back_global {
                if let Some(ref s) = sc_hk.switch_back {
                    if let Some(combo) = crate::hotkey_registry::KeyCombo::parse(s) {
                        if let Some(hk) = combo.to_global_hotkey() {
                            let id = hk.id();
                            let _ = manager.register(hk);
                            new_map.insert(
                                id,
                                crate::hotkey_registry::RebindTarget::SceneSwitchBack(
                                    sc_id.clone(),
                                ),
                            );
                        }
                    }
                }
            }
        }

        // Per-source global hotkeys
        for (src_id, src_hk) in &reg.source_hotkeys {
            if src_hk.mute_global {
                if let Some(ref s) = src_hk.mute {
                    if let Some(combo) = crate::hotkey_registry::KeyCombo::parse(s) {
                        if let Some(hk) = combo.to_global_hotkey() {
                            let id = hk.id();
                            let _ = manager.register(hk);
                            new_map.insert(
                                id,
                                crate::hotkey_registry::RebindTarget::SourceMute(src_id.clone()),
                            );
                        }
                    }
                }
            }
            if src_hk.show_hide_global {
                if let Some(ref s) = src_hk.show_hide {
                    if let Some(combo) = crate::hotkey_registry::KeyCombo::parse(s) {
                        if let Some(hk) = combo.to_global_hotkey() {
                            let id = hk.id();
                            let _ = manager.register(hk);
                            new_map.insert(
                                id,
                                crate::hotkey_registry::RebindTarget::SourceShowHide(
                                    src_id.clone(),
                                ),
                            );
                        }
                    }
                }
            }
        }

        self.global_hotkey_map = new_map;
    }

    /// Dispatches an action bound to a generic rebind target.
    pub fn dispatch_target_action(&mut self, target: &crate::hotkey_registry::RebindTarget) {
        match target {
            crate::hotkey_registry::RebindTarget::Action(id) => self.dispatch_action(*id),
            crate::hotkey_registry::RebindTarget::SceneSwitch(sc_id) => {
                self.switch_to_scene_by_id(sc_id);
            }
            crate::hotkey_registry::RebindTarget::SceneSwitchBack(_) => {
                self.switch_to_previous_scene();
            }
            crate::hotkey_registry::RebindTarget::SourceMute(src_id) => {
                self.toggle_source_mute(src_id);
            }
            crate::hotkey_registry::RebindTarget::SourcePushToMute(_)
            | crate::hotkey_registry::RebindTarget::SourcePushToTalk(_) => {}
            crate::hotkey_registry::RebindTarget::SourceShowHide(src_id) => {
                self.toggle_source_visibility_by_id(src_id);
            }
        }
    }

    /// Switches the active Program scene directly to `scene_id`.
    pub fn switch_to_scene_by_id(&mut self, scene_id: &str) {
        if self.project.scenes.iter().any(|s| s.id == scene_id) {
            self.previous_scene_id = self.project.active_scene_id.clone();
            self.project.active_scene_id = Some(scene_id.to_string());
            self.selected_scene_id = Some(scene_id.to_string());
            if let Some(ref mut comp) = *self.compositor.write() {
                comp.set_program(string_to_scene_id(scene_id));
            }
            info!("Directly switched to scene '{}'", scene_id);
        }
    }

    /// Switches back to the previous scene.
    pub fn switch_to_previous_scene(&mut self) {
        if let Some(prev) = self.previous_scene_id.clone() {
            self.switch_to_scene_by_id(&prev);
        }
    }

    /// Advances to the next scene in list order.
    pub fn next_scene(&mut self) {
        if self.project.scenes.is_empty() {
            return;
        }
        let cur_id = self.project.active_scene_id.as_deref().unwrap_or("");
        let cur_idx = self
            .project
            .scenes
            .iter()
            .position(|s| s.id == cur_id)
            .unwrap_or(0);
        let next_idx = (cur_idx + 1) % self.project.scenes.len();
        let next_id = self.project.scenes[next_idx].id.clone();
        self.switch_to_scene_by_id(&next_id);
    }

    /// Steps backward to the previous scene in list order.
    pub fn previous_scene(&mut self) {
        if self.project.scenes.is_empty() {
            return;
        }
        let cur_id = self.project.active_scene_id.as_deref().unwrap_or("");
        let cur_idx = self
            .project
            .scenes
            .iter()
            .position(|s| s.id == cur_id)
            .unwrap_or(0);
        let prev_idx = (cur_idx + self.project.scenes.len() - 1) % self.project.scenes.len();
        let prev_id = self.project.scenes[prev_idx].id.clone();
        self.switch_to_scene_by_id(&prev_id);
    }

    /// Saves a snapshot of current project for Live Studio undo.
    pub fn studio_snapshot_for_undo(&mut self) {
        self.studio_undo_stack.push(self.project.clone());
        if self.studio_undo_stack.len() > 50 {
            self.studio_undo_stack.remove(0);
        }
        self.studio_redo_stack.clear();
    }

    /// Returns human-readable accelerator shortcut text for an action.
    pub fn accelerator(&self, action: ActionId) -> String {
        self.project
            .settings
            .hotkeys
            .registry
            .accelerator_text(action)
    }

    /// Undoes the last Live Studio modification.
    pub fn studio_undo(&mut self) {
        if let Some(prev) = self.studio_undo_stack.pop() {
            self.studio_redo_stack.push(self.project.clone());
            self.project = prev;
            if let Some(sc_id) = self.project.preview_scene_id.clone() {
                self.sync_scene_sources_order_to_compositor(&sc_id);
                for src in self.project.preview_sources() {
                    self.sync_source_transform_to_compositor(
                        &src.id,
                        src.x,
                        src.y,
                        src.width,
                        src.height,
                        src.rotation,
                    );
                }
            }
        }
    }

    /// Redoes the last undone Live Studio modification.
    pub fn studio_redo(&mut self) {
        if let Some(next) = self.studio_redo_stack.pop() {
            self.studio_undo_stack.push(self.project.clone());
            self.project = next;
            if let Some(sc_id) = self.project.preview_scene_id.clone() {
                self.sync_scene_sources_order_to_compositor(&sc_id);
                for src in self.project.preview_sources() {
                    self.sync_source_transform_to_compositor(
                        &src.id,
                        src.x,
                        src.y,
                        src.width,
                        src.height,
                        src.rotation,
                    );
                }
            }
        }
    }

    /// Deletes currently selected source or overlay layer.
    pub fn delete_selected_source(&mut self) {
        if let Some(ref sel_id) = self.selected_source_id.clone() {
            let is_locked = self
                .project
                .preview_sources()
                .iter()
                .find(|s| &s.id == sel_id)
                .map(|s| s.locked)
                .unwrap_or(false);
            if is_locked {
                self.source_warning_message =
                    Some(("Unlock to delete".to_string(), Instant::now()));
                return;
            }
            self.studio_snapshot_for_undo();
            self.remove_source_by_id(sel_id);
            self.selected_source_id = None;
        }
    }

    /// Duplicates currently selected source in preview scene.
    pub fn duplicate_selected_source(&mut self) {
        let Some(ref sc_id) = self.project.preview_scene_id.clone() else {
            return;
        };
        let sel_id_opt = self.selected_source_id.clone();
        if let Some(ref sel_id) = sel_id_opt {
            self.studio_snapshot_for_undo();
            if let Some(sources) = self.project.preview_working_sources_mut() {
                if let Some(idx) = sources.iter().position(|s| &s.id == sel_id) {
                    let mut dup = sources[idx].clone();
                    let new_id = format!("{}_copy_{}", dup.id, uuid::Uuid::new_v4().simple());
                    dup.id = new_id.clone();
                    dup.name = format!("{} (Copy)", dup.name);
                    dup.x += 20.0;
                    dup.y += 20.0;
                    sources.insert(idx + 1, dup);
                    self.selected_source_id = Some(new_id);
                    self.sync_scene_sources_order_to_compositor(sc_id);
                }
            }
        }
    }

    /// Nudges selected source or layer by (dx, dy).
    pub fn nudge_selection(&mut self, dx: f32, dy: f32) {
        if self.app_mode == AppMode::EditOverlay {
            if let Some(layer_id) = self.overlay_editor.selected_layer_id {
                if let Some(layer) = self
                    .overlay_editor
                    .composition
                    .layers
                    .iter_mut()
                    .find(|l| l.id == layer_id)
                {
                    let cur = *layer.transform.position.fallback_value();
                    layer.transform.position =
                        zeno_core::Property::constant(cur + zeno_core::Vec3::new(dx, dy, 0.0));
                }
            }
        } else {
            let Some(sources) = self.project.preview_working_sources_mut() else {
                return;
            };
            if let Some(ref sel_id) = self.selected_source_id {
                if let Some(src) = sources.iter_mut().find(|s| &s.id == sel_id) {
                    src.x += dx;
                    src.y += dy;
                    let (x, y, w, h, rot) = (src.x, src.y, src.width, src.height, src.rotation);
                    let sid = src.id.clone();
                    self.sync_source_transform_to_compositor(&sid, x, y, w, h, rot);
                }
            }
        }
    }

    /// Toggles mute state of a source by ID.
    pub fn toggle_source_mute(&mut self, source_id: &str) {
        for scene in &mut self.project.scenes {
            for src in &mut scene.sources {
                if src.id == source_id {
                    src.muted = !src.muted;
                    let sid = string_to_source_id(&src.id);
                    if let Some(s) = self.source_registry.write().get_mut(sid) {
                        let mut props = s.properties();
                        match &mut props {
                            SourceProperties::AudioInput { muted, .. } => *muted = src.muted,
                            SourceProperties::AudioOutput { muted, .. } => *muted = src.muted,
                            _ => {}
                        }
                        if let (Some(dev), Some(q)) = (&self.wgpu_device, &self.wgpu_queue) {
                            let _ = s.update_properties(&props, dev, q);
                        }
                    }
                }
            }
        }
    }

    /// Sets explicit mute state of a source by ID.
    pub fn set_source_muted(&mut self, source_id: &str, muted: bool) {
        for scene in &mut self.project.scenes {
            for src in &mut scene.sources {
                if src.id == source_id {
                    src.muted = muted;
                    let sid = string_to_source_id(&src.id);
                    if let Some(s) = self.source_registry.write().get_mut(sid) {
                        let mut props = s.properties();
                        match &mut props {
                            SourceProperties::AudioInput { muted, .. } => *muted = src.muted,
                            SourceProperties::AudioOutput { muted, .. } => *muted = src.muted,
                            _ => {}
                        }
                        if let (Some(dev), Some(q)) = (&self.wgpu_device, &self.wgpu_queue) {
                            let _ = s.update_properties(&props, dev, q);
                        }
                    }
                }
            }
        }
    }

    /// Toggles visibility of a source by ID.
    pub fn toggle_source_visibility_by_id(&mut self, source_id: &str) {
        self.toggle_source_visibility(source_id);
    }

    /// Dispatches an action ID from the master shortcuts table.
    pub fn dispatch_action(&mut self, id: crate::hotkey_registry::ActionId) {
        use crate::hotkey_registry::ActionId;
        match id {
            ActionId::StartStreaming => {
                if !self.is_streaming {
                    self.toggle_streaming();
                }
            }
            ActionId::StopStreaming => {
                if self.is_streaming {
                    self.toggle_streaming();
                }
            }
            ActionId::StartRecording => {
                if !self.is_recording {
                    self.toggle_recording();
                }
            }
            ActionId::StopRecording => {
                if self.is_recording {
                    self.toggle_recording();
                }
            }
            ActionId::PauseRecording => {}
            ActionId::ToggleReplayBuffer | ActionId::SaveReplayBuffer => {
                info!("Replay buffer action triggered");
            }
            ActionId::TogglePreview => {}
            ActionId::Cut => {
                self.execute_cut();
            }
            ActionId::AutoTransition => {
                self.execute_auto();
            }
            ActionId::NextScene => {
                self.next_scene();
            }
            ActionId::PreviousScene => {
                self.previous_scene();
            }
            ActionId::ToggleStudioMode => {}
            ActionId::Undo => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor.undo();
                } else {
                    self.studio_undo();
                }
            }
            ActionId::Redo => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor.redo();
                } else {
                    self.studio_redo();
                }
            }
            ActionId::DeleteSelected => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor.delete_selected_layer();
                } else {
                    self.delete_selected_source();
                }
            }
            ActionId::DuplicateSelected => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor.copy_selected();
                    self.overlay_editor.paste();
                } else {
                    self.duplicate_selected_source();
                }
            }
            ActionId::Copy => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor.copy_selected();
                }
            }
            ActionId::Paste | ActionId::PasteInPlace => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor.paste();
                }
            }
            ActionId::SelectAll => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor.selected_layer_id =
                        self.overlay_editor.composition.layers.first().map(|l| l.id);
                } else {
                    let sources = self.project.preview_sources();
                    self.selected_source_id = sources.first().map(|s| s.id.clone());
                }
            }
            ActionId::DeselectAll => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor.selected_layer_id = None;
                } else {
                    self.selected_source_id = None;
                }
            }
            ActionId::BringToFront => {
                if self.app_mode == AppMode::EditOverlay {
                    if let Some(cur) = self.overlay_editor.selected_layer_id {
                        let total = self.overlay_editor.composition.layers.len();
                        if let Some(i) = self
                            .overlay_editor
                            .composition
                            .layers
                            .iter()
                            .position(|l| l.id == cur)
                        {
                            if i + 1 < total {
                                self.overlay_editor.reorder_layer(i, total - 1);
                            }
                        }
                    }
                } else {
                    let mut changed = false;
                    let sc_id_opt = self.project.preview_scene_id.clone();
                    if let Some(sources) = self.project.preview_working_sources_mut() {
                        if let Some(ref sel) = self.selected_source_id {
                            if let Some(i) = sources.iter().position(|s| &s.id == sel) {
                                if i + 1 < sources.len() {
                                    let s = sources.remove(i);
                                    sources.push(s);
                                    changed = true;
                                }
                            }
                        }
                    }
                    if changed {
                        self.studio_snapshot_for_undo();
                        if let Some(sc_id) = sc_id_opt {
                            self.sync_scene_sources_order_to_compositor(&sc_id);
                        }
                    }
                }
            }
            ActionId::BringForward => {
                if self.app_mode == AppMode::EditOverlay {
                    if let Some(cur) = self.overlay_editor.selected_layer_id {
                        if let Some(i) = self
                            .overlay_editor
                            .composition
                            .layers
                            .iter()
                            .position(|l| l.id == cur)
                        {
                            if i + 1 < self.overlay_editor.composition.layers.len() {
                                self.overlay_editor.reorder_layer(i, i + 1);
                            }
                        }
                    }
                } else {
                    let mut changed = false;
                    let sc_id_opt = self.project.preview_scene_id.clone();
                    if let Some(sources) = self.project.preview_working_sources_mut() {
                        if let Some(ref sel) = self.selected_source_id {
                            if let Some(i) = sources.iter().position(|s| &s.id == sel) {
                                if i + 1 < sources.len() {
                                    sources.swap(i, i + 1);
                                    changed = true;
                                }
                            }
                        }
                    }
                    if changed {
                        self.studio_snapshot_for_undo();
                        if let Some(sc_id) = sc_id_opt {
                            self.sync_scene_sources_order_to_compositor(&sc_id);
                        }
                    }
                }
            }
            ActionId::SendBackward => {
                if self.app_mode == AppMode::EditOverlay {
                    if let Some(cur) = self.overlay_editor.selected_layer_id {
                        if let Some(i) = self
                            .overlay_editor
                            .composition
                            .layers
                            .iter()
                            .position(|l| l.id == cur)
                        {
                            if i > 0 {
                                self.overlay_editor.reorder_layer(i, i - 1);
                            }
                        }
                    }
                } else {
                    let mut changed = false;
                    let sc_id_opt = self.project.preview_scene_id.clone();
                    if let Some(sources) = self.project.preview_working_sources_mut() {
                        if let Some(ref sel) = self.selected_source_id {
                            if let Some(i) = sources.iter().position(|s| &s.id == sel) {
                                if i > 0 {
                                    sources.swap(i, i - 1);
                                    changed = true;
                                }
                            }
                        }
                    }
                    if changed {
                        self.studio_snapshot_for_undo();
                        if let Some(sc_id) = sc_id_opt {
                            self.sync_scene_sources_order_to_compositor(&sc_id);
                        }
                    }
                }
            }
            ActionId::SendToBack => {
                if self.app_mode == AppMode::EditOverlay {
                    if let Some(cur) = self.overlay_editor.selected_layer_id {
                        if let Some(i) = self
                            .overlay_editor
                            .composition
                            .layers
                            .iter()
                            .position(|l| l.id == cur)
                        {
                            if i > 0 {
                                self.overlay_editor.reorder_layer(i, 0);
                            }
                        }
                    }
                } else {
                    let mut changed = false;
                    let sc_id_opt = self.project.preview_scene_id.clone();
                    if let Some(sources) = self.project.preview_working_sources_mut() {
                        if let Some(ref sel) = self.selected_source_id {
                            if let Some(i) = sources.iter().position(|s| &s.id == sel) {
                                if i > 0 {
                                    let s = sources.remove(i);
                                    sources.insert(0, s);
                                    changed = true;
                                }
                            }
                        }
                    }
                    if changed {
                        self.studio_snapshot_for_undo();
                        if let Some(sc_id) = sc_id_opt {
                            self.sync_scene_sources_order_to_compositor(&sc_id);
                        }
                    }
                }
            }
            ActionId::ToggleSourceVisibility => {
                if let Some(ref sel_id) = self.selected_source_id.clone() {
                    self.studio_snapshot_for_undo();
                    self.toggle_source_visibility(sel_id);
                }
            }
            ActionId::ToggleSourceLock => {
                if let Some(ref sel_id) = self.selected_source_id.clone() {
                    self.studio_snapshot_for_undo();
                    self.toggle_source_lock(sel_id);
                }
            }
            ActionId::ToggleSourceMute => {
                if let Some(ref sel_id) = self.selected_source_id.clone() {
                    self.studio_snapshot_for_undo();
                    self.toggle_source_mute(sel_id);
                }
            }
            ActionId::OpenSourceProperties => {
                if let Some(ref sel_id) = self.selected_source_id.clone() {
                    if let Some(src) = self
                        .project
                        .preview_sources()
                        .into_iter()
                        .find(|s| &s.id == sel_id)
                    {
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
            ActionId::RenameSelected => {
                if let Some(ref sel_sc) = self.selected_scene_id.clone() {
                    if let Some(sc) = self.project.scenes.iter().find(|s| &s.id == sel_sc) {
                        self.rename_scene_id = Some(sel_sc.clone());
                        self.rename_scene_buffer = sc.name.clone();
                    }
                }
            }
            ActionId::AddTextLayer => {
                if self.app_mode == AppMode::EditOverlay {
                    let count = self.overlay_editor.composition.layers.len() + 1;
                    self.overlay_editor
                        .add_text_layer(&format!("Text Layer {}", count), "New Text");
                }
            }
            ActionId::AddShapeLayer => {
                if self.app_mode == AppMode::EditOverlay {
                    let count = self.overlay_editor.composition.layers.len() + 1;
                    self.overlay_editor.add_shape_layer(
                        &format!("Shape Layer {}", count),
                        zeno_core::Vec2::new(400.0, 80.0),
                        zeno_core::Color::rgba(0.1, 0.1, 0.1, 0.9),
                    );
                }
            }
            ActionId::AddCameraLayer => {
                if self.app_mode == AppMode::EditOverlay {
                    let count = self.overlay_editor.composition.layers.len() + 1;
                    self.overlay_editor
                        .add_camera_layer(&format!("Camera {}", count));
                }
            }
            ActionId::AddParticlesLayer => {
                if self.app_mode == AppMode::EditOverlay {
                    let count = self.overlay_editor.composition.layers.len() + 1;
                    self.overlay_editor.add_particle_layer(
                        &format!("Particles {}", count),
                        zeno_engine::ParticleEmitter::sparks(),
                    );
                }
            }
            ActionId::DeleteLayer => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor.delete_selected_layer();
                }
            }
            ActionId::GroupLayers | ActionId::UngroupLayers => {}
            ActionId::AddKeyframe => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor
                        .add_opacity_keyframe(1.0, zeno_core::Easing::Linear);
                }
            }
            ActionId::DeleteKeyframe => {
                if self.app_mode == AppMode::EditOverlay {
                    if let Some(sel) = self.overlay_editor.selected_keyframe.clone() {
                        self.overlay_editor
                            .delete_keyframe(sel.layer_id, sel.property, sel.time);
                    }
                }
            }
            ActionId::PlayPauseTimeline => {
                if self.app_mode == AppMode::EditOverlay {
                    if self.overlay_editor.clock.is_playing() {
                        self.overlay_editor.clock.pause();
                    } else {
                        self.overlay_editor.clock.play();
                    }
                }
            }
            ActionId::StopTimeline => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor.clock.stop();
                }
            }
            ActionId::GoToStart => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor.clock.seek(0.0);
                }
            }
            ActionId::GoToEnd => {
                if self.app_mode == AppMode::EditOverlay {
                    let dur = self.overlay_editor.composition.duration_seconds;
                    self.overlay_editor.clock.seek(dur);
                }
            }
            ActionId::StepFrameBack => {
                if self.app_mode == AppMode::EditOverlay {
                    let cur = self.overlay_editor.clock.time();
                    let fps = self.overlay_editor.composition.frames_per_second as f64;
                    self.overlay_editor.clock.seek((cur - 1.0 / fps).max(0.0));
                }
            }
            ActionId::StepFrameForward => {
                if self.app_mode == AppMode::EditOverlay {
                    let cur = self.overlay_editor.clock.time();
                    let dur = self.overlay_editor.composition.duration_seconds;
                    let fps = self.overlay_editor.composition.frames_per_second as f64;
                    self.overlay_editor.clock.seek((cur + 1.0 / fps).min(dur));
                }
            }
            ActionId::SetInMarker => {
                if self.app_mode == AppMode::EditOverlay {
                    let t = self.overlay_editor.clock.time();
                    self.overlay_editor.markers.retain(|m| m.name != "In");
                    self.overlay_editor
                        .markers
                        .push(crate::editor::TimelineMarker {
                            time_seconds: t,
                            name: "In".into(),
                            comment: None,
                        });
                }
            }
            ActionId::SetOutMarker => {
                if self.app_mode == AppMode::EditOverlay {
                    let t = self.overlay_editor.clock.time();
                    self.overlay_editor.markers.retain(|m| m.name != "Out");
                    self.overlay_editor
                        .markers
                        .push(crate::editor::TimelineMarker {
                            time_seconds: t,
                            name: "Out".into(),
                            comment: None,
                        });
                }
            }
            ActionId::AddMarker => {
                if self.app_mode == AppMode::EditOverlay {
                    let t = self.overlay_editor.clock.time();
                    let num = self.overlay_editor.markers.len() + 1;
                    self.overlay_editor
                        .markers
                        .push(crate::editor::TimelineMarker {
                            time_seconds: t,
                            name: format!("Marker {}", num),
                            comment: None,
                        });
                }
            }
            ActionId::ZoomTimelineIn | ActionId::ZoomTimelineOut | ActionId::FitTimeline => {}
            ActionId::ZoomCanvasIn => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor.zoom = (self.overlay_editor.zoom * 1.25).min(8.0);
                }
            }
            ActionId::ZoomCanvasOut => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor.zoom = (self.overlay_editor.zoom / 1.25).max(0.1);
                }
            }
            ActionId::ResetCanvasZoom => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor.zoom = 1.0;
                    self.overlay_editor.pan = zeno_core::Vec2::ZERO;
                }
            }
            ActionId::FitCanvas => {
                if self.app_mode == AppMode::EditOverlay {
                    self.overlay_editor.zoom = 1.0;
                }
            }
            ActionId::ToggleCurveEditor => {
                if self.app_mode == AppMode::EditOverlay {
                    self.timeline_mode = match self.timeline_mode {
                        TimelineMode::DopeSheet => TimelineMode::CurveEditor,
                        TimelineMode::CurveEditor => TimelineMode::DopeSheet,
                    };
                }
            }
            ActionId::ToolSelect | ActionId::ToolPan => {}
            ActionId::ModeLiveStudio => {
                self.app_mode = AppMode::LiveStudio;
            }
            ActionId::ModeEditOverlay => {
                self.app_mode = AppMode::EditOverlay;
            }
            ActionId::ModeDataControl => {
                self.app_mode = AppMode::DataControl;
            }
            ActionId::OpenSettings => {
                self.show_settings = true;
            }
            ActionId::NewProject => {
                self.reset_to_new_project();
            }
            ActionId::OpenProject => {
                self.show_open_dialog = true;
                self.file_dialog_path = "project.arxproj".to_string();
            }
            ActionId::SaveProject => {
                if let Some(ref path) = self.project_path {
                    let _ = self.project.save_to_file(path);
                } else {
                    self.show_save_as_dialog = true;
                    self.file_dialog_path = "project.arxproj".to_string();
                }
            }
            ActionId::SaveProjectAs => {
                self.show_save_as_dialog = true;
                self.file_dialog_path = self
                    .project_path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|| "project.arxproj".to_string());
            }
            ActionId::ToggleFullscreen => {}
            ActionId::DialogCancel => {
                if self.show_settings {
                    self.show_settings = false;
                } else if self.show_open_dialog {
                    self.show_open_dialog = false;
                } else if self.show_save_as_dialog {
                    self.show_save_as_dialog = false;
                } else if self.source_dialog.is_open() {
                    self.source_dialog.close();
                }
            }
            ActionId::DialogConfirm => {}
        }
    }

    /// Advances real-time engine processing: captures, GPU compositor rendering, and periodic autosaves.
    pub fn tick_engine(&mut self, ctx: &Context) {
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
                if !comp.is_transition_active() {
                    if let Some(ref act) = self.project.active_scene_id {
                        comp.set_program(string_to_scene_id(act));
                    }
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
                    let was_transitioning = comp.is_transition_active();
                    let _ = comp.render_frame(&frame_views, 1.0 / 60.0);
                    if was_transitioning && !comp.is_transition_active() {
                        self.project.commit_preview_working_sources();
                        self.project.active_scene_id = self.project.preview_scene_id.clone();
                    }
                }
            }
            ctx.request_repaint();
        }

        // 30-second periodic autosave check
        let _ = self.autosaver.check_and_save(&mut self.project);
    }

    /// Renders the complete broadcast UI layout, panels, and modal dialogs.
    pub fn draw_ui(&mut self, ctx: &Context) {
        // 1. Drain global hotkey events from OS
        while let Ok(event) = global_hotkey::GlobalHotKeyEvent::receiver().try_recv() {
            if event.state == global_hotkey::HotKeyState::Pressed {
                if let Some(target) = self.global_hotkey_map.get(&event.id).cloned() {
                    self.dispatch_target_action(&target);
                }
            }
        }

        // 2. Interactive Hotkey Rebinding capture
        if let Some(target) = self.rebinding_target.clone() {
            let captured = ctx.input(|i| {
                if i.key_pressed(egui::Key::Escape) {
                    Some(None)
                } else {
                    KeyCombo::detect_pressed(i).map(Some)
                }
            });

            if let Some(res) = captured {
                if let Some(combo) = res {
                    self.project
                        .settings
                        .hotkeys
                        .registry
                        .set_target_combo(&target, &combo.format_text());
                    self.sync_global_hotkeys();
                }
                self.rebinding_target = None;
            }
        }

        // 3. Normal Focus-Aware Hotkey Handling (only when not interactive rebinding)
        if self.rebinding_target.is_none() {
            let text_editing_active = self.rename_scene_id.is_some()
                || ((self.show_settings
                    || self.show_open_dialog
                    || self.show_save_as_dialog
                    || self.source_dialog.is_open())
                    && ctx.wants_keyboard_input());

            #[cfg(debug_assertions)]
            if !text_editing_active {
                let ctrl_r = ctx.input(|i| {
                    (i.modifiers.ctrl || i.modifiers.command) && i.key_pressed(egui::Key::R)
                });
                if ctrl_r {
                    info!("[dev-watch] Ctrl+R triggered: closing window for rebuild and relaunch");
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            }

            let mut triggered_action: Option<ActionId> = None;
            let mut triggered_target: Option<RebindTarget> = None;

            ctx.input(|i| {
                // Check all master table action definitions
                for meta in crate::hotkey_registry::MASTER_ACTION_DEFINITIONS {
                    if self
                        .project
                        .settings
                        .hotkeys
                        .registry
                        .action_triggered(meta.id, i)
                    {
                        // Guard single-key typing shortcuts when actively editing text fields
                        if text_editing_active {
                            if let Some(combo) =
                                self.project.settings.hotkeys.registry.get_combo(meta.id)
                            {
                                if !combo.ctrl
                                    && !combo.alt
                                    && meta.id != ActionId::DialogCancel
                                    && meta.id != ActionId::DialogConfirm
                                {
                                    continue;
                                }
                            }
                        }
                        triggered_action = Some(meta.id);
                        break;
                    }
                }
                // Check per-scene hotkeys
                if triggered_action.is_none() {
                    for (sc_id, sc_hk) in &self.project.settings.hotkeys.registry.scene_hotkeys {
                        if let Some(ref s) = sc_hk.switch_to {
                            if let Some(c) = KeyCombo::parse(s) {
                                if c.matches(i) {
                                    triggered_target =
                                        Some(RebindTarget::SceneSwitch(sc_id.clone()));
                                    break;
                                }
                            }
                        }
                        if let Some(ref s) = sc_hk.switch_back {
                            if let Some(c) = KeyCombo::parse(s) {
                                if c.matches(i) {
                                    triggered_target =
                                        Some(RebindTarget::SceneSwitchBack(sc_id.clone()));
                                    break;
                                }
                            }
                        }
                    }
                }
                // Check per-source hotkeys (toggle mute, show/hide)
                if triggered_action.is_none() && triggered_target.is_none() {
                    for (src_id, src_hk) in &self.project.settings.hotkeys.registry.source_hotkeys {
                        if let Some(ref s) = src_hk.mute {
                            if let Some(c) = KeyCombo::parse(s) {
                                if c.matches(i) {
                                    triggered_target =
                                        Some(RebindTarget::SourceMute(src_id.clone()));
                                    break;
                                }
                            }
                        }
                        if let Some(ref s) = src_hk.show_hide {
                            if let Some(c) = KeyCombo::parse(s) {
                                if c.matches(i) {
                                    triggered_target =
                                        Some(RebindTarget::SourceShowHide(src_id.clone()));
                                    break;
                                }
                            }
                        }
                    }
                }
            });

            if let Some(act) = triggered_action {
                self.dispatch_action(act);
            } else if let Some(target) = triggered_target {
                self.dispatch_target_action(&target);
            }

            // Check push-to-mute and push-to-talk state changes
            let source_hotkeys = self
                .project
                .settings
                .hotkeys
                .registry
                .source_hotkeys
                .clone();
            ctx.input(|i| {
                for (src_id, src_hk) in &source_hotkeys {
                    if let Some(ref s) = src_hk.push_to_mute {
                        if let Some(c) = KeyCombo::parse(s) {
                            let down = c.is_down(i);
                            let was_down = self
                                .push_to_mute_active
                                .get(src_id)
                                .copied()
                                .unwrap_or(false);
                            if down != was_down {
                                self.push_to_mute_active.insert(src_id.clone(), down);
                                self.set_source_muted(src_id, down);
                            }
                        }
                    }
                    if let Some(ref s) = src_hk.push_to_talk {
                        if let Some(c) = KeyCombo::parse(s) {
                            let down = c.is_down(i);
                            let was_down = self
                                .push_to_talk_active
                                .get(src_id)
                                .copied()
                                .unwrap_or(false);
                            if down != was_down {
                                self.push_to_talk_active.insert(src_id.clone(), down);
                                self.set_source_muted(src_id, !down);
                            }
                        }
                    }
                }
            });
        }

        // Top Menu Bar
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui
                        .add(
                            egui::Button::new("New Project")
                                .shortcut_text(self.accelerator(ActionId::NewProject)),
                        )
                        .clicked()
                    {
                        self.reset_to_new_project();
                        ui.close_menu();
                    }
                    if ui
                        .add(
                            egui::Button::new("Open Project...")
                                .shortcut_text(self.accelerator(ActionId::OpenProject)),
                        )
                        .clicked()
                    {
                        self.show_open_dialog = true;
                        self.file_dialog_path = "project.arxproj".to_string();
                        ui.close_menu();
                    }
                    if ui
                        .add(
                            egui::Button::new("Save Project")
                                .shortcut_text(self.accelerator(ActionId::SaveProject)),
                        )
                        .clicked()
                    {
                        if let Some(ref path) = self.project_path {
                            let _ = self.project.save_to_file(path);
                        } else {
                            self.show_save_as_dialog = true;
                            self.file_dialog_path = "project.arxproj".to_string();
                        }
                        ui.close_menu();
                    }
                    if ui
                        .add(
                            egui::Button::new("Save Project As...")
                                .shortcut_text(self.accelerator(ActionId::SaveProjectAs)),
                        )
                        .clicked()
                    {
                        self.show_save_as_dialog = true;
                        self.file_dialog_path = self
                            .project_path
                            .as_ref()
                            .map(|p| p.to_string_lossy().to_string())
                            .unwrap_or_else(|| "project.arxproj".to_string());
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .add(
                            egui::Button::new("Settings...")
                                .shortcut_text(self.accelerator(ActionId::OpenSettings)),
                        )
                        .clicked()
                    {
                        self.show_settings = true;
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Exit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("Edit", |ui| {
                    if ui
                        .add(
                            egui::Button::new("Undo")
                                .shortcut_text(self.accelerator(ActionId::Undo)),
                        )
                        .clicked()
                    {
                        self.dispatch_action(ActionId::Undo);
                        ui.close_menu();
                    }
                    if ui
                        .add(
                            egui::Button::new("Redo")
                                .shortcut_text(self.accelerator(ActionId::Redo)),
                        )
                        .clicked()
                    {
                        self.dispatch_action(ActionId::Redo);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .add(
                            egui::Button::new("Duplicate")
                                .shortcut_text(self.accelerator(ActionId::DuplicateSelected)),
                        )
                        .clicked()
                    {
                        self.dispatch_action(ActionId::DuplicateSelected);
                        ui.close_menu();
                    }
                    if ui
                        .add(
                            egui::Button::new("Delete")
                                .shortcut_text(self.accelerator(ActionId::DeleteSelected)),
                        )
                        .clicked()
                    {
                        self.dispatch_action(ActionId::DeleteSelected);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .add(
                            egui::Button::new("Select All")
                                .shortcut_text(self.accelerator(ActionId::SelectAll)),
                        )
                        .clicked()
                    {
                        self.dispatch_action(ActionId::SelectAll);
                        ui.close_menu();
                    }
                    if ui
                        .add(
                            egui::Button::new("Deselect All")
                                .shortcut_text(self.accelerator(ActionId::DeselectAll)),
                        )
                        .clicked()
                    {
                        self.dispatch_action(ActionId::DeselectAll);
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui
                        .add(
                            egui::Button::new("Bring to Front")
                                .shortcut_text(self.accelerator(ActionId::BringToFront)),
                        )
                        .clicked()
                    {
                        self.dispatch_action(ActionId::BringToFront);
                        ui.close_menu();
                    }
                    if ui
                        .add(
                            egui::Button::new("Bring Forward")
                                .shortcut_text(self.accelerator(ActionId::BringForward)),
                        )
                        .clicked()
                    {
                        self.dispatch_action(ActionId::BringForward);
                        ui.close_menu();
                    }
                    if ui
                        .add(
                            egui::Button::new("Send Backward")
                                .shortcut_text(self.accelerator(ActionId::SendBackward)),
                        )
                        .clicked()
                    {
                        self.dispatch_action(ActionId::SendBackward);
                        ui.close_menu();
                    }
                    if ui
                        .add(
                            egui::Button::new("Send to Back")
                                .shortcut_text(self.accelerator(ActionId::SendToBack)),
                        )
                        .clicked()
                    {
                        self.dispatch_action(ActionId::SendToBack);
                        ui.close_menu();
                    }
                });

                ui.menu_button("Mode", |ui| {
                    let studio_acc = self.accelerator(ActionId::ModeLiveStudio);
                    let overlay_acc = self.accelerator(ActionId::ModeEditOverlay);
                    let data_acc = self.accelerator(ActionId::ModeDataControl);

                    let studio_text = if studio_acc.is_empty() {
                        "Live Studio".to_string()
                    } else {
                        format!("Live Studio ({})", studio_acc)
                    };
                    let overlay_text = if overlay_acc.is_empty() {
                        "Edit Overlay".to_string()
                    } else {
                        format!("Edit Overlay ({})", overlay_acc)
                    };
                    let data_text = if data_acc.is_empty() {
                        "Data Control".to_string()
                    } else {
                        format!("Data Control ({})", data_acc)
                    };

                    if ui
                        .selectable_label(self.app_mode == AppMode::LiveStudio, studio_text)
                        .clicked()
                    {
                        self.app_mode = AppMode::LiveStudio;
                        ui.close_menu();
                    }
                    if ui
                        .selectable_label(self.app_mode == AppMode::EditOverlay, overlay_text)
                        .clicked()
                    {
                        self.app_mode = AppMode::EditOverlay;
                        ui.close_menu();
                    }
                    if ui
                        .selectable_label(self.app_mode == AppMode::DataControl, data_text)
                        .clicked()
                    {
                        self.app_mode = AppMode::DataControl;
                        ui.close_menu();
                    }
                });

                ui.menu_button("View", |ui| {
                    let mut sm = self.studio_mode;
                    if ui.checkbox(&mut sm, "Studio Mode").clicked() {
                        self.toggle_studio_mode();
                        ui.close_menu();
                    }
                    if ui.button("Reset UI Layout").clicked() {
                        self.dock_state = create_default_live_layout();
                        ui.close_menu();
                    }
                });

                ui.menu_button("Tools", |ui| {
                    if ui.button("Auto-Configuration Wizard...").clicked() {
                        self.wizard.is_open = true;
                        ui.close_menu();
                    }
                    if ui
                        .add(
                            egui::Button::new("Settings...")
                                .shortcut_text(self.accelerator(ActionId::OpenSettings)),
                        )
                        .clicked()
                    {
                        self.show_settings = true;
                        ui.close_menu();
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.is_streaming {
                        ui.colored_label(RED_LIVE, "LIVE");
                    }
                    if self.is_recording {
                        ui.colored_label(RED_LIVE, "REC");
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
                #[cfg(debug_assertions)]
                if let Some(reload_time) = crate::get_reload_indicator() {
                    ui.separator();
                    ui.colored_label(crate::theme::ACCENT, format!("UI reloaded {}", reload_time));
                }
                if let Some((msg, time)) = &self.source_warning_message {
                    if time.elapsed().as_secs_f32() < 3.0 {
                        ui.separator();
                        ui.colored_label(crate::theme::AMBER_WARN, msg);
                    }
                }
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
        let mut trigger_open_add_source_with_path: Option<String> = None;
        let mut trigger_open_edit_source: Option<String> = None;
        let mut trigger_remove_source: Option<String> = None;
        let mut trigger_toggle_source_visibility: Option<String> = None;
        let mut trigger_toggle_source_lock: Option<String> = None;
        let mut trigger_transform_source: Option<(String, f32, f32, f32, f32, f32)> = None;
        let mut trigger_add_scene = false;
        let mut trigger_remove_scene: Option<String> = None;
        let mut trigger_rename_scene: Option<(String, String)> = None;
        let mut trigger_move_scene_up: Option<String> = None;
        let mut trigger_move_scene_down: Option<String> = None;
        let mut trigger_sync_source_order: Option<String> = None;
        let mut trigger_snapshot_undo = false;
        let mut trigger_audio_change = false;
        let mut trigger_toggle_studio_mode = false;
        let mut trigger_switch_scene_direct: Option<String> = None;
        let mut trigger_paste_reference = false;
        let mut trigger_paste_duplicate = false;
        let mut trigger_save_screenshot: Option<(Option<String>, bool)> = None;
        let mut trigger_resize_output_to_source: Option<String> = None;
        let mut trigger_open_filters: Option<String> = None;
        let mut trigger_copy_filters: Option<String> = None;
        let mut trigger_paste_filters: Option<String> = None;

        let stats = self.output_manager.stats();
        let fps = if stats.encode_time_ms > 0.0 {
            (1000.0 / stats.encode_time_ms).min(60.0)
        } else {
            60.0
        };
        let dropped_percent = stats.dropped_percent;
        let bitrate_kbps = if self.is_streaming && stats.current_bitrate_kbps > 0.0 {
            stats.current_bitrate_kbps as u32
        } else {
            self.project.settings.output.video_bitrate_kbps
        };
        let stream_time = self.stream_duration_string();

        // Central Panel based on selected AppMode
        CentralPanel::default().show(ctx, |ui| match self.app_mode {
            AppMode::LiveStudio => {
                let mut viewer = StudioTabViewer {
                    project: &mut self.project,
                    selected_scene_id: &mut self.selected_scene_id,
                    selected_source_id: &mut self.selected_source_id,
                    rebinding_target: &mut self.rebinding_target,
                    is_streaming: self.is_streaming,
                    is_recording: self.is_recording,
                    fps,
                    dropped_percent,
                    bitrate_kbps,
                    stream_time,
                    active_transition_kind: &mut self.active_transition_kind,
                    transition_duration_ms: &mut self.transition_duration_ms,
                    t_bar_progress: &mut self.t_bar_progress,
                    master_volume: &mut self.master_volume,
                    master_muted: &mut self.master_muted,
                    rename_scene_id: &mut self.rename_scene_id,
                    rename_scene_buffer: &mut self.rename_scene_buffer,
                    trigger_rename_scene: &mut trigger_rename_scene,
                    trigger_move_scene_up: &mut trigger_move_scene_up,
                    trigger_move_scene_down: &mut trigger_move_scene_down,
                    trigger_audio_change: &mut trigger_audio_change,
                    trigger_cut: &mut trigger_cut,
                    trigger_auto: &mut trigger_auto,
                    trigger_toggle_stream: &mut trigger_toggle_stream,
                    trigger_toggle_record: &mut trigger_toggle_record,
                    preview_texture_id: self.preview_texture_id,
                    program_texture_id: self.program_texture_id,
                    trigger_open_add_source: &mut trigger_open_add_source,
                    trigger_open_add_source_with_path: &mut trigger_open_add_source_with_path,
                    trigger_open_edit_source: &mut trigger_open_edit_source,
                    trigger_remove_source: &mut trigger_remove_source,
                    trigger_toggle_source_visibility: &mut trigger_toggle_source_visibility,
                    trigger_toggle_source_lock: &mut trigger_toggle_source_lock,
                    trigger_add_scene: &mut trigger_add_scene,
                    trigger_remove_scene: &mut trigger_remove_scene,
                    live_gizmo_state: &mut self.live_gizmo_state,
                    trigger_transform_source: &mut trigger_transform_source,
                    trigger_sync_source_order: &mut trigger_sync_source_order,
                    trigger_snapshot_undo: &mut trigger_snapshot_undo,
                    source_warning_message: &mut self.source_warning_message,
                    studio_mode: self.studio_mode,
                    trigger_toggle_studio_mode: &mut trigger_toggle_studio_mode,
                    trigger_switch_scene_direct: &mut trigger_switch_scene_direct,
                    clipboard_source: &mut self.clipboard_source,
                    preview_locked: &mut self.preview_locked,
                    preview_scaling_mode: &mut self.preview_scaling_mode,
                    trigger_paste_reference: &mut trigger_paste_reference,
                    trigger_paste_duplicate: &mut trigger_paste_duplicate,
                    trigger_save_screenshot: &mut trigger_save_screenshot,
                    trigger_resize_output_to_source: &mut trigger_resize_output_to_source,
                    trigger_open_filters: &mut trigger_open_filters,
                    trigger_copy_filters: &mut trigger_copy_filters,
                    trigger_paste_filters: &mut trigger_paste_filters,
                    has_clipboard_filters: self.clipboard_filters.is_some(),
                };

                if ui.available_width() >= 100.0 && ui.available_height() >= 100.0 {
                    let mut dock_style = egui_dock::Style::from_egui(ui.style().as_ref());
                    dock_style.separator.width = 3.0;
                    dock_style.separator.extra_interact_width = 4.0;
                    dock_style.separator.extra = 100.0;
                    dock_style.separator.color_idle = crate::theme::SURFACE_3;
                    dock_style.separator.color_hovered = crate::theme::ACCENT;
                    dock_style.separator.color_dragged = crate::theme::ACCENT;

                    DockArea::new(&mut self.dock_state)
                        .show_close_buttons(false)
                        .show_add_buttons(false)
                        .show_leaf_close_all_buttons(false)
                        .show_leaf_collapse_buttons(false)
                        .style(dock_style)
                        .show_inside(ui, &mut viewer);
                }
            }
            AppMode::EditOverlay => {
                ui.vertical(|ui| {
                    let total_height = ui.available_height();
                    let top_height = (total_height * 0.65).max(200.0);

                    ui.allocate_ui_with_layout(
                        vec2(ui.available_width(), top_height),
                        egui::Layout::left_to_right(egui::Align::Min),
                        |ui| {
                            let left_w = 280.0_f32.min(ui.available_width() * 0.28);
                            ui.allocate_ui_with_layout(
                                vec2(left_w, top_height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    egui::Frame::group(ui.style()).show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            if ui
                                                .selectable_label(
                                                    self.overlay_editor.active_left_tab
                                                        == LeftTab::Layers,
                                                    "Layers",
                                                )
                                                .clicked()
                                            {
                                                self.overlay_editor.active_left_tab =
                                                    LeftTab::Layers;
                                            }
                                            if ui
                                                .selectable_label(
                                                    self.overlay_editor.active_left_tab
                                                        == LeftTab::Assets,
                                                    "Assets",
                                                )
                                                .clicked()
                                            {
                                                self.overlay_editor.active_left_tab =
                                                    LeftTab::Assets;
                                            }
                                        });
                                        ui.separator();
                                        match self.overlay_editor.active_left_tab {
                                            LeftTab::Layers => {
                                                ui_layers_panel(&mut self.overlay_editor, ui);
                                            }
                                            LeftTab::Assets => {
                                                ui_assets_panel(&mut self.overlay_editor, ui);
                                            }
                                        }
                                    });
                                },
                            );
                            ui.separator();

                            let canvas_w = (ui.available_width() - 320.0).max(300.0);
                            ui.allocate_ui_with_layout(
                                vec2(canvas_w, top_height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    egui::Frame::group(ui.style()).show(ui, |ui| {
                                        ui_canvas(&mut self.overlay_editor, ui);
                                    });
                                },
                            );
                            ui.separator();

                            ui.allocate_ui_with_layout(
                                vec2(ui.available_width(), top_height),
                                egui::Layout::top_down(egui::Align::Min),
                                |ui| {
                                    egui::Frame::group(ui.style()).show(ui, |ui| {
                                        ui_properties_panel(&mut self.overlay_editor, ui);
                                    });
                                },
                            );
                        },
                    );

                    ui.separator();

                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui_timeline(&mut self.overlay_editor, &mut self.timeline_mode, ui);
                    });
                });
            }
            AppMode::DataControl => {
                ui_esports_data_panel(
                    &self.data_store,
                    &self.control_state,
                    &mut self.data_panel_state,
                    ui,
                );
            }
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
        if trigger_toggle_studio_mode {
            self.toggle_studio_mode();
        }
        if let Some(ref sc_id) = trigger_switch_scene_direct {
            self.switch_to_scene_direct(sc_id);
        }
        if trigger_open_add_source {
            self.source_dialog.open_add();
        }
        if let Some(path) = trigger_open_add_source_with_path {
            self.source_dialog.open_add_media_with_path(path);
        }
        if let Some(ref edit_id) = trigger_open_edit_source {
            if let Some(src) = self
                .project
                .preview_sources()
                .into_iter()
                .find(|s| &s.id == edit_id)
            {
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
        if trigger_snapshot_undo {
            self.studio_snapshot_for_undo();
        }
        if let Some(ref rem_id) = trigger_remove_source {
            let is_locked = self
                .project
                .preview_sources()
                .iter()
                .find(|s| &s.id == rem_id)
                .map(|s| s.locked)
                .unwrap_or(false);
            if is_locked {
                self.source_warning_message =
                    Some(("Unlock to delete".to_string(), Instant::now()));
            } else {
                self.studio_snapshot_for_undo();
                self.remove_source_by_id(rem_id);
            }
        }
        if let Some(ref vis_id) = trigger_toggle_source_visibility {
            self.studio_snapshot_for_undo();
            self.toggle_source_visibility(vis_id);
        }
        if let Some(ref lock_id) = trigger_toggle_source_lock {
            self.studio_snapshot_for_undo();
            self.toggle_source_lock(lock_id);
        }
        if let Some((ref src_id, x, y, width, height, rot)) = trigger_transform_source {
            self.sync_source_transform_to_compositor(src_id, x, y, width, height, rot);
        }
        if trigger_add_scene {
            self.studio_snapshot_for_undo();
            self.add_new_scene();
        }
        if let Some(ref rem_sc) = trigger_remove_scene {
            self.studio_snapshot_for_undo();
            self.remove_scene_by_id(rem_sc);
        }
        if let Some((ref sc_id, ref new_name)) = trigger_rename_scene {
            self.studio_snapshot_for_undo();
            self.rename_scene(sc_id, new_name);
        }
        if let Some(ref sc_id) = trigger_move_scene_up {
            self.studio_snapshot_for_undo();
            self.move_scene_up(sc_id);
        }
        if let Some(ref sc_id) = trigger_move_scene_down {
            self.studio_snapshot_for_undo();
            self.move_scene_down(sc_id);
        }
        if let Some(ref sc_id) = trigger_sync_source_order {
            self.sync_scene_sources_order_to_compositor(sc_id);
        }
        if trigger_paste_reference {
            self.paste_source_reference();
        }
        if trigger_paste_duplicate {
            self.paste_source_duplicate();
        }
        if let Some((src_id_opt, is_prev)) = trigger_save_screenshot {
            self.save_screenshot(src_id_opt.as_deref(), is_prev);
        }
        if let Some(ref s_id) = trigger_resize_output_to_source {
            self.resize_output_to_source(s_id);
        }
        if let Some(ref s_id) = trigger_open_filters {
            self.open_source_filters(s_id);
        }
        if let Some(ref s_id) = trigger_copy_filters {
            self.copy_source_filters(s_id);
        }
        if let Some(ref s_id) = trigger_paste_filters {
            self.paste_source_filters(s_id);
        }
        if trigger_audio_change {
            if let Some(ref act_id) = self.project.active_scene_id {
                if let Some(scene) = self.project.scenes.iter().find(|s| &s.id == act_id) {
                    let mut registry = self.source_registry.write();
                    for src_def in &scene.sources {
                        let sid = string_to_source_id(&src_def.id);
                        if let Some(src) = registry.get_mut(sid) {
                            let eff_vol = if self.master_muted {
                                0.0
                            } else {
                                src_def.volume * self.master_volume
                            };
                            let eff_muted = src_def.muted || self.master_muted;
                            let mut props = src.properties();
                            match &mut props {
                                SourceProperties::AudioInput { volume, muted, .. } => {
                                    *volume = eff_vol;
                                    *muted = eff_muted;
                                }
                                SourceProperties::AudioOutput { volume, muted, .. } => {
                                    *volume = eff_vol;
                                    *muted = eff_muted;
                                }
                                _ => {}
                            }
                            if let (Some(dev), Some(q)) = (&self.wgpu_device, &self.wgpu_queue) {
                                let _ = src.update_properties(&props, dev, q);
                            }
                        }
                    }
                }
            }
        }

        // Open Project Modal Dialog
        if self.show_open_dialog {
            let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
            let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
            if esc {
                self.show_open_dialog = false;
            }
            Window::new("Open Project")
                .collapsible(false)
                .resizable(false)
                .default_size(vec2(480.0, 160.0))
                .show(ctx, |ui| {
                    ui.label("Project file (.arxproj):");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&self.file_dialog_path).monospace(),
                            )
                            .truncate(),
                        );
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Zenocast Project", &["arxproj"])
                                .add_filter("All Files", &["*"])
                                .pick_file()
                            {
                                self.file_dialog_path = path.to_string_lossy().to_string();
                            }
                        }
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let open_clicked = ui.button("Open").clicked()
                            || (enter && !self.file_dialog_path.trim().is_empty());
                        if open_clicked {
                            let path = PathBuf::from(&self.file_dialog_path);
                            let _ = self.open_project_from_path(path);
                            self.show_open_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_open_dialog = false;
                        }
                    });
                });
        }

        // Save Project As Modal Dialog
        if self.show_save_as_dialog {
            let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
            let enter = ctx.input(|i| i.key_pressed(egui::Key::Enter));
            if esc {
                self.show_save_as_dialog = false;
            }
            Window::new("Save Project As")
                .collapsible(false)
                .resizable(false)
                .default_size(vec2(480.0, 160.0))
                .show(ctx, |ui| {
                    ui.label("Destination file (.arxproj):");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&self.file_dialog_path).monospace(),
                            )
                            .truncate(),
                        );
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Zenocast Project", &["arxproj"])
                                .add_filter("All Files", &["*"])
                                .save_file()
                            {
                                self.file_dialog_path = path.to_string_lossy().to_string();
                            }
                        }
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let save_clicked = ui.button("Save").clicked()
                            || (enter && !self.file_dialog_path.trim().is_empty());
                        if save_clicked {
                            let path = PathBuf::from(&self.file_dialog_path);
                            let _ = self.save_project_to_path(path);
                            self.show_save_as_dialog = false;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_save_as_dialog = false;
                        }
                    });
                });
        }

        // Render Source Modal Dialog if open
        if let Some(action) = self.source_dialog.ui_with_hotkeys(
            ctx,
            &mut self.project.settings.hotkeys.registry,
            &mut self.rebinding_target,
        ) {
            self.create_and_add_source(action);
        }

        // Render Filter Modal Dialog if open
        if let Some(action) = self.filter_dialog.ui(ctx, self.clipboard_filters.is_some()) {
            match action {
                crate::filter_dialog::FilterDialogAction::Update { source_id, filters } => {
                    self.sync_source_filters(&source_id, filters);
                }
                crate::filter_dialog::FilterDialogAction::CopyFilters { filters } => {
                    self.clipboard_filters = Some(filters);
                }
                crate::filter_dialog::FilterDialogAction::PasteFilters { source_id } => {
                    self.paste_source_filters(&source_id);
                }
            }
        }

        // Settings Dialog Window (OBS-style tabbed preferences)
        if self.show_settings {
            let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
            if esc && self.rebinding_target.is_none() {
                self.show_settings = false;
            }
            Window::new("Studio Settings")
                .collapsible(false)
                .resizable(true)
                .default_size(vec2(820.0, 580.0))
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        // Left sidebar: Category tabs
                        ui.allocate_ui_with_layout(
                            vec2(130.0, ui.available_height()),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                egui::Frame::group(ui.style()).show(ui, |ui| {
                                    ui.set_min_width(120.0);
                                    if ui
                                        .selectable_label(
                                            self.settings_tab == SettingsTab::Video,
                                            "Video",
                                        )
                                        .clicked()
                                    {
                                        self.settings_tab = SettingsTab::Video;
                                    }
                                    if ui
                                        .selectable_label(
                                            self.settings_tab == SettingsTab::Audio,
                                            "Audio",
                                        )
                                        .clicked()
                                    {
                                        self.settings_tab = SettingsTab::Audio;
                                    }
                                    if ui
                                        .selectable_label(
                                            self.settings_tab == SettingsTab::Output,
                                            "Output",
                                        )
                                        .clicked()
                                    {
                                        self.settings_tab = SettingsTab::Output;
                                    }
                                    if ui
                                        .selectable_label(
                                            self.settings_tab == SettingsTab::Hotkeys,
                                            "Hotkeys",
                                        )
                                        .clicked()
                                    {
                                        self.settings_tab = SettingsTab::Hotkeys;
                                    }
                                    if ui
                                        .selectable_label(
                                            self.settings_tab == SettingsTab::Licensing,
                                            "Licensing",
                                        )
                                        .clicked()
                                    {
                                        self.settings_tab = SettingsTab::Licensing;
                                    }
                                });
                            },
                        );

                        ui.separator();

                        // Right panel: Content for selected tab
                        ui.allocate_ui_with_layout(
                            vec2(ui.available_width(), ui.available_height()),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                match self.settings_tab {
                                    SettingsTab::Video => {
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
                                            ui.colored_label(
                                                TEXT,
                                                format!("{:.0}", self.project.settings.video.fps),
                                            );
                                        });
                                    }
                                    SettingsTab::Audio => {
                                        ui.heading("Audio Settings");
                                        ui.horizontal(|ui| {
                                            ui.label("Sample Rate:");
                                            ui.colored_label(
                                                TEXT,
                                                format!(
                                                    "{} Hz",
                                                    self.project.settings.audio.sample_rate
                                                ),
                                            );
                                            ui.label("Channels:");
                                            ui.colored_label(
                                                TEXT,
                                                format!("{}", self.project.settings.audio.channels),
                                            );
                                        });
                                    }
                                    SettingsTab::Output => {
                                        ui.heading("Output & Stream Settings");
                                        ui.horizontal(|ui| {
                                            ui.label("Service:");
                                            ui.text_edit_singleline(
                                                &mut self.project.settings.output.stream_service,
                                            );
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Server URL:");
                                            ui.text_edit_singleline(
                                                &mut self.project.settings.output.stream_url,
                                            );
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Stream Key:");
                                            ui.text_edit_singleline(
                                                &mut self.project.settings.output.stream_key,
                                            );
                                        });
                                        ui.horizontal(|ui| {
                                            ui.label("Bitrate (kbps):");
                                            ui.add(egui::Slider::new(
                                                &mut self
                                                    .project
                                                    .settings
                                                    .output
                                                    .video_bitrate_kbps,
                                                1000..=20000,
                                            ));
                                        });
                                    }
                                    SettingsTab::Hotkeys => {
                                        self.render_hotkeys_settings_tab(ui);
                                    }
                                    SettingsTab::Licensing => {
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
                                                    ui.colored_label(TEXT, "Licensed & Active");
                                                });
                                                ui.horizontal(|ui| {
                                                    ui.label("Edition:");
                                                    ui.colored_label(
                                                        TEXT,
                                                        certificate.edition.display_name(),
                                                    );
                                                });
                                            }
                                            LicenseState::GracePeriod {
                                                offline_since_sec, ..
                                            } => {
                                                ui.horizontal(|ui| {
                                                    ui.label("Status:");
                                                    ui.colored_label(
                                                        AMBER_WARN,
                                                        "Offline Grace Period Active",
                                                    );
                                                });
                                                let days_offline = (now_sec
                                                    .saturating_sub(*offline_since_sec))
                                                    / 86400;
                                                let days_left = 14u64.saturating_sub(days_offline);
                                                ui.label(format!(
                                                    "Offline for {} days ({} days remaining)",
                                                    days_offline, days_left
                                                ));
                                            }
                                            LicenseState::Restricted { reason } => {
                                                ui.horizontal(|ui| {
                                                    ui.label("Status:");
                                                    ui.colored_label(
                                                        RED_LIVE,
                                                        "Restricted Mode (Output Gated)",
                                                    );
                                                });
                                                ui.colored_label(RED_LIVE, reason.message());
                                            }
                                        }

                                        if let Some(wm) =
                                            AntiTamperGuard::get_watermark_text(&license_state)
                                        {
                                            ui.colored_label(
                                                AMBER_WARN,
                                                format!("Watermark Active: '{}'", wm),
                                            );
                                        }

                                        ui.horizontal(|ui| {
                                            ui.label("Device Fingerprint:");
                                            let fp = zeno_license::get_device_fingerprint();
                                            let short_fp =
                                                if fp.len() >= 16 { &fp[..16] } else { &fp };
                                            ui.colored_label(
                                                TEXT_MUTED,
                                                format!("{}...", short_fp),
                                            );
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
                                    }
                                }

                                ui.separator();
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.button("Close").clicked() {
                                            self.show_settings = false;
                                        }
                                    },
                                );
                            },
                        );
                    });
                });
        }

        // Render First-Run Setup Wizard if open
        self.wizard.ui(ctx, &mut self.project.settings);
    }

    /// Renders the complete OBS-parity Hotkeys settings panel.
    pub fn render_hotkeys_settings_tab(&mut self, ui: &mut egui::Ui) {
        ui.heading("Keyboard Shortcuts & Hotkeys");
        ui.label(
            "Configure application shortcuts and global hotkeys. Global hotkeys work even when Zenocast is not focused.",
        );
        ui.add_space(4.0);

        // Top Controls: Filter & Reset to Defaults
        ui.horizontal(|ui| {
            ui.label("Filter:");
            ui.add(
                egui::TextEdit::singleline(&mut self.hotkey_search_filter)
                    .hint_text("Search by action name, key, or category..."),
            );
            if !self.hotkey_search_filter.is_empty() && ui.button("Clear Filter").clicked() {
                self.hotkey_search_filter.clear();
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button("Reset to Defaults")
                    .on_hover_text(
                        "Reset all keyboard shortcuts and global flags to original defaults",
                    )
                    .clicked()
                {
                    self.project.settings.hotkeys.registry.reset_to_defaults();
                    self.sync_global_hotkeys();
                }
            });
        });

        // Active Rebinding Banner
        if let Some(ref target) = self.rebinding_target.clone() {
            let desc = match target {
                RebindTarget::Action(id) => crate::hotkey_registry::MASTER_ACTION_DEFINITIONS
                    .iter()
                    .find(|m| m.id == *id)
                    .map(|m| m.name)
                    .unwrap_or("Action"),
                RebindTarget::SceneSwitch(_) => "Switch to Scene",
                RebindTarget::SceneSwitchBack(_) => "Switch to Previous Scene",
                RebindTarget::SourceMute(_) => "Source Mute / Unmute",
                RebindTarget::SourcePushToMute(_) => "Source Push-to-Mute",
                RebindTarget::SourcePushToTalk(_) => "Source Push-to-Talk",
                RebindTarget::SourceShowHide(_) => "Source Show / Hide",
            };
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(
                        ACCENT,
                        format!(
                            "Press any key combination to bind '{}' (Esc to cancel)...",
                            desc
                        ),
                    );
                    if ui.button("Cancel").clicked() {
                        self.rebinding_target = None;
                    }
                });
            });
        }

        // Conflict Warnings Banner
        let conflicts = self.project.settings.hotkeys.registry.find_conflicts();
        if !conflicts.is_empty() {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.colored_label(
                    AMBER_WARN,
                    format!(
                        "Warning: {} key combination(s) are assigned to multiple actions:",
                        conflicts.len()
                    ),
                );
                for c in &conflicts {
                    ui.colored_label(
                        AMBER_WARN,
                        format!("  - {}: {}", c.combo, c.action_names.join(", ")),
                    );
                }
            });
        }

        ui.add_space(4.0);

        // Scrollable list of categories and actions
        egui::ScrollArea::vertical().show(ui, |ui| {
            let query = self.hotkey_search_filter.trim().to_lowercase();

            let categories = [
                ActionCategory::StreamingAndOutput,
                ActionCategory::SceneAndTransition,
                ActionCategory::LiveStudioEditing,
                ActionCategory::OverlayEditor,
                ActionCategory::WindowAndModes,
            ];

            let mut action_to_rebind: Option<RebindTarget> = None;
            let mut action_to_clear: Option<RebindTarget> = None;
            let mut toggle_global_action: Option<(ActionId, bool)> = None;

            for cat in categories {
                let actions: Vec<&'static crate::hotkey_registry::ActionMetadata> =
                    crate::hotkey_registry::MASTER_ACTION_DEFINITIONS
                        .iter()
                        .filter(|m| m.category == cat)
                        .filter(|m| {
                            if query.is_empty() {
                                true
                            } else {
                                m.name.to_lowercase().contains(&query)
                                    || m.description.to_lowercase().contains(&query)
                                    || self
                                        .project
                                        .settings
                                        .hotkeys
                                        .registry
                                        .accelerator_text(m.id)
                                        .to_lowercase()
                                        .contains(&query)
                                    || m.default_combo_str.to_lowercase().contains(&query)
                            }
                        })
                        .collect();

                if actions.is_empty() {
                    continue;
                }

                egui::CollapsingHeader::new(format!("{}. {}", cat.section_num(), cat.title()))
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new(format!("grid_{:?}", cat))
                            .striped(true)
                            .num_columns(5)
                            .spacing([12.0, 6.0])
                            .show(ui, |ui| {
                                ui.strong("Action");
                                ui.strong("Key Binding");
                                ui.label("");
                                ui.strong("Make Global");
                                ui.strong("Default");
                                ui.end_row();

                                for meta in actions {
                                    // Action name + tooltip
                                    ui.label(meta.name).on_hover_text(meta.description);

                                    // Key binding button
                                    let cur_text = self
                                        .project
                                        .settings
                                        .hotkeys
                                        .registry
                                        .accelerator_text(meta.id);
                                    let display_text = if cur_text.is_empty() {
                                        "None"
                                    } else {
                                        &cur_text
                                    };
                                    let is_rebinding = self.rebinding_target
                                        == Some(RebindTarget::Action(meta.id));
                                    let btn_text = if is_rebinding {
                                        "Press keys..."
                                    } else {
                                        display_text
                                    };

                                    if ui.button(btn_text).clicked() {
                                        action_to_rebind = Some(RebindTarget::Action(meta.id));
                                    }

                                    // Clear button
                                    if ui.button("X").on_hover_text("Clear shortcut").clicked() {
                                        action_to_clear = Some(RebindTarget::Action(meta.id));
                                    }

                                    // Global checkbox
                                    if meta.global_eligible {
                                        let mut is_glob = self
                                            .project
                                            .settings
                                            .hotkeys
                                            .registry
                                            .is_global(meta.id);
                                        if ui
                                            .checkbox(&mut is_glob, "")
                                            .on_hover_text(
                                                "Make shortcut active globally across the OS",
                                            )
                                            .changed()
                                        {
                                            toggle_global_action = Some((meta.id, is_glob));
                                        }
                                    } else {
                                        ui.colored_label(TEXT_MUTED, "-");
                                    }

                                    // Default hint
                                    let def_str = if meta.default_combo_str.is_empty() {
                                        "(unbound)"
                                    } else {
                                        meta.default_combo_str
                                    };
                                    ui.colored_label(TEXT_MUTED, def_str);

                                    ui.end_row();
                                }
                            });
                    });
            }

            if let Some(target) = action_to_rebind {
                self.rebinding_target = Some(target);
            }
            if let Some(target) = action_to_clear {
                self.project.settings.hotkeys.registry.clear_target(&target);
                self.sync_global_hotkeys();
            }
            if let Some((id, is_glob)) = toggle_global_action {
                self.project
                    .settings
                    .hotkeys
                    .registry
                    .set_global(id, is_glob);
                self.sync_global_hotkeys();
            }

            // Section 6: Per-Scene Hotkeys
            let scenes_info: Vec<(String, String)> = self
                .project
                .scenes
                .iter()
                .map(|s| (s.id.clone(), s.name.clone()))
                .filter(|(id, name)| {
                    query.is_empty()
                        || name.to_lowercase().contains(&query)
                        || id.to_lowercase().contains(&query)
                        || "scenes".contains(&query)
                })
                .collect();

            if !scenes_info.is_empty() {
                egui::CollapsingHeader::new("6. Per-Scene Hotkeys")
                    .default_open(true)
                    .show(ui, |ui| {
                        let mut scene_rebind: Option<RebindTarget> = None;
                        let mut scene_clear: Option<RebindTarget> = None;
                        let mut scene_global_change = false;

                        for (sc_id, sc_name) in &scenes_info {
                            egui::CollapsingHeader::new(format!("Scene: {}", sc_name))
                                .default_open(true)
                                .show(ui, |ui| {
                                    egui::Grid::new(format!("grid_sc_{}", sc_id))
                                        .striped(true)
                                        .num_columns(4)
                                        .spacing([12.0, 6.0])
                                        .show(ui, |ui| {
                                            // Switch to Scene
                                            ui.label("Switch to Scene");
                                            let cur = self
                                                .project
                                                .settings
                                                .hotkeys
                                                .registry
                                                .get_target_combo_str(&RebindTarget::SceneSwitch(
                                                    sc_id.clone(),
                                                ));
                                            let text = if self.rebinding_target
                                                == Some(RebindTarget::SceneSwitch(sc_id.clone()))
                                            {
                                                "Press keys..."
                                            } else if cur.is_empty() {
                                                "None"
                                            } else {
                                                &cur
                                            };
                                            if ui.button(text).clicked() {
                                                scene_rebind =
                                                    Some(RebindTarget::SceneSwitch(sc_id.clone()));
                                            }
                                            if ui.button("X").clicked() {
                                                scene_clear =
                                                    Some(RebindTarget::SceneSwitch(sc_id.clone()));
                                            }
                                            let mut glob = self
                                                .project
                                                .settings
                                                .hotkeys
                                                .registry
                                                .scene_hotkeys
                                                .get(sc_id)
                                                .map(|h| h.switch_to_global)
                                                .unwrap_or(false);
                                            if ui.checkbox(&mut glob, "Global").changed() {
                                                self.project
                                                    .settings
                                                    .hotkeys
                                                    .registry
                                                    .scene_hotkeys
                                                    .entry(sc_id.clone())
                                                    .or_default()
                                                    .switch_to_global = glob;
                                                scene_global_change = true;
                                            }
                                            ui.end_row();

                                            // Switch to Previous Scene
                                            ui.label("Switch to Previous Scene");
                                            let cur = self
                                                .project
                                                .settings
                                                .hotkeys
                                                .registry
                                                .get_target_combo_str(
                                                    &RebindTarget::SceneSwitchBack(sc_id.clone()),
                                                );
                                            let text = if self.rebinding_target
                                                == Some(RebindTarget::SceneSwitchBack(
                                                    sc_id.clone(),
                                                )) {
                                                "Press keys..."
                                            } else if cur.is_empty() {
                                                "None"
                                            } else {
                                                &cur
                                            };
                                            if ui.button(text).clicked() {
                                                scene_rebind = Some(RebindTarget::SceneSwitchBack(
                                                    sc_id.clone(),
                                                ));
                                            }
                                            if ui.button("X").clicked() {
                                                scene_clear = Some(RebindTarget::SceneSwitchBack(
                                                    sc_id.clone(),
                                                ));
                                            }
                                            let mut glob = self
                                                .project
                                                .settings
                                                .hotkeys
                                                .registry
                                                .scene_hotkeys
                                                .get(sc_id)
                                                .map(|h| h.switch_back_global)
                                                .unwrap_or(false);
                                            if ui.checkbox(&mut glob, "Global").changed() {
                                                self.project
                                                    .settings
                                                    .hotkeys
                                                    .registry
                                                    .scene_hotkeys
                                                    .entry(sc_id.clone())
                                                    .or_default()
                                                    .switch_back_global = glob;
                                                scene_global_change = true;
                                            }
                                            ui.end_row();
                                        });
                                });
                        }

                        if let Some(target) = scene_rebind {
                            self.rebinding_target = Some(target);
                        }
                        if let Some(target) = scene_clear {
                            self.project.settings.hotkeys.registry.clear_target(&target);
                            self.sync_global_hotkeys();
                        }
                        if scene_global_change {
                            self.sync_global_hotkeys();
                        }
                    });
            }

            // Section 6: Per-Source Hotkeys
            let mut unique_sources: Vec<(String, String)> = Vec::new();
            for sc in &self.project.scenes {
                for src in &sc.sources {
                    if !unique_sources.iter().any(|(id, _)| id == &src.id) {
                        unique_sources.push((src.id.clone(), src.name.clone()));
                    }
                }
            }

            let sources_info: Vec<(String, String)> = unique_sources
                .into_iter()
                .filter(|(id, name)| {
                    query.is_empty()
                        || name.to_lowercase().contains(&query)
                        || id.to_lowercase().contains(&query)
                        || "sources".contains(&query)
                })
                .collect();

            if !sources_info.is_empty() {
                egui::CollapsingHeader::new("6. Per-Source Hotkeys")
                    .default_open(true)
                    .show(ui, |ui| {
                        let mut source_rebind: Option<RebindTarget> = None;
                        let mut source_clear: Option<RebindTarget> = None;
                        let mut source_global_change = false;

                        for (src_id, src_name) in &sources_info {
                            egui::CollapsingHeader::new(format!("Source: {}", src_name))
                                .default_open(false)
                                .show(ui, |ui| {
                                    egui::Grid::new(format!("grid_src_{}", src_id))
                                        .striped(true)
                                        .num_columns(4)
                                        .spacing([12.0, 6.0])
                                        .show(ui, |ui| {
                                            // Mute / Unmute
                                            ui.label("Mute / Unmute");
                                            let cur = self
                                                .project
                                                .settings
                                                .hotkeys
                                                .registry
                                                .get_target_combo_str(&RebindTarget::SourceMute(
                                                    src_id.clone(),
                                                ));
                                            let text = if self.rebinding_target
                                                == Some(RebindTarget::SourceMute(src_id.clone()))
                                            {
                                                "Press keys..."
                                            } else if cur.is_empty() {
                                                "None"
                                            } else {
                                                &cur
                                            };
                                            if ui.button(text).clicked() {
                                                source_rebind =
                                                    Some(RebindTarget::SourceMute(src_id.clone()));
                                            }
                                            if ui.button("X").clicked() {
                                                source_clear =
                                                    Some(RebindTarget::SourceMute(src_id.clone()));
                                            }
                                            let mut glob = self
                                                .project
                                                .settings
                                                .hotkeys
                                                .registry
                                                .source_hotkeys
                                                .get(src_id)
                                                .map(|h| h.mute_global)
                                                .unwrap_or(false);
                                            if ui.checkbox(&mut glob, "Global").changed() {
                                                self.project
                                                    .settings
                                                    .hotkeys
                                                    .registry
                                                    .source_hotkeys
                                                    .entry(src_id.clone())
                                                    .or_default()
                                                    .mute_global = glob;
                                                source_global_change = true;
                                            }
                                            ui.end_row();

                                            // Push-to-Mute
                                            ui.label("Push-to-Mute");
                                            let cur = self
                                                .project
                                                .settings
                                                .hotkeys
                                                .registry
                                                .get_target_combo_str(
                                                    &RebindTarget::SourcePushToMute(src_id.clone()),
                                                );
                                            let text = if self.rebinding_target
                                                == Some(RebindTarget::SourcePushToMute(
                                                    src_id.clone(),
                                                )) {
                                                "Press keys..."
                                            } else if cur.is_empty() {
                                                "None"
                                            } else {
                                                &cur
                                            };
                                            if ui.button(text).clicked() {
                                                source_rebind = Some(
                                                    RebindTarget::SourcePushToMute(src_id.clone()),
                                                );
                                            }
                                            if ui.button("X").clicked() {
                                                source_clear = Some(
                                                    RebindTarget::SourcePushToMute(src_id.clone()),
                                                );
                                            }
                                            let mut glob = self
                                                .project
                                                .settings
                                                .hotkeys
                                                .registry
                                                .source_hotkeys
                                                .get(src_id)
                                                .map(|h| h.push_to_mute_global)
                                                .unwrap_or(false);
                                            if ui.checkbox(&mut glob, "Global").changed() {
                                                self.project
                                                    .settings
                                                    .hotkeys
                                                    .registry
                                                    .source_hotkeys
                                                    .entry(src_id.clone())
                                                    .or_default()
                                                    .push_to_mute_global = glob;
                                                source_global_change = true;
                                            }
                                            ui.end_row();

                                            // Push-to-Talk
                                            ui.label("Push-to-Talk");
                                            let cur = self
                                                .project
                                                .settings
                                                .hotkeys
                                                .registry
                                                .get_target_combo_str(
                                                    &RebindTarget::SourcePushToTalk(src_id.clone()),
                                                );
                                            let text = if self.rebinding_target
                                                == Some(RebindTarget::SourcePushToTalk(
                                                    src_id.clone(),
                                                )) {
                                                "Press keys..."
                                            } else if cur.is_empty() {
                                                "None"
                                            } else {
                                                &cur
                                            };
                                            if ui.button(text).clicked() {
                                                source_rebind = Some(
                                                    RebindTarget::SourcePushToTalk(src_id.clone()),
                                                );
                                            }
                                            if ui.button("X").clicked() {
                                                source_clear = Some(
                                                    RebindTarget::SourcePushToTalk(src_id.clone()),
                                                );
                                            }
                                            let mut glob = self
                                                .project
                                                .settings
                                                .hotkeys
                                                .registry
                                                .source_hotkeys
                                                .get(src_id)
                                                .map(|h| h.push_to_talk_global)
                                                .unwrap_or(false);
                                            if ui.checkbox(&mut glob, "Global").changed() {
                                                self.project
                                                    .settings
                                                    .hotkeys
                                                    .registry
                                                    .source_hotkeys
                                                    .entry(src_id.clone())
                                                    .or_default()
                                                    .push_to_talk_global = glob;
                                                source_global_change = true;
                                            }
                                            ui.end_row();

                                            // Show / Hide
                                            ui.label("Show / Hide");
                                            let cur = self
                                                .project
                                                .settings
                                                .hotkeys
                                                .registry
                                                .get_target_combo_str(
                                                    &RebindTarget::SourceShowHide(src_id.clone()),
                                                );
                                            let text = if self.rebinding_target
                                                == Some(RebindTarget::SourceShowHide(
                                                    src_id.clone(),
                                                )) {
                                                "Press keys..."
                                            } else if cur.is_empty() {
                                                "None"
                                            } else {
                                                &cur
                                            };
                                            if ui.button(text).clicked() {
                                                source_rebind = Some(RebindTarget::SourceShowHide(
                                                    src_id.clone(),
                                                ));
                                            }
                                            if ui.button("X").clicked() {
                                                source_clear = Some(RebindTarget::SourceShowHide(
                                                    src_id.clone(),
                                                ));
                                            }
                                            let mut glob = self
                                                .project
                                                .settings
                                                .hotkeys
                                                .registry
                                                .source_hotkeys
                                                .get(src_id)
                                                .map(|h| h.show_hide_global)
                                                .unwrap_or(false);
                                            if ui.checkbox(&mut glob, "Global").changed() {
                                                self.project
                                                    .settings
                                                    .hotkeys
                                                    .registry
                                                    .source_hotkeys
                                                    .entry(src_id.clone())
                                                    .or_default()
                                                    .show_hide_global = glob;
                                                source_global_change = true;
                                            }
                                            ui.end_row();
                                        });
                                });
                        }

                        if let Some(target) = source_rebind {
                            self.rebinding_target = Some(target);
                        }
                        if let Some(target) = source_clear {
                            self.project.settings.hotkeys.registry.clear_target(&target);
                            self.sync_global_hotkeys();
                        }
                        if source_global_change {
                            self.sync_global_hotkeys();
                        }
                    });
            }
        });
    }

    /// Drives a complete frame: advances engine state and renders UI.
    pub fn update_ui(&mut self, ctx: &Context) {
        self.tick_engine(ctx);
        self.draw_ui(ctx);
    }

    /// Performs graceful shutdown of output streams, recordings, and active sources.
    pub fn clean_exit(&mut self) {
        if self.is_streaming {
            self.output_manager.stop_streaming();
            self.is_streaming = false;
        }
        if self.is_recording {
            let _ = self
                .output_manager
                .stop_recording(self.project.settings.output.remux_to_mp4);
            self.is_recording = false;
        }
        self.source_registry.write().clear();
        info!("Clean application exit completed");
    }
}

impl App for ZenocastApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        self.update_ui(ctx);
    }

    fn on_exit(&mut self) {
        self.clean_exit();
    }
}
