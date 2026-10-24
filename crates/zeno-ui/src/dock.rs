//! Dockable studio layout and panels for Live Mode using egui_dock.

use egui::{vec2, Align2, CornerRadius, FontId, Sense, Stroke, StrokeKind, Ui};
use egui_dock::{DockState, NodeIndex, TabViewer};
use serde::{Deserialize, Serialize};

use crate::project::Project;
use crate::theme::{
    ACCENT, AMBER_WARN, BORDER_WIDTH, BUTTON_HEIGHT, CORNER_RADIUS, GRID_UNIT, RED_LIVE, SURFACE_0,
    SURFACE_3, TEXT, TEXT_MUTED,
};
use crate::transform_gizmo::{TransformGizmoEvent, TransformGizmoItem, TransformGizmoState};

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
    pub preview_texture_id: Option<egui::TextureId>,
    pub program_texture_id: Option<egui::TextureId>,
    pub master_volume: &'a mut f32,
    pub master_muted: &'a mut bool,
    pub rename_scene_id: &'a mut Option<String>,
    pub rename_scene_buffer: &'a mut String,
    pub trigger_rename_scene: &'a mut Option<(String, String)>,
    pub trigger_move_scene_up: &'a mut Option<String>,
    pub trigger_move_scene_down: &'a mut Option<String>,
    pub trigger_audio_change: &'a mut bool,
    pub trigger_open_add_source: &'a mut bool,
    pub trigger_open_add_source_with_path: &'a mut Option<String>,
    pub trigger_open_edit_source: &'a mut Option<String>,
    pub trigger_remove_source: &'a mut Option<String>,
    pub trigger_toggle_source_visibility: &'a mut Option<String>,
    pub trigger_toggle_source_lock: &'a mut Option<String>,
    pub trigger_add_scene: &'a mut bool,
    pub trigger_remove_scene: &'a mut Option<String>,
    pub live_gizmo_state: &'a mut TransformGizmoState,
    pub trigger_transform_source: &'a mut Option<(String, f32, f32, f32, f32, f32)>,
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

    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        false
    }

    fn allowed_in_windows(&self, _tab: &mut Self::Tab) -> bool {
        false
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
                if let Some(tex_id) = self.preview_texture_id {
                    ui.painter().image(
                        tex_id,
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                } else {
                    ui.painter().rect(
                        rect,
                        CornerRadius::same(CORNER_RADIUS as u8),
                        SURFACE_0,
                        Stroke::NONE,
                        StrokeKind::Inside,
                    );
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        preview_name,
                        FontId::proportional(16.0),
                        TEXT_MUTED,
                    );
                }

                // Interactive on-canvas Transform Gizmo for Preview Monitor
                let preview_scene_id_opt = self
                    .project
                    .preview_scene_id
                    .clone()
                    .or_else(|| self.selected_scene_id.clone());
                let mut gizmo_items = Vec::new();
                if let Some(ref sc_id) = preview_scene_id_opt {
                    if let Some(scene) = self.project.scenes.iter().find(|s| &s.id == sc_id) {
                        for src in &scene.sources {
                            gizmo_items.push(TransformGizmoItem {
                                id: src.id.clone(),
                                name: src.name.clone(),
                                center_x: src.x + src.width * 0.5,
                                center_y: src.y + src.height * 0.5,
                                width: src.width,
                                height: src.height,
                                rotation_degrees: src.rotation,
                                locked: src.locked,
                                visible: src.visible,
                            });
                        }
                    }
                }

                let selected_ids = if let Some(ref sel_id) = *self.selected_source_id {
                    vec![sel_id.clone()]
                } else {
                    Vec::new()
                };

                let canvas_w = (self.project.settings.video.base_width as f32).max(1.0);
                let canvas_h = (self.project.settings.video.base_height as f32).max(1.0);
                let canvas_size = vec2(canvas_w, canvas_h);
                let zoom = rect.width() / canvas_size.x;

                let events = self.live_gizmo_state.draw_and_interact(
                    ui,
                    rect,
                    canvas_size,
                    egui::Vec2::ZERO,
                    zoom,
                    &gizmo_items,
                    &selected_ids,
                );

                for event in events {
                    match event {
                        TransformGizmoEvent::SelectOnly(id) => {
                            *self.selected_source_id = Some(id);
                        }
                        TransformGizmoEvent::ToggleSelect(id) => {
                            if self.selected_source_id.as_deref() == Some(&id) {
                                *self.selected_source_id = None;
                            } else {
                                *self.selected_source_id = Some(id);
                            }
                        }
                        TransformGizmoEvent::SelectMultiple(ids) => {
                            if let Some(first) = ids.first() {
                                *self.selected_source_id = Some(first.clone());
                            }
                        }
                        TransformGizmoEvent::DeselectAll => {
                            *self.selected_source_id = None;
                        }
                        TransformGizmoEvent::Transformed {
                            id,
                            center_x,
                            center_y,
                            width,
                            height,
                            rotation_degrees,
                        } => {
                            if let Some(ref sc_id) = preview_scene_id_opt {
                                if let Some(scene) =
                                    self.project.scenes.iter_mut().find(|s| &s.id == sc_id)
                                {
                                    if let Some(src) = scene.sources.iter_mut().find(|s| s.id == id)
                                    {
                                        src.x = center_x - width * 0.5;
                                        src.y = center_y - height * 0.5;
                                        src.width = width;
                                        src.height = height;
                                        src.rotation = rotation_degrees;
                                        *self.trigger_transform_source = Some((
                                            src.id.clone(),
                                            src.x,
                                            src.y,
                                            src.width,
                                            src.height,
                                            src.rotation,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }

                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(CORNER_RADIUS as u8),
                    Stroke::new(BORDER_WIDTH, AMBER_WARN),
                    StrokeKind::Inside,
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
                let resp = ui.add_sized(vec2(40.0, 100.0), slider);
                if resp.drag_stopped() && *self.t_bar_progress >= 0.95 {
                    *self.trigger_cut = true;
                    *self.t_bar_progress = 0.0;
                }
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
                if let Some(tex_id) = self.program_texture_id {
                    ui.painter().image(
                        tex_id,
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        egui::Color32::WHITE,
                    );
                } else {
                    ui.painter().rect(
                        rect,
                        CornerRadius::same(CORNER_RADIUS as u8),
                        SURFACE_0,
                        Stroke::NONE,
                        StrokeKind::Inside,
                    );
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        active_name,
                        FontId::proportional(16.0),
                        TEXT_MUTED,
                    );
                }
                ui.painter().rect_stroke(
                    rect,
                    CornerRadius::same(CORNER_RADIUS as u8),
                    Stroke::new(BORDER_WIDTH, border_color),
                    StrokeKind::Inside,
                );
            });
        });
    }

    /// Renders Scenes list with Add, Duplicate, Remove, and Switch operations.
    fn ui_scenes(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("+ Add Scene").clicked() {
                *self.trigger_add_scene = true;
            }

            let can_remove = self.project.scenes.len() > 1 && self.selected_scene_id.is_some();
            if ui
                .add_enabled(can_remove, egui::Button::new("- Remove"))
                .clicked()
            {
                if let Some(ref sel) = *self.selected_scene_id {
                    *self.trigger_remove_scene = Some(sel.clone());
                }
            }

            if let Some(ref sel) = *self.selected_scene_id {
                if ui.button("Rename").clicked() {
                    if let Some(sc) = self.project.scenes.iter().find(|s| &s.id == sel) {
                        *self.rename_scene_id = Some(sel.clone());
                        *self.rename_scene_buffer = sc.name.clone();
                    }
                }

                if ui.button("▲ Up").clicked() {
                    *self.trigger_move_scene_up = Some(sel.clone());
                }
                if ui.button("▼ Down").clicked() {
                    *self.trigger_move_scene_down = Some(sel.clone());
                }
            }
        });

        if let Some(sel_rename_id) = self.rename_scene_id.clone() {
            ui.horizontal(|ui| {
                ui.label("Name:");
                ui.text_edit_singleline(self.rename_scene_buffer);
                if ui.button("Save").clicked() {
                    *self.trigger_rename_scene =
                        Some((sel_rename_id.clone(), self.rename_scene_buffer.clone()));
                    *self.rename_scene_id = None;
                }
                if ui.button("Cancel").clicked() {
                    *self.rename_scene_id = None;
                }
            });
        }

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
            if ui.button("+ Add Source").clicked() {
                *self.trigger_open_add_source = true;
            }

            if ui.button("- Remove").clicked() {
                if let Some(ref sel_id) = *self.selected_source_id {
                    *self.trigger_remove_source = Some(sel_id.clone());
                }
            }

            if let Some(ref sel_id) = *self.selected_source_id {
                if ui.button("Properties").clicked() {
                    *self.trigger_open_edit_source = Some(sel_id.clone());
                }
            }
        });

        ui.separator();

        // Drag & drop file onto Sources panel
        let panel_rect = ui.max_rect();
        if ui.rect_contains_pointer(panel_rect) {
            ui.ctx().input(|i| {
                if let Some(dropped) = i.raw.dropped_files.first() {
                    if let Some(ref path) = dropped.path {
                        *self.trigger_open_add_source_with_path =
                            Some(path.to_string_lossy().to_string());
                    }
                }
            });
        }

        if scene.sources.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.colored_label(
                    TEXT_MUTED,
                    "No sources yet — click + Add Source or drag media here",
                );
            });
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for src in &mut scene.sources {
                ui.horizontal(|ui| {
                    // Visibility toggle
                    let vis_icon = if src.visible { "👁" } else { "---" };
                    if ui.selectable_label(src.visible, vis_icon).clicked() {
                        *self.trigger_toggle_source_visibility = Some(src.id.clone());
                    }

                    // Lock toggle
                    let lock_icon = if src.locked { "🔒" } else { "🔓" };
                    if ui.selectable_label(src.locked, lock_icon).clicked() {
                        *self.trigger_toggle_source_lock = Some(src.id.clone());
                    }

                    let is_sel = self.selected_source_id.as_deref() == Some(&src.id);
                    let resp = ui.selectable_label(is_sel, &src.name);
                    if resp.clicked() {
                        *self.selected_source_id = Some(src.id.clone());
                    }
                    if resp.double_clicked() {
                        *self.trigger_open_edit_source = Some(src.id.clone());
                    }
                });
            }
        });

        if let Some(ref sel_id) = *self.selected_source_id {
            if let Some(src) = scene.sources.iter_mut().find(|s| &s.id == sel_id) {
                ui.separator();
                ui.collapsing("Transform", |ui| {
                    let mut changed = false;
                    ui.horizontal(|ui| {
                        ui.label("X:");
                        changed |= ui
                            .add(egui::DragValue::new(&mut src.x).speed(1.0))
                            .changed();
                        ui.label("Y:");
                        changed |= ui
                            .add(egui::DragValue::new(&mut src.y).speed(1.0))
                            .changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Width:");
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut src.width)
                                    .speed(1.0)
                                    .range(1.0..=7680.0),
                            )
                            .changed();
                        ui.label("Height:");
                        changed |= ui
                            .add(
                                egui::DragValue::new(&mut src.height)
                                    .speed(1.0)
                                    .range(1.0..=4320.0),
                            )
                            .changed();
                    });
                    ui.horizontal(|ui| {
                        ui.label("Rotation (°):");
                        changed |= ui
                            .add(egui::DragValue::new(&mut src.rotation).speed(0.5))
                            .changed();
                        if ui.button("Reset").clicked() {
                            src.rotation = 0.0;
                            changed = true;
                        }
                    });
                    if changed {
                        *self.trigger_transform_source = Some((
                            src.id.clone(),
                            src.x,
                            src.y,
                            src.width,
                            src.height,
                            src.rotation,
                        ));
                    }
                });
            }
        }
    }

    /// Renders 6-track audio mixer sliders, mute buttons, and live meters.
    fn ui_audio_mixer(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            // Master Track
            ui.vertical(|ui| {
                ui.label("Master");
                let vol_resp = ui.add(
                    egui::Slider::new(self.master_volume, 0.0..=1.5)
                        .vertical()
                        .show_value(false),
                );
                if vol_resp.changed() {
                    *self.trigger_audio_change = true;
                }

                let master_db_text = if *self.master_volume <= 0.0 || *self.master_muted {
                    "-inf dB".to_string()
                } else {
                    format!("{:.1} dB", 20.0 * (*self.master_volume).log10())
                };
                ui.label(master_db_text);

                let mute_btn = ui.selectable_label(*self.master_muted, "Mute");
                if mute_btn.clicked() {
                    *self.master_muted = !*self.master_muted;
                    *self.trigger_audio_change = true;
                }
            });

            ui.separator();

            // Source Tracks
            if let Some(active_id) = self.project.active_scene_id.as_ref() {
                if let Some(scene) = self.project.scenes.iter_mut().find(|s| &s.id == active_id) {
                    for src in &mut scene.sources {
                        ui.vertical(|ui| {
                            ui.label(&src.name);
                            let src_vol_resp = ui.add(
                                egui::Slider::new(&mut src.volume, 0.0..=1.5)
                                    .vertical()
                                    .show_value(false),
                            );
                            if src_vol_resp.changed() {
                                *self.trigger_audio_change = true;
                            }

                            let db_text = if src.volume <= 0.0 || src.muted {
                                "-inf dB".to_string()
                            } else {
                                format!("{:.1} dB", 20.0 * src.volume.log10())
                            };
                            ui.label(db_text);

                            let mute_btn = ui.selectable_label(src.muted, "Mute");
                            if mute_btn.clicked() {
                                src.muted = !src.muted;
                                *self.trigger_audio_change = true;
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
