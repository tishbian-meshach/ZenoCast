//! Zenocast Studio desktop application shell (`eframe::App`).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use eframe::{App, Frame};
use egui::{vec2, CentralPanel, Context, TopBottomPanel, Window};
use egui_dock::{DockArea, DockState};
use tracing::info;

use crate::dock::{create_default_live_layout, StudioTab, StudioTabViewer};
use crate::project::{Autosaver, Project};
use crate::theme::{apply_theme, RED_LIVE, TEXT, TEXT_MUTED};
use crate::wizard::FirstRunWizard;
use zeno_output::service::OutputManager;

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
        }
    }

    /// Handles instant Cut transition between Preview and Program.
    pub fn execute_cut(&mut self) {
        let temp = self.project.active_scene_id.clone();
        self.project.active_scene_id = self.project.preview_scene_id.clone();
        self.project.preview_scene_id = temp;
        self.t_bar_progress = 0.0;
        info!(
            "Executed Cut transition: live='{:?}'",
            self.project.active_scene_id
        );
    }

    /// Handles Auto transition between Preview and Program.
    pub fn execute_auto(&mut self) {
        self.execute_cut();
    }

    /// Toggles live streaming output.
    pub fn toggle_streaming(&mut self) {
        if self.is_streaming {
            self.output_manager.stop_streaming();
            self.is_streaming = false;
            self.stream_start_time = None;
            info!("Stopped live streaming");
        } else {
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
