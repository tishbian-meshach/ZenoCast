//! Dockable studio layout and panels for Live Mode using egui_dock.

use egui::{vec2, Align2, CornerRadius, FontId, Sense, Stroke, StrokeKind, Ui};
use egui_dock::{DockState, NodeIndex, TabViewer};
use serde::{Deserialize, Serialize};

use crate::project::{Project, SceneDefinition, SourceDefinition, SourceKind};
use crate::theme::{
    ACCENT, AMBER_WARN, BORDER_WIDTH, BUTTON_HEIGHT, CORNER_RADIUS, GRID_UNIT, RED_LIVE, SURFACE_0,
    SURFACE_3, TEXT, TEXT_MUTED,
};

/// Studio panel tabs available in Live production mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StudioTab {
    PreviewProgram,
    Scenes,
    Sources,
    AudioMixer,
    Transitions,
    StreamControls,
}

impl StudioTab {
    /// Human-readable title for the tab bar.
    pub fn title(&self) -> &'static str {
        match self {
            Self::PreviewProgram => "Preview / Program",
            Self::Scenes => "Scenes",
            Self::Sources => "Sources",
            Self::AudioMixer => "Audio Mixer",
            Self::Transitions => "Transitions",
            Self::StreamControls => "Stream & Record",
        }
    }
}

/// Creates the broadcast studio default dock layout for Live mode.
pub fn create_default_live_layout() -> DockState<StudioTab> {
    let mut dock_state = DockState::new(vec![StudioTab::PreviewProgram]);

    // Split bottom area for studio control panels (taking ~45% bottom space)
    let [_top_node, bottom_node] =
        dock_state
            .main_surface_mut()
            .split_below(NodeIndex::root(), 0.55, vec![StudioTab::Scenes]);

    // Split bottom into Scenes (left), Sources (mid-left), Mixer (middle), Transitions (mid-right), Controls (right)
    let [_scenes, sources] =
        dock_state
            .main_surface_mut()
            .split_right(bottom_node, 0.20, vec![StudioTab::Sources]);

    let [_sources, mixer] =
        dock_state
            .main_surface_mut()
            .split_right(sources, 0.25, vec![StudioTab::AudioMixer]);

    let [_mixer, transitions] =
        dock_state
            .main_surface_mut()
            .split_right(mixer, 0.33, vec![StudioTab::Transitions]);

    let [_transitions, _controls] = dock_state.main_surface_mut().split_right(
        transitions,
        0.50,
        vec![StudioTab::StreamControls],
    );

    dock_state
}

/// Dynamic context passed into tab rendering.
pub struct StudioTabViewer<'a> {
    pub project: &'a mut Project,
    pub selected_scene_id: &'a mut Option<String>,
    pub selected_source_id: &'a mut Option<String>,
    pub is_streaming: bool,
    pub is_recording: bool,
    pub fps: f32,
    pub dropped_percent: f32,
    pub bitrate_kbps: u32,
    pub stream_time: String,
    pub active_transition_kind: &'a mut String,
    pub transition_duration_ms: &'a mut u32,
    pub t_bar_progress: &'a mut f32,
    pub trigger_cut: &'a mut bool,
    pub trigger_auto: &'a mut bool,
    pub trigger_toggle_stream: &'a mut bool,
    pub trigger_toggle_record: &'a mut bool,
}

impl<'a> TabViewer for StudioTabViewer<'a> {
    type Tab = StudioTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        tab.title().into()
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        match tab {
            StudioTab::PreviewProgram => self.ui_preview_program(ui),
            StudioTab::Scenes => self.ui_scenes(ui),
            StudioTab::Sources => self.ui_sources(ui),
            StudioTab::AudioMixer => self.ui_audio_mixer(ui),
            StudioTab::Transitions => self.ui_transitions(ui),
            StudioTab::StreamControls => self.ui_stream_controls(ui),
        }
    }
}

impl<'a> StudioTabViewer<'a> {
    /// Renders Preview and Program monitors with T-Bar and Quick Action bus.
    fn ui_preview_program(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let available_width = ui.available_width();
            let monitor_width = ((available_width - 120.0) / 2.0).max(160.0);
            let monitor_height = (monitor_width * 9.0 / 16.0).max(90.0);

            // Left: Preview Monitor
            ui.vertical(|ui| {
                let preview_name = self.project.preview_scene_id.as_deref().unwrap_or("None");
                ui.horizontal(|ui| {
                    ui.colored_label(AMBER_WARN, "● PREVIEW");
                    ui.colored_label(TEXT_MUTED, format!("({})", preview_name));
                });

                let (rect, _response) =
                    ui.allocate_exact_size(vec2(monitor_width, monitor_height), Sense::hover());
                ui.painter().rect(
                    rect,
                    CornerRadius::same(CORNER_RADIUS as u8),
                    SURFACE_0,
                    Stroke::new(BORDER_WIDTH, AMBER_WARN),
                    StrokeKind::Inside,
                );
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    preview_name,
                    FontId::proportional(16.0),
                    TEXT,
                );
            });

            // Center: T-Bar & Transition Buttons
            ui.vertical(|ui| {
                ui.add_space(20.0);
                if ui.button(" CUT ").clicked() {
                    *self.trigger_cut = true;
                }
                ui.add_space(GRID_UNIT);
                if ui.button(" AUTO ").clicked() {
                    *self.trigger_auto = true;
                }
                ui.add_space(GRID_UNIT);

                ui.label("T-BAR");
                let slider = egui::Slider::new(self.t_bar_progress, 0.0..=1.0)
                    .vertical()
                    .show_value(false);
                ui.add_sized(vec2(40.0, 100.0), slider);
            });

            // Right: Program Monitor
            ui.vertical(|ui| {
                let active_name = self.project.active_scene_id.as_deref().unwrap_or("None");
                ui.horizontal(|ui| {
                    if self.is_streaming || self.is_recording {
                        ui.colored_label(RED_LIVE, "● PROGRAM - ON AIR");
                    } else {
                        ui.colored_label(TEXT, "● PROGRAM");
                    }
                    ui.colored_label(TEXT_MUTED, format!("({})", active_name));
                });

                let (rect, _response) =
                    ui.allocate_exact_size(vec2(monitor_width, monitor_height), Sense::hover());
                let border_color = if self.is_streaming || self.is_recording {
                    RED_LIVE
                } else {
                    ACCENT
                };
                ui.painter().rect(
                    rect,
                    CornerRadius::same(CORNER_RADIUS as u8),
                    SURFACE_0,
                    Stroke::new(BORDER_WIDTH, border_color),
                    StrokeKind::Inside,
                );
                ui.painter().text(
                    rect.center(),
                    Align2::CENTER_CENTER,
                    active_name,
                    FontId::proportional(16.0),
                    TEXT,
                );
            });
        });
    }

    /// Renders Scenes list with Add, Duplicate, Remove, and Switch operations.
    fn ui_scenes(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("+ Add Scene").clicked() {
                let id = format!("scene_{}", self.project.scenes.len() + 1);
                let name = format!("Scene {}", self.project.scenes.len() + 1);
                self.project.scenes.push(SceneDefinition {
                    id: id.clone(),
                    name,
                    sources: Vec::new(),
                });
                *self.selected_scene_id = Some(id);
            }

            if ui.button("- Remove").clicked() {
                if let Some(ref sel) = *self.selected_scene_id {
                    self.project.scenes.retain(|s| &s.id != sel);
                    *self.selected_scene_id = self.project.scenes.first().map(|s| s.id.clone());
                }
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for scene in &self.project.scenes {
                let is_selected = self.selected_scene_id.as_deref() == Some(&scene.id);
                let is_active = self.project.active_scene_id.as_deref() == Some(&scene.id);
                let is_preview = self.project.preview_scene_id.as_deref() == Some(&scene.id);

                let label_text = format!(
                    "{} ({}){}{}",
                    scene.name,
                    scene.sources.len(),
                    if is_active { " [LIVE]" } else { "" },
                    if is_preview { " [PRV]" } else { "" }
                );

                let response = ui.selectable_label(is_selected, label_text);
                if response.clicked() {
                    *self.selected_scene_id = Some(scene.id.clone());
                    self.project.preview_scene_id = Some(scene.id.clone());
                }
                if response.double_clicked() {
                    self.project.active_scene_id = Some(scene.id.clone());
                }
            }
        });
    }

    /// Renders Sources list for the currently selected scene.
    fn ui_sources(&mut self, ui: &mut Ui) {
        let selected_scene = self
            .selected_scene_id
            .as_ref()
            .and_then(|id| self.project.scenes.iter_mut().find(|s| &s.id == id));

        let Some(scene) = selected_scene else {
            ui.colored_label(TEXT_MUTED, "Select a scene to manage sources");
            return;
        };

        ui.horizontal(|ui| {
            egui::ComboBox::from_label("")
                .selected_text("+ Add Source")
                .show_ui(ui, |ui| {
                    if ui.selectable_label(false, "Display Capture").clicked() {
                        let id = format!("src_disp_{}", scene.sources.len() + 1);
                        scene.sources.push(SourceDefinition {
                            id: id.clone(),
                            name: "Display Capture".into(),
                            kind: SourceKind::DisplayCapture,
                            target: "Primary Monitor".into(),
                            visible: true,
                            locked: false,
                            volume: 1.0,
                            muted: false,
                            x: 0.0,
                            y: 0.0,
                            width: 1920.0,
                            height: 1080.0,
                        });
                        *self.selected_source_id = Some(id);
                    }

                    if ui.selectable_label(false, "Window Capture").clicked() {
                        let id = format!("src_win_{}", scene.sources.len() + 1);
                        scene.sources.push(SourceDefinition {
                            id: id.clone(),
                            name: "Window Capture".into(),
                            kind: SourceKind::WindowCapture,
                            target: "Application Window".into(),
                            visible: true,
                            locked: false,
                            volume: 1.0,
                            muted: false,
                            x: 0.0,
                            y: 0.0,
                            width: 1280.0,
                            height: 720.0,
                        });
                        *self.selected_source_id = Some(id);
                    }

                    if ui.selectable_label(false, "Camera Feed").clicked() {
                        let id = format!("src_cam_{}", scene.sources.len() + 1);
                        scene.sources.push(SourceDefinition {
                            id: id.clone(),
                            name: "Camera Feed".into(),
                            kind: SourceKind::CameraFeed,
                            target: "Media Foundation Device".into(),
                            visible: true,
                            locked: false,
                            volume: 1.0,
                            muted: false,
                            x: 1280.0,
                            y: 720.0,
                            width: 640.0,
                            height: 360.0,
                        });
                        *self.selected_source_id = Some(id);
                    }

                    if ui.selectable_label(false, "ARX Overlay (.arx)").clicked() {
                        let id = format!("src_arx_{}", scene.sources.len() + 1);
                        scene.sources.push(SourceDefinition {
                            id: id.clone(),
                            name: "Lower-Third Overlay".into(),
                            kind: SourceKind::ArxOverlay,
                            target: "templates/esports/lower_third.arx".into(),
                            visible: true,
                            locked: false,
                            volume: 1.0,
                            muted: false,
                            x: 0.0,
                            y: 0.0,
                            width: 1920.0,
                            height: 1080.0,
                        });
                        *self.selected_source_id = Some(id);
                    }

                    if ui.selectable_label(false, "Browser Source (Web)").clicked() {
                        let id = format!("src_browser_{}", scene.sources.len() + 1);
                        scene.sources.push(SourceDefinition {
                            id: id.clone(),
                            name: "Web Scoreboard".into(),
                            kind: SourceKind::Browser,
                            target: "https://zenocast.live/scoreboard".into(),
                            visible: true,
                            locked: false,
                            volume: 1.0,
                            muted: false,
                            x: 0.0,
                            y: 0.0,
                            width: 1920.0,
                            height: 1080.0,
                        });
                        *self.selected_source_id = Some(id);
                    }
                });

            if ui.button("- Delete").clicked() {
                if let Some(ref sel_id) = *self.selected_source_id {
                    scene.sources.retain(|s| &s.id != sel_id);
                    *self.selected_source_id = scene.sources.first().map(|s| s.id.clone());
                }
            }
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            for src in &mut scene.sources {
                ui.horizontal(|ui| {
                    // Visibility toggle
                    let vis_icon = if src.visible { "👁" } else { "---" };
                    if ui.selectable_label(src.visible, vis_icon).clicked() {
                        src.visible = !src.visible;
                    }

                    // Lock toggle
                    let lock_icon = if src.locked { "🔒" } else { "🔓" };
                    if ui.selectable_label(src.locked, lock_icon).clicked() {
                        src.locked = !src.locked;
                    }

                    let is_sel = self.selected_source_id.as_deref() == Some(&src.id);
                    if ui.selectable_label(is_sel, &src.name).clicked() {
                        *self.selected_source_id = Some(src.id.clone());
                    }
                });
            }
        });
    }

    /// Renders 6-track audio mixer sliders, mute buttons, and live meters.
    fn ui_audio_mixer(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Master Track
            ui.vertical(|ui| {
                ui.label("Master");
                let mut vol = 1.0f32;
                ui.add(
                    egui::Slider::new(&mut vol, 0.0..=1.5)
                        .vertical()
                        .show_value(false),
                );
                ui.label("0.0 dB");
                let _ = ui.button("Mute");
            });

            ui.separator();

            // Source Tracks
            if let Some(active_id) = self.project.active_scene_id.as_ref() {
                if let Some(scene) = self.project.scenes.iter_mut().find(|s| &s.id == active_id) {
                    for src in &mut scene.sources {
                        ui.vertical(|ui| {
                            ui.label(&src.name);
                            ui.add(
                                egui::Slider::new(&mut src.volume, 0.0..=1.5)
                                    .vertical()
                                    .show_value(false),
                            );
                            let db = 20.0 * src.volume.max(0.001).log10();
                            ui.label(format!("{:.1} dB", db));
                            let mute_btn = ui.selectable_label(src.muted, "Mute");
                            if mute_btn.clicked() {
                                src.muted = !src.muted;
                            }
                        });
                    }
                }
            }
        });
    }

    /// Renders live transition controls (kind, duration, trigger).
    fn ui_transitions(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.label("Active Transition");
            egui::ComboBox::from_label("")
                .selected_text(self.active_transition_kind.as_str())
                .show_ui(ui, |ui| {
                    ui.selectable_value(self.active_transition_kind, "Cut".into(), "Cut");
                    ui.selectable_value(
                        self.active_transition_kind,
                        "Fade".into(),
                        "Fade (Dissolve)",
                    );
                    ui.selectable_value(
                        self.active_transition_kind,
                        "Slide".into(),
                        "Slide (Left-to-Right)",
                    );
                    ui.selectable_value(
                        self.active_transition_kind,
                        "Wipe".into(),
                        "Wipe (Horizontal Soft)",
                    );
                    ui.selectable_value(
                        self.active_transition_kind,
                        "Stinger".into(),
                        "Stinger Video (.arx)",
                    );
                });

            ui.add_space(GRID_UNIT);
            ui.label("Duration (ms)");
            ui.add(egui::Slider::new(self.transition_duration_ms, 50..=2000).suffix(" ms"));

            ui.add_space(GRID_UNIT);
            if ui.button("Trigger Transition (Auto)").clicked() {
                *self.trigger_auto = true;
            }
        });
    }

    /// Renders streaming and recording toggles and real-time broadcast statistics.
    fn ui_stream_controls(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            // Stream button
            let stream_text = if self.is_streaming {
                "■ STOP STREAMING"
            } else {
                "▶ START STREAMING"
            };
            let stream_btn = ui.add_sized(
                vec2(ui.available_width(), BUTTON_HEIGHT),
                egui::Button::new(stream_text).fill(if self.is_streaming {
                    RED_LIVE
                } else {
                    ACCENT
                }),
            );
            if stream_btn.clicked() {
                *self.trigger_toggle_stream = true;
            }

            ui.add_space(GRID_UNIT);

            // Record button
            let record_text = if self.is_recording {
                "■ STOP RECORDING"
            } else {
                "● START RECORDING"
            };
            let record_btn = ui.add_sized(
                vec2(ui.available_width(), BUTTON_HEIGHT),
                egui::Button::new(record_text).fill(if self.is_recording {
                    RED_LIVE
                } else {
                    SURFACE_3
                }),
            );
            if record_btn.clicked() {
                *self.trigger_toggle_record = true;
            }

            ui.separator();

            // Real-Time Output Statistics
            ui.horizontal(|ui| {
                ui.colored_label(TEXT_MUTED, "Status:");
                if self.is_streaming {
                    ui.colored_label(RED_LIVE, "LIVE");
                } else {
                    ui.colored_label(TEXT, "OFFLINE");
                }
            });

            ui.horizontal(|ui| {
                ui.colored_label(TEXT_MUTED, "FPS:");
                ui.colored_label(TEXT, format!("{:.1}", self.fps));
            });

            ui.horizontal(|ui| {
                ui.colored_label(TEXT_MUTED, "Dropped:");
                let drop_color = if self.dropped_percent > 0.1 {
                    AMBER_WARN
                } else {
                    TEXT
                };
                ui.colored_label(drop_color, format!("{:.2}%", self.dropped_percent));
            });

            ui.horizontal(|ui| {
                ui.colored_label(TEXT_MUTED, "Bitrate:");
                ui.colored_label(TEXT, format!("{} kbps", self.bitrate_kbps));
            });

            ui.horizontal(|ui| {
                ui.colored_label(TEXT_MUTED, "Duration:");
                ui.colored_label(TEXT, &self.stream_time);
            });
        });
    }
}
