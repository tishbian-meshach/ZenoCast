//! Dockable studio layout and panels for Live Mode using egui_dock.

use egui::{vec2, Align2, CornerRadius, FontId, Sense, Stroke, StrokeKind, Ui};
use egui_dock::{DockState, NodeIndex, TabViewer};
use serde::{Deserialize, Serialize};

use crate::button::Button;
use crate::icon::{
    grip_handle_colored, icon_button, icon_button_enabled, icon_toggle_button_colored,
    paint_lucide_icon, IconButton, IconKind,
};
use crate::project::Project;
use crate::theme::{
    ACCENT, AMBER_WARN, BORDER_WIDTH, CORNER_RADIUS, GRID_UNIT, RED_LIVE, ROW_HEIGHT_LIST,
    SURFACE_2, TEXT, TEXT_MUTED, TRANSPARENT,
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
            .split_right(bottom_node, 0.18, vec![StudioTab::Sources]);

    let [_sources, mixer] =
        dock_state
            .main_surface_mut()
            .split_right(sources, 0.26, vec![StudioTab::AudioMixer]);

    let [_mixer, transitions] =
        dock_state
            .main_surface_mut()
            .split_right(mixer, 0.45, vec![StudioTab::Transitions]);

    let [_transitions, _controls] = dock_state.main_surface_mut().split_right(
        transitions,
        0.42,
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
    pub trigger_sync_source_order: &'a mut Option<String>,
    pub trigger_snapshot_undo: &'a mut bool,
    pub rebinding_target: &'a mut Option<crate::hotkey_registry::RebindTarget>,
    pub source_warning_message: &'a mut Option<(String, std::time::Instant)>,
    pub studio_mode: bool,
    pub trigger_toggle_studio_mode: &'a mut bool,
    pub trigger_switch_scene_direct: &'a mut Option<String>,
    pub clipboard_source: &'a mut Option<crate::project::SourceDefinition>,
    pub preview_locked: &'a mut bool,
    pub preview_scaling_mode: &'a mut crate::project::PreviewScalingMode,
    pub trigger_paste_reference: &'a mut bool,
    pub trigger_paste_duplicate: &'a mut bool,
    pub trigger_save_screenshot: &'a mut Option<(Option<String>, bool)>,
    pub trigger_resize_output_to_source: &'a mut Option<String>,
    pub trigger_open_filters: &'a mut Option<String>,
    pub trigger_copy_filters: &'a mut Option<String>,
    pub trigger_paste_filters: &'a mut Option<String>,
    pub has_clipboard_filters: bool,
    pub trigger_change_blend_mode: &'a mut Option<(String, zeno_engine::BlendMode)>,
    pub trigger_change_blend_method: &'a mut Option<(String, crate::project::BlendingMethod)>,
}

impl<'a> TabViewer for StudioTabViewer<'a> {
    type Tab = StudioTab;

    fn title(&mut self, tab: &mut Self::Tab) -> egui::WidgetText {
        match tab {
            StudioTab::PreviewProgram => {
                if self.studio_mode {
                    "Preview / Program".into()
                } else {
                    "Program".into()
                }
            }
            _ => tab.title().into(),
        }
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
    /// Renders Preview and Program monitors with T-Bar and Quick Action bus in Studio Mode,
    /// or single Program monitor in Normal Mode.
    fn ui_preview_program(&mut self, ui: &mut Ui) {
        if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            *self.selected_source_id = None;
        }
        if self.studio_mode {
            self.ui_studio_mode_monitors(ui);
        } else {
            self.ui_normal_mode_monitor(ui);
        }
    }

    /// Renders dual Preview and Program monitors with T-Bar and Quick Action bus.
    fn ui_studio_mode_monitors(&mut self, ui: &mut Ui) {
        let avail_w = ui.available_width().max(320.0);
        let avail_h = (ui.available_height() - 28.0).max(90.0);

        const CONTROLS_COL_WIDTH: f32 = 90.0;
        let max_w_per_monitor = ((avail_w - CONTROLS_COL_WIDTH) * 0.5).max(120.0);
        let max_h_per_monitor = avail_h;

        let (monitor_width, monitor_height) = if max_w_per_monitor * 9.0 / 16.0 <= max_h_per_monitor
        {
            (max_w_per_monitor, max_w_per_monitor * 9.0 / 16.0)
        } else {
            (max_h_per_monitor * 16.0 / 9.0, max_h_per_monitor)
        };

        let total_content_w = 2.0 * monitor_width + CONTROLS_COL_WIDTH;
        let x_pad = ((avail_w - total_content_w) * 0.5).max(0.0);

        ui.horizontal(|ui| {
            if x_pad > 0.0 {
                ui.add_space(x_pad);
            }

            // Left: Preview Monitor
            ui.vertical(|ui| {
                let preview_name = self.project.preview_scene_id.as_deref().unwrap_or("None");
                ui.horizontal(|ui| {
                    ui.colored_label(AMBER_WARN, "PREVIEW");
                    ui.colored_label(TEXT_MUTED, format!("({})", preview_name));
                });

                let (rect, _response) =
                    ui.allocate_exact_size(vec2(monitor_width, monitor_height), Sense::hover());
                crate::theme::paint_checkerboard(ui.painter(), rect, 16.0);
                if let Some(tex_id) = self.preview_texture_id {
                    ui.painter().image(
                        tex_id,
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        crate::theme::TINT_NONE,
                    );
                } else {
                    ui.painter().text(
                        rect.center(),
                        Align2::CENTER_CENTER,
                        preview_name,
                        FontId::proportional(16.0),
                        TEXT_MUTED,
                    );
                }

                // Interactive on-canvas Transform Gizmo for Preview Monitor
                let mut gizmo_items = Vec::new();
                for src in self.project.editable_sources(self.studio_mode) {
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

                let selected_ids = if let Some(ref sel_id) = *self.selected_source_id {
                    vec![sel_id.clone()]
                } else {
                    Vec::new()
                };

                let canvas_w = (self.project.settings.video.base_width as f32).max(1.0);
                let canvas_h = (self.project.settings.video.base_height as f32).max(1.0);
                let canvas_size = vec2(canvas_w, canvas_h);
                let zoom = rect.width() / canvas_size.x;

                let (canvas_response, events) =
                    self.live_gizmo_state.draw_and_interact_with_response(
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
                        TransformGizmoEvent::DragStarted => {
                            *self.trigger_snapshot_undo = true;
                        }
                        TransformGizmoEvent::Transformed {
                            id,
                            center_x,
                            center_y,
                            width,
                            height,
                            rotation_degrees,
                        } => {
                            if let Some(sources) =
                                self.project.editable_sources_mut(self.studio_mode)
                            {
                                if let Some(src) = sources.iter_mut().find(|s| s.id == id) {
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

                // Context menu on the Preview Canvas (Right-Click support)
                self.render_canvas_context_menu(&canvas_response);

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
                let cut_acc = self
                    .project
                    .settings
                    .hotkeys
                    .registry
                    .accelerator_text(crate::hotkey_registry::ActionId::Cut);
                let cut_tooltip = if cut_acc.is_empty() {
                    "Cut (Preview Ã¢â€ â€™ Program immediately)".to_string()
                } else {
                    format!("Cut (Preview Ã¢â€ â€™ Program immediately) [{}]", cut_acc)
                };
                if Button::new("CUT")
                    .primary()
                    .tooltip(cut_tooltip)
                    .show(ui)
                    .clicked()
                {
                    *self.trigger_cut = true;
                }
                ui.add_space(GRID_UNIT);
                let auto_acc = self
                    .project
                    .settings
                    .hotkeys
                    .registry
                    .accelerator_text(crate::hotkey_registry::ActionId::AutoTransition);
                let auto_tooltip = if auto_acc.is_empty() {
                    "Trigger Transition (Auto)".to_string()
                } else {
                    format!("Trigger Transition (Auto) [{}]", auto_acc)
                };
                if Button::new("AUTO")
                    .secondary()
                    .tooltip(auto_tooltip)
                    .show(ui)
                    .clicked()
                {
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
                        ui.colored_label(RED_LIVE, "PROGRAM - ON AIR");
                    } else {
                        ui.colored_label(TEXT, "PROGRAM");
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
                crate::theme::paint_checkerboard(ui.painter(), rect, 16.0);
                if let Some(tex_id) = self.program_texture_id {
                    ui.painter().image(
                        tex_id,
                        rect,
                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                        crate::theme::TINT_NONE,
                    );
                } else {
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

    /// Renders all OBS-parity context menu items for a specific source.
    pub fn render_source_context_menu_items(
        &mut self,
        ui: &mut egui::Ui,
        sel_id: &str,
        canvas_w: f32,
        canvas_h: f32,
    ) {
        let sources = self.project.editable_sources(self.studio_mode);
        let src_idx_opt = sources.iter().position(|s| s.id == sel_id);
        let src_def_opt = src_idx_opt.and_then(|i| sources.get(i).cloned());
        let src_name = src_def_opt
            .as_ref()
            .map(|s| s.name.clone())
            .unwrap_or_else(|| sel_id.to_string());
        let is_locked = src_def_opt.as_ref().map(|s| s.locked).unwrap_or(false);
        let sc_id_opt = self.project.editable_scene_id(self.studio_mode);
        let num_sources = sources.len();

        ui.label(egui::RichText::new(&src_name).strong());
        ui.separator();

        // 1. Add Source
        if ui.button("+ Add Source...").clicked() {
            *self.trigger_open_add_source = true;
            ui.close_menu();
        }

        // 2. Studio Mode: Enable / Lock Preview
        if self.studio_mode {
            ui.checkbox(self.preview_locked, "Lock Preview");
        }

        // 3. Preview Scaling Submenu
        ui.menu_button("Preview Scaling", |ui| {
            if ui
                .selectable_value(
                    self.preview_scaling_mode,
                    crate::project::PreviewScalingMode::ScaleToWindow,
                    "Scale to Window (Fit)",
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .selectable_value(
                    self.preview_scaling_mode,
                    crate::project::PreviewScalingMode::Canvas1To1,
                    "Canvas (1:1)",
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .selectable_value(
                    self.preview_scaling_mode,
                    crate::project::PreviewScalingMode::Output1To1,
                    "Output (1:1)",
                )
                .clicked()
            {
                ui.close_menu();
            }
        });

        // 4. Projector
        ui.menu_button("Projector", |ui| {
            if ui.button("Open Windowed Projector (Source)").clicked() {
                ui.close_menu();
            }
            if ui.button("Open Fullscreen Projector (Source)").clicked() {
                ui.close_menu();
            }
        });

        // 5. Screenshot
        if ui.button("Save Source Screenshot (PNG)...").clicked() {
            *self.trigger_save_screenshot = Some((Some(sel_id.to_string()), false));
            ui.close_menu();
        }

        ui.separator();

        // 6. Scale Filtering Submenu
        ui.menu_button("Scale Filtering", |ui| {
            let cur_filter = src_def_opt
                .as_ref()
                .map(|s| s.scale_filter)
                .unwrap_or_default();
            for &(filter, label) in &[
                (
                    crate::project::ScaleFilter::Point,
                    "Point (Nearest Neighbor)",
                ),
                (crate::project::ScaleFilter::Bilinear, "Bilinear"),
                (crate::project::ScaleFilter::Bicubic, "Bicubic"),
                (crate::project::ScaleFilter::Lanczos, "Lanczos"),
            ] {
                let is_cur = cur_filter == filter;
                if ui.selectable_label(is_cur, label).clicked() {
                    *self.trigger_snapshot_undo = true;
                    if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                        if let Some(s) = srcs.iter_mut().find(|s| s.id == sel_id) {
                            s.scale_filter = filter;
                        }
                    }
                    ui.close_menu();
                }
            }
        });

        // 7. Blending Mode Submenu
        ui.menu_button("Blending Mode", |ui| {
            let cur_mode = src_def_opt
                .as_ref()
                .map(|s| s.blend_mode)
                .unwrap_or_default();
            for &(mode, label) in &[
                (zeno_engine::BlendMode::Normal, "Normal"),
                (zeno_engine::BlendMode::Add, "Add"),
                (zeno_engine::BlendMode::Subtract, "Subtract"),
                (zeno_engine::BlendMode::Multiply, "Multiply"),
                (zeno_engine::BlendMode::Screen, "Screen"),
                (zeno_engine::BlendMode::Overlay, "Overlay"),
                (zeno_engine::BlendMode::Darken, "Darken"),
                (zeno_engine::BlendMode::Lighten, "Lighten"),
                (zeno_engine::BlendMode::ColorDodge, "Color Dodge"),
                (zeno_engine::BlendMode::ColorBurn, "Color Burn"),
                (zeno_engine::BlendMode::Difference, "Difference"),
            ] {
                let is_cur = cur_mode == mode;
                if ui.selectable_label(is_cur, label).clicked() {
                    *self.trigger_snapshot_undo = true;
                    if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                        if let Some(s) = srcs.iter_mut().find(|s| s.id == sel_id) {
                            s.blend_mode = mode;
                        }
                    }
                    *self.trigger_change_blend_mode = Some((sel_id.to_string(), mode));
                    ui.close_menu();
                }
            }
        });

        // 8. Blending Method Submenu
        ui.menu_button("Blending Method", |ui| {
            let cur_method = src_def_opt
                .as_ref()
                .map(|s| s.blend_method)
                .unwrap_or_default();
            for &(method, label) in &[
                (crate::project::BlendingMethod::Linear, "Default"),
                (crate::project::BlendingMethod::SrgbOff, "SRGB Off (Linear)"),
            ] {
                let is_cur = cur_method == method;
                if ui.selectable_label(is_cur, label).clicked() {
                    *self.trigger_snapshot_undo = true;
                    if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                        if let Some(s) = srcs.iter_mut().find(|s| s.id == sel_id) {
                            s.blend_method = method;
                        }
                    }
                    *self.trigger_change_blend_method = Some((sel_id.to_string(), method));
                    ui.close_menu();
                }
            }
        });

        // 9. Show / Hide Transitions
        ui.menu_button("Show Transition", |ui| {
            let cur_tr = src_def_opt.as_ref().and_then(|s| s.show_transition.clone());
            for &(val, label) in &[
                (None, "None"),
                (Some("Cut"), "Cut"),
                (Some("Fade"), "Fade"),
                (Some("Slide"), "Slide"),
            ] {
                let is_cur = cur_tr.as_deref() == val;
                if ui.selectable_label(is_cur, label).clicked() {
                    *self.trigger_snapshot_undo = true;
                    if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                        if let Some(s) = srcs.iter_mut().find(|s| s.id == sel_id) {
                            s.show_transition = val.map(String::from);
                        }
                    }
                    ui.close_menu();
                }
            }
        });
        ui.menu_button("Hide Transition", |ui| {
            let cur_tr = src_def_opt.as_ref().and_then(|s| s.hide_transition.clone());
            for &(val, label) in &[
                (None, "None"),
                (Some("Cut"), "Cut"),
                (Some("Fade"), "Fade"),
                (Some("Slide"), "Slide"),
            ] {
                let is_cur = cur_tr.as_deref() == val;
                if ui.selectable_label(is_cur, label).clicked() {
                    *self.trigger_snapshot_undo = true;
                    if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                        if let Some(s) = srcs.iter_mut().find(|s| s.id == sel_id) {
                            s.hide_transition = val.map(String::from);
                        }
                    }
                    ui.close_menu();
                }
            }
        });

        ui.separator();

        // 10. Resize output (source size)
        if ui.button("Resize output (source size)").clicked() {
            *self.trigger_resize_output_to_source = Some(sel_id.to_string());
            ui.close_menu();
        }

        ui.separator();

        // 11. Order Submenu
        let can_front = src_idx_opt.is_some_and(|i| i + 1 < num_sources);
        let can_back = src_idx_opt.is_some_and(|i| i > 0);
        ui.menu_button("Order", |ui| {
            if ui
                .add_enabled(can_front, egui::Button::new("Move to Top\tCtrl+Shift+]"))
                .clicked()
            {
                if let (Some(idx), Some(sc_id)) = (src_idx_opt, sc_id_opt.clone()) {
                    if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                        *self.trigger_snapshot_undo = true;
                        if idx < srcs.len() {
                            let s = srcs.remove(idx);
                            srcs.push(s);
                            *self.trigger_sync_source_order = Some(sc_id);
                        }
                    }
                }
                ui.close_menu();
            }
            if ui
                .add_enabled(can_front, egui::Button::new("Move Up\tCtrl+]"))
                .clicked()
            {
                if let (Some(idx), Some(sc_id)) = (src_idx_opt, sc_id_opt.clone()) {
                    if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                        *self.trigger_snapshot_undo = true;
                        if idx + 1 < srcs.len() {
                            srcs.swap(idx, idx + 1);
                            *self.trigger_sync_source_order = Some(sc_id);
                        }
                    }
                }
                ui.close_menu();
            }
            if ui
                .add_enabled(can_back, egui::Button::new("Move Down\tCtrl+["))
                .clicked()
            {
                if let (Some(idx), Some(sc_id)) = (src_idx_opt, sc_id_opt.clone()) {
                    if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                        *self.trigger_snapshot_undo = true;
                        if idx > 0 && idx < srcs.len() {
                            srcs.swap(idx, idx - 1);
                            *self.trigger_sync_source_order = Some(sc_id);
                        }
                    }
                }
                ui.close_menu();
            }
            if ui
                .add_enabled(can_back, egui::Button::new("Move to Bottom\tCtrl+Shift+["))
                .clicked()
            {
                if let (Some(idx), Some(sc_id)) = (src_idx_opt, sc_id_opt.clone()) {
                    if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                        *self.trigger_snapshot_undo = true;
                        if idx < srcs.len() {
                            let s = srcs.remove(idx);
                            srcs.insert(0, s);
                            *self.trigger_sync_source_order = Some(sc_id);
                        }
                    }
                }
                ui.close_menu();
            }
        });

        // 12. Transform Submenu
        ui.menu_button("Transform", |ui| {
            if ui.button("Reset Transform\tCtrl+R").clicked() {
                *self.trigger_snapshot_undo = true;
                if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                    if let Some(s) = srcs.iter_mut().find(|s| s.id == *sel_id) {
                        s.x = 0.0;
                        s.y = 0.0;
                        s.rotation = 0.0;
                        *self.trigger_transform_source =
                            Some((s.id.clone(), s.x, s.y, s.width, s.height, s.rotation));
                    }
                }
                ui.close_menu();
            }
            if ui.button("Rotate 90Ã‚Â° CW").clicked() {
                *self.trigger_snapshot_undo = true;
                if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                    if let Some(s) = srcs.iter_mut().find(|s| s.id == *sel_id) {
                        s.rotation = (s.rotation + 90.0) % 360.0;
                        *self.trigger_transform_source =
                            Some((s.id.clone(), s.x, s.y, s.width, s.height, s.rotation));
                    }
                }
                ui.close_menu();
            }
            if ui.button("Rotate 90Ã‚Â° CCW").clicked() {
                *self.trigger_snapshot_undo = true;
                if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                    if let Some(s) = srcs.iter_mut().find(|s| s.id == *sel_id) {
                        s.rotation = (s.rotation - 90.0 + 360.0) % 360.0;
                        *self.trigger_transform_source =
                            Some((s.id.clone(), s.x, s.y, s.width, s.height, s.rotation));
                    }
                }
                ui.close_menu();
            }
            if ui.button("Rotate 180Ã‚Â°").clicked() {
                *self.trigger_snapshot_undo = true;
                if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                    if let Some(s) = srcs.iter_mut().find(|s| s.id == *sel_id) {
                        s.rotation = (s.rotation + 180.0) % 360.0;
                        *self.trigger_transform_source =
                            Some((s.id.clone(), s.x, s.y, s.width, s.height, s.rotation));
                    }
                }
                ui.close_menu();
            }
            if ui.button("Flip Horizontal").clicked() {
                *self.trigger_snapshot_undo = true;
                if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                    if let Some(s) = srcs.iter_mut().find(|s| s.id == *sel_id) {
                        s.rotation = (360.0 - s.rotation) % 360.0;
                        *self.trigger_transform_source =
                            Some((s.id.clone(), s.x, s.y, s.width, s.height, s.rotation));
                    }
                }
                ui.close_menu();
            }
            if ui.button("Flip Vertical").clicked() {
                *self.trigger_snapshot_undo = true;
                if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                    if let Some(s) = srcs.iter_mut().find(|s| s.id == *sel_id) {
                        s.rotation = (180.0 - s.rotation + 360.0) % 360.0;
                        *self.trigger_transform_source =
                            Some((s.id.clone(), s.x, s.y, s.width, s.height, s.rotation));
                    }
                }
                ui.close_menu();
            }
            if ui.button("Fit to Screen\tCtrl+F").clicked() {
                *self.trigger_snapshot_undo = true;
                if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                    if let Some(s) = srcs.iter_mut().find(|s| s.id == *sel_id) {
                        let aspect = if s.height > 0.0 {
                            s.width / s.height
                        } else {
                            16.0 / 9.0
                        };
                        if canvas_w / aspect <= canvas_h {
                            s.width = canvas_w;
                            s.height = canvas_w / aspect;
                        } else {
                            s.height = canvas_h;
                            s.width = canvas_h * aspect;
                        }
                        s.x = (canvas_w - s.width) * 0.5;
                        s.y = (canvas_h - s.height) * 0.5;
                        s.rotation = 0.0;
                        *self.trigger_transform_source =
                            Some((s.id.clone(), s.x, s.y, s.width, s.height, s.rotation));
                    }
                }
                ui.close_menu();
            }
            if ui.button("Stretch to Screen").clicked() {
                *self.trigger_snapshot_undo = true;
                if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                    if let Some(s) = srcs.iter_mut().find(|s| s.id == *sel_id) {
                        s.x = 0.0;
                        s.y = 0.0;
                        s.width = canvas_w;
                        s.height = canvas_h;
                        s.rotation = 0.0;
                        *self.trigger_transform_source =
                            Some((s.id.clone(), s.x, s.y, s.width, s.height, s.rotation));
                    }
                }
                ui.close_menu();
            }
            if ui.button("Center to Screen\tCtrl+D").clicked() {
                *self.trigger_snapshot_undo = true;
                if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                    if let Some(s) = srcs.iter_mut().find(|s| s.id == *sel_id) {
                        s.x = (canvas_w - s.width) * 0.5;
                        s.y = (canvas_h - s.height) * 0.5;
                        *self.trigger_transform_source =
                            Some((s.id.clone(), s.x, s.y, s.width, s.height, s.rotation));
                    }
                }
                ui.close_menu();
            }
            if ui.button("Center Horizontally").clicked() {
                *self.trigger_snapshot_undo = true;
                if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                    if let Some(s) = srcs.iter_mut().find(|s| s.id == *sel_id) {
                        s.x = (canvas_w - s.width) * 0.5;
                        *self.trigger_transform_source =
                            Some((s.id.clone(), s.x, s.y, s.width, s.height, s.rotation));
                    }
                }
                ui.close_menu();
            }
            if ui.button("Center Vertically").clicked() {
                *self.trigger_snapshot_undo = true;
                if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                    if let Some(s) = srcs.iter_mut().find(|s| s.id == *sel_id) {
                        s.y = (canvas_h - s.height) * 0.5;
                        *self.trigger_transform_source =
                            Some((s.id.clone(), s.x, s.y, s.width, s.height, s.rotation));
                    }
                }
                ui.close_menu();
            }
        });

        // 13. Group Selected Items
        if ui.button("Group Selected Items\tCtrl+G").clicked() {
            *self.trigger_snapshot_undo = true;
            if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                if let Some(s) = srcs.iter_mut().find(|s| s.id == *sel_id) {
                    s.name = format!("[Group] {}", s.name);
                }
            }
            ui.close_menu();
        }

        ui.separator();

        // 14. Copy / Paste (Reference) / Paste (Duplicate)
        if ui.button("Copy\tCtrl+C").clicked() {
            if let Some(ref s) = src_def_opt {
                *self.clipboard_source = Some(s.clone());
            }
            ui.close_menu();
        }
        let can_paste = self.clipboard_source.is_some();
        if ui
            .add_enabled(
                can_paste,
                egui::Button::new("Paste (Reference)\tCtrl+Shift+V"),
            )
            .clicked()
        {
            *self.trigger_paste_reference = true;
            ui.close_menu();
        }
        if ui
            .add_enabled(can_paste, egui::Button::new("Paste (Duplicate)\tCtrl+V"))
            .clicked()
        {
            *self.trigger_paste_duplicate = true;
            ui.close_menu();
        }

        if ui.button("Copy Filters").clicked() {
            *self.trigger_copy_filters = Some(sel_id.to_string());
            ui.close_menu();
        }
        if ui
            .add_enabled(
                self.has_clipboard_filters,
                egui::Button::new("Paste Filters"),
            )
            .clicked()
        {
            *self.trigger_paste_filters = Some(sel_id.to_string());
            ui.close_menu();
        }

        ui.separator();

        // 15. Toggle Visibility & Lock
        let vis_text = if src_def_opt.as_ref().map(|s| s.visible).unwrap_or(true) {
            "Hide Source"
        } else {
            "Show Source"
        };
        if ui.button(vis_text).clicked() {
            *self.trigger_toggle_source_visibility = Some(sel_id.to_string());
            ui.close_menu();
        }
        let lock_text = if is_locked {
            "Unlock Source"
        } else {
            "Lock Source"
        };
        if ui.button(lock_text).clicked() {
            *self.trigger_toggle_source_lock = Some(sel_id.to_string());
            ui.close_menu();
        }

        // 16. Rename
        if ui.button("Rename...").clicked() {
            *self.rename_scene_id = Some(sel_id.to_string());
            *self.rename_scene_buffer = src_name.clone();
            ui.close_menu();
        }

        // 17. Remove Source (respects lock safety block)
        let del_btn = if is_locked {
            egui::Button::new("Remove Source (Locked)")
        } else {
            egui::Button::new("Remove Source\tDelete")
        };
        if ui.add(del_btn).clicked() {
            if is_locked {
                *self.source_warning_message =
                    Some(("Unlock to delete".to_string(), std::time::Instant::now()));
            } else {
                *self.trigger_remove_source = Some(sel_id.to_string());
            }
            ui.close_menu();
        }

        ui.separator();

        // 18. Filters & Properties
        if ui.button("Filters...").clicked() {
            *self.trigger_open_filters = Some(sel_id.to_string());
            ui.close_menu();
        }
        if ui.button("Properties...").clicked() {
            *self.trigger_open_edit_source = Some(sel_id.to_string());
            ui.close_menu();
        }

        ui.separator();
        ui.label(egui::RichText::new("Source Hotkeys (OBS Parity)").strong());
        let src_hk = self
            .project
            .settings
            .hotkeys
            .registry
            .source_hotkeys
            .get(sel_id)
            .cloned()
            .unwrap_or_default();

        // Mute/Unmute
        let mute_str = src_hk.mute.unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label("Mute/Unmute:");
            let btn_text = if mute_str.is_empty() {
                egui::RichText::new("None").color(TEXT_MUTED)
            } else {
                egui::RichText::new(&mute_str).strong()
            };
            if ui.button(btn_text).clicked() {
                *self.rebinding_target = Some(crate::hotkey_registry::RebindTarget::SourceMute(
                    sel_id.to_string(),
                ));
                ui.close_menu();
            }
            if !mute_str.is_empty()
                && IconButton::new(IconKind::X)
                    .ghost(true)
                    .size(vec2(18.0, 18.0))
                    .tooltip("Clear shortcut")
                    .show(ui)
                    .clicked()
            {
                self.project
                    .settings
                    .hotkeys
                    .registry
                    .source_hotkeys
                    .entry(sel_id.to_string())
                    .or_default()
                    .mute = None;
            }
        });

        // Push-to-mute
        let ptm_str = src_hk.push_to_mute.unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label("Push-to-Mute:");
            let btn_text = if ptm_str.is_empty() {
                egui::RichText::new("None").color(TEXT_MUTED)
            } else {
                egui::RichText::new(&ptm_str).strong()
            };
            if ui.button(btn_text).clicked() {
                *self.rebinding_target = Some(
                    crate::hotkey_registry::RebindTarget::SourcePushToMute(sel_id.to_string()),
                );
                ui.close_menu();
            }
            if !ptm_str.is_empty()
                && IconButton::new(IconKind::X)
                    .ghost(true)
                    .size(vec2(18.0, 18.0))
                    .tooltip("Clear shortcut")
                    .show(ui)
                    .clicked()
            {
                self.project
                    .settings
                    .hotkeys
                    .registry
                    .source_hotkeys
                    .entry(sel_id.to_string())
                    .or_default()
                    .push_to_mute = None;
            }
        });

        // Push-to-talk
        let ptt_str = src_hk.push_to_talk.unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label("Push-to-Talk:");
            let btn_text = if ptt_str.is_empty() {
                egui::RichText::new("None").color(TEXT_MUTED)
            } else {
                egui::RichText::new(&ptt_str).strong()
            };
            if ui.button(btn_text).clicked() {
                *self.rebinding_target = Some(
                    crate::hotkey_registry::RebindTarget::SourcePushToTalk(sel_id.to_string()),
                );
                ui.close_menu();
            }
            if !ptt_str.is_empty()
                && IconButton::new(IconKind::X)
                    .ghost(true)
                    .size(vec2(18.0, 18.0))
                    .tooltip("Clear shortcut")
                    .show(ui)
                    .clicked()
            {
                self.project
                    .settings
                    .hotkeys
                    .registry
                    .source_hotkeys
                    .entry(sel_id.to_string())
                    .or_default()
                    .push_to_talk = None;
            }
        });

        // Show/Hide
        let sh_str = src_hk.show_hide.unwrap_or_default();
        ui.horizontal(|ui| {
            ui.label("Show/Hide Toggle:");
            let btn_text = if sh_str.is_empty() {
                egui::RichText::new("None").color(TEXT_MUTED)
            } else {
                egui::RichText::new(&sh_str).strong()
            };
            if ui.button(btn_text).clicked() {
                *self.rebinding_target = Some(
                    crate::hotkey_registry::RebindTarget::SourceShowHide(sel_id.to_string()),
                );
                ui.close_menu();
            }
            if !sh_str.is_empty()
                && IconButton::new(IconKind::X)
                    .ghost(true)
                    .size(vec2(18.0, 18.0))
                    .tooltip("Clear shortcut")
                    .show(ui)
                    .clicked()
            {
                self.project
                    .settings
                    .hotkeys
                    .registry
                    .source_hotkeys
                    .entry(sel_id.to_string())
                    .or_default()
                    .show_hide = None;
            }
        });
    }

    /// Renders context menu on empty canvas or empty list space.
    pub fn render_empty_context_menu_items(&mut self, ui: &mut egui::Ui) {
        ui.label(egui::RichText::new("Canvas").strong());
        ui.separator();

        if ui.button("+ Add Source...").clicked() {
            *self.trigger_open_add_source = true;
            ui.close_menu();
        }

        let can_paste = self.clipboard_source.is_some();
        if ui
            .add_enabled(
                can_paste,
                egui::Button::new("Paste (Reference)\tCtrl+Shift+V"),
            )
            .clicked()
        {
            *self.trigger_paste_reference = true;
            ui.close_menu();
        }
        if ui
            .add_enabled(can_paste, egui::Button::new("Paste (Duplicate)\tCtrl+V"))
            .clicked()
        {
            *self.trigger_paste_duplicate = true;
            ui.close_menu();
        }

        if self.studio_mode {
            ui.separator();
            ui.checkbox(self.preview_locked, "Lock Preview");
        }

        ui.menu_button("Preview Scaling", |ui| {
            if ui
                .selectable_value(
                    self.preview_scaling_mode,
                    crate::project::PreviewScalingMode::ScaleToWindow,
                    "Scale to Window (Fit)",
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .selectable_value(
                    self.preview_scaling_mode,
                    crate::project::PreviewScalingMode::Canvas1To1,
                    "Canvas (1:1)",
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .selectable_value(
                    self.preview_scaling_mode,
                    crate::project::PreviewScalingMode::Output1To1,
                    "Output (1:1)",
                )
                .clicked()
            {
                ui.close_menu();
            }
        });

        ui.menu_button("Projector", |ui| {
            if ui.button("Open Windowed Projector (Preview)").clicked() {
                ui.close_menu();
            }
            if ui.button("Open Fullscreen Projector (Preview)").clicked() {
                ui.close_menu();
            }
        });

        if ui.button("Save Preview Screenshot (PNG)...").clicked() {
            *self.trigger_save_screenshot = Some((None, true));
            ui.close_menu();
        }

        ui.separator();

        if ui
            .add(egui::Button::new("Deselect All").shortcut_text("Escape"))
            .clicked()
        {
            *self.selected_source_id = None;
            ui.close_menu();
        }
    }

    /// Renders the on-canvas right-click context menu for sources and canvas area.
    fn render_canvas_context_menu(&mut self, canvas_response: &egui::Response) {
        let canvas_w = (self.project.settings.video.base_width as f32).max(1.0);
        let canvas_h = (self.project.settings.video.base_height as f32).max(1.0);

        canvas_response.context_menu(|ui| {
            if let Some(ref sel_id) = *self.selected_source_id {
                let sel = sel_id.clone();
                self.render_source_context_menu_items(ui, &sel, canvas_w, canvas_h);
            } else {
                self.render_empty_context_menu_items(ui);
            }
        });
    }

    /// Renders single Program monitor in Normal Mode (OBS default).
    fn ui_normal_mode_monitor(&mut self, ui: &mut Ui) {
        let active_name = self
            .project
            .active_scene_id
            .clone()
            .unwrap_or_else(|| "None".to_string());

        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                if self.is_streaming || self.is_recording {
                    ui.colored_label(RED_LIVE, format!("{} - ON AIR", active_name));
                } else {
                    ui.colored_label(TEXT, &active_name);
                }
            });

            let avail_w = ui.available_width().max(160.0);
            let avail_h = (ui.available_height() - 28.0).max(90.0);
            let (monitor_w, monitor_h) = if avail_w * 9.0 / 16.0 <= avail_h {
                (avail_w, avail_w * 9.0 / 16.0)
            } else {
                (avail_h * 16.0 / 9.0, avail_h)
            };

            let x_pad = ((avail_w - monitor_w) * 0.5).max(0.0);
            ui.horizontal(|ui| {
                if x_pad > 0.0 {
                    ui.add_space(x_pad);
                }
                ui.vertical(|ui| {
                    let (rect, _response) =
                        ui.allocate_exact_size(vec2(monitor_w, monitor_h), Sense::hover());
                    let border_color = if self.is_streaming || self.is_recording {
                        RED_LIVE
                    } else {
                        ACCENT
                    };
                    crate::theme::paint_checkerboard(ui.painter(), rect, 16.0);
                    if let Some(tex_id) = self.program_texture_id {
                        ui.painter().image(
                            tex_id,
                            rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            crate::theme::TINT_NONE,
                        );
                    } else {
                        ui.painter().text(
                            rect.center(),
                            Align2::CENTER_CENTER,
                            &active_name,
                            FontId::proportional(20.0),
                            TEXT_MUTED,
                        );
                    }

                    // Interactive on-canvas Transform Gizmo for Program Monitor
                    let mut gizmo_items = Vec::new();
                    for src in self.project.editable_sources(self.studio_mode) {
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

                    let selected_ids = if let Some(ref sel_id) = *self.selected_source_id {
                        vec![sel_id.clone()]
                    } else {
                        Vec::new()
                    };

                    let canvas_w = (self.project.settings.video.base_width as f32).max(1.0);
                    let canvas_h = (self.project.settings.video.base_height as f32).max(1.0);
                    let canvas_size = vec2(canvas_w, canvas_h);
                    let zoom = rect.width() / canvas_size.x;

                    let (canvas_response, events) =
                        self.live_gizmo_state.draw_and_interact_with_response(
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
                            TransformGizmoEvent::DragStarted => {
                                *self.trigger_snapshot_undo = true;
                            }
                            TransformGizmoEvent::Transformed {
                                id,
                                center_x,
                                center_y,
                                width,
                                height,
                                rotation_degrees,
                            } => {
                                if let Some(sources) =
                                    self.project.editable_sources_mut(self.studio_mode)
                                {
                                    if let Some(src) = sources.iter_mut().find(|s| s.id == id) {
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

                    self.render_canvas_context_menu(&canvas_response);

                    ui.painter().rect_stroke(
                        rect,
                        CornerRadius::same(CORNER_RADIUS as u8),
                        Stroke::new(BORDER_WIDTH, border_color),
                        StrokeKind::Inside,
                    );
                });
            });
        });
    }

    /// Renders Scenes list with Add, Duplicate, Remove, and Switch operations.
    fn ui_scenes(&mut self, ui: &mut Ui) {
        if !self.studio_mode {
            if self.selected_scene_id.as_ref() != self.project.active_scene_id.as_ref()
                && self.project.active_scene_id.is_some()
            {
                *self.selected_scene_id = self.project.active_scene_id.clone();
                self.project.preview_scene_id = self.project.active_scene_id.clone();
            }
        } else {
            // Ensure selected_scene_id always tracks project.preview_scene_id in Studio mode
            if self.selected_scene_id.as_ref() != self.project.preview_scene_id.as_ref() {
                if self.project.preview_scene_id.is_some() {
                    *self.selected_scene_id = self.project.preview_scene_id.clone();
                } else if self.selected_scene_id.is_some() {
                    self.project.preview_scene_id = self.selected_scene_id.clone();
                }
            }
        }

        ui.horizontal(|ui| {
            if icon_button(ui, IconKind::Plus, "Add Scene").clicked() {
                *self.trigger_add_scene = true;
            }

            let can_remove = self.project.scenes.len() > 1 && self.selected_scene_id.is_some();
            if icon_button_enabled(ui, can_remove, IconKind::Trash, "Remove Scene").clicked() {
                if let Some(ref sel) = *self.selected_scene_id {
                    *self.trigger_remove_scene = Some(sel.clone());
                }
            }

            let has_sel = self.selected_scene_id.is_some();
            if icon_button_enabled(ui, has_sel, IconKind::Pencil, "Rename Scene").clicked() {
                if let Some(ref sel) = *self.selected_scene_id {
                    if let Some(sc) = self.project.scenes.iter().find(|s| &s.id == sel) {
                        *self.rename_scene_id = Some(sel.clone());
                        *self.rename_scene_buffer = sc.name.clone();
                    }
                }
            }

            let sel_idx = self
                .selected_scene_id
                .as_ref()
                .and_then(|sel| self.project.scenes.iter().position(|s| &s.id == sel));
            let can_up = sel_idx.is_some_and(|i| i > 0);
            let can_down = sel_idx.is_some_and(|i| i + 1 < self.project.scenes.len());

            if icon_button_enabled(ui, can_up, IconKind::ChevronUp, "Move Scene Up").clicked() {
                if let Some(ref sel) = *self.selected_scene_id {
                    *self.trigger_move_scene_up = Some(sel.clone());
                }
            }
            if icon_button_enabled(ui, can_down, IconKind::ChevronDown, "Move Scene Down").clicked()
            {
                if let Some(ref sel) = *self.selected_scene_id {
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
            let mut scene_clicked: Option<(String, Vec<crate::project::SourceDefinition>)> = None;
            let mut scene_double_clicked: Option<String> = None;
            let mut clear_scene_switch_to: Option<String> = None;
            let mut clear_scene_switch_back: Option<String> = None;

            if self.project.scenes.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(24.0);
                    ui.colored_label(TEXT_MUTED, "No scenes yet Ã¢â‚¬â€ click + Add Scene");
                });
            } else {
                for scene in &self.project.scenes {
                    let is_selected = self.selected_scene_id.as_deref() == Some(&scene.id);
                    let is_active = self.project.active_scene_id.as_deref() == Some(&scene.id);
                    let is_preview = self.project.preview_scene_id.as_deref() == Some(&scene.id);

                    let label_text = if self.studio_mode {
                        format!(
                            "{} ({}){}{}",
                            scene.name,
                            scene.sources.len(),
                            if is_active { " [LIVE]" } else { "" },
                            if is_preview { " [PRV]" } else { "" }
                        )
                    } else {
                        format!(
                            "{} ({}){}",
                            scene.name,
                            scene.sources.len(),
                            if is_active { " [LIVE]" } else { "" }
                        )
                    };

                    let row_w = ui.available_width();
                    let (row_rect, response) =
                        ui.allocate_exact_size(egui::vec2(row_w, ROW_HEIGHT_LIST), Sense::click());

                    let is_hovered = response.hovered();
                    let bg_color = if is_selected {
                        ACCENT
                    } else if is_hovered {
                        SURFACE_2
                    } else {
                        TRANSPARENT
                    };

                    if bg_color != TRANSPARENT {
                        ui.painter().rect_filled(
                            row_rect,
                            CornerRadius::same(CORNER_RADIUS as u8),
                            bg_color,
                        );
                    }

                    let text_color = if is_selected {
                        TEXT
                    } else if is_active {
                        ACCENT
                    } else {
                        TEXT
                    };

                    let font_id = egui::TextStyle::Body.resolve(ui.style());
                    let text_pos = egui::pos2(row_rect.min.x + 8.0, row_rect.center().y);
                    ui.painter().text(
                        text_pos,
                        egui::Align2::LEFT_CENTER,
                        &label_text,
                        font_id,
                        text_color,
                    );
                    if response.clicked() {
                        if self.studio_mode {
                            scene_clicked = Some((scene.id.clone(), scene.sources.clone()));
                        } else {
                            scene_double_clicked = Some(scene.id.clone());
                        }
                    }
                    if response.double_clicked() {
                        scene_double_clicked = Some(scene.id.clone());
                    }

                    let scene_id = scene.id.clone();
                    let scene_name = scene.name.clone();
                    let sc_hk = self
                        .project
                        .settings
                        .hotkeys
                        .registry
                        .scene_hotkeys
                        .get(&scene_id)
                        .cloned()
                        .unwrap_or_default();

                    response.context_menu(|ui| {
                        if ui.button("Switch to Scene").clicked() {
                            scene_double_clicked = Some(scene_id.clone());
                            ui.close_menu();
                        }
                        if ui.button("Rename (F2)").clicked() {
                            *self.rename_scene_id = Some(scene_id.clone());
                            *self.rename_scene_buffer = scene_name.clone();
                            ui.close_menu();
                        }
                        ui.separator();
                        ui.label(egui::RichText::new("Scene Hotkeys (OBS Parity)").strong());

                        // Switch To
                        let switch_to_str = sc_hk.switch_to.unwrap_or_default();
                        ui.horizontal(|ui| {
                            ui.label("Switch To:");
                            let btn_text = if switch_to_str.is_empty() {
                                egui::RichText::new("None").color(TEXT_MUTED)
                            } else {
                                egui::RichText::new(&switch_to_str).color(TEXT)
                            };
                            if ui.button(btn_text).clicked() {
                                *self.rebinding_target =
                                    Some(crate::hotkey_registry::RebindTarget::SceneSwitch(
                                        scene_id.clone(),
                                    ));
                                ui.close_menu();
                            }
                            if !switch_to_str.is_empty()
                                && IconButton::new(IconKind::X)
                                    .ghost(true)
                                    .size(vec2(18.0, 18.0))
                                    .tooltip("Clear shortcut")
                                    .show(ui)
                                    .clicked()
                            {
                                clear_scene_switch_to = Some(scene_id.clone());
                            }
                        });

                        // Switch Back
                        let switch_back_str = sc_hk.switch_back.unwrap_or_default();
                        ui.horizontal(|ui| {
                            ui.label("Switch Back:");
                            let btn_text = if switch_back_str.is_empty() {
                                egui::RichText::new("None").color(TEXT_MUTED)
                            } else {
                                egui::RichText::new(&switch_back_str).color(TEXT)
                            };
                            if ui.button(btn_text).clicked() {
                                *self.rebinding_target =
                                    Some(crate::hotkey_registry::RebindTarget::SceneSwitchBack(
                                        scene_id.clone(),
                                    ));
                                ui.close_menu();
                            }
                            if !switch_back_str.is_empty()
                                && IconButton::new(IconKind::X)
                                    .ghost(true)
                                    .size(vec2(18.0, 18.0))
                                    .tooltip("Clear shortcut")
                                    .show(ui)
                                    .clicked()
                            {
                                clear_scene_switch_back = Some(scene_id.clone());
                            }
                        });
                    });
                }
            }

            if let Some((sc_id, sc_sources)) = scene_clicked {
                self.project.commit_preview_working_sources();
                *self.selected_scene_id = Some(sc_id.clone());
                self.project.preview_scene_id = Some(sc_id.clone());
                self.project.preview_working_sources = Some((sc_id, sc_sources));
            }
            if let Some(sc_id) = scene_double_clicked {
                *self.trigger_switch_scene_direct = Some(sc_id);
            }
            if let Some(sc_id) = clear_scene_switch_to {
                self.project
                    .settings
                    .hotkeys
                    .registry
                    .scene_hotkeys
                    .entry(sc_id)
                    .or_default()
                    .switch_to = None;
            }
            if let Some(sc_id) = clear_scene_switch_back {
                self.project
                    .settings
                    .hotkeys
                    .registry
                    .scene_hotkeys
                    .entry(sc_id)
                    .or_default()
                    .switch_back = None;
            }
        });
    }

    /// Renders Sources list for the currently selected scene.
    fn ui_sources(&mut self, ui: &mut Ui) {
        if self.project.preview_scene_id.is_none() {
            self.project.preview_scene_id = self
                .selected_scene_id
                .clone()
                .or_else(|| self.project.scenes.first().map(|s| s.id.clone()));
        }
        let sc_id_opt = self.project.editable_scene_id(self.studio_mode);
        let Some(sc_id) = sc_id_opt else {
            ui.colored_label(TEXT_MUTED, "Select a scene to manage sources");
            return;
        };

        let sources = self.project.editable_sources(self.studio_mode);
        let num_sources = sources.len();
        let sel_idx = self
            .selected_source_id
            .as_ref()
            .and_then(|id| sources.iter().position(|s| &s.id == id));

        ui.horizontal(|ui| {
            if icon_button(ui, IconKind::Plus, "Add Source").clicked() {
                *self.trigger_open_add_source = true;
            }

            let has_sel = self.selected_source_id.is_some();
            let is_sel_locked = sel_idx.map(|i| sources[i].locked).unwrap_or(false);
            if icon_button_enabled(
                ui,
                has_sel,
                IconKind::Trash,
                if is_sel_locked {
                    "Unlock to delete (Ctrl+Shift+L)"
                } else {
                    "Remove Source (Delete)"
                },
            )
            .clicked()
            {
                if let Some(ref sel_id) = *self.selected_source_id {
                    if is_sel_locked {
                        *self.source_warning_message =
                            Some(("Unlock to delete".to_string(), std::time::Instant::now()));
                    } else {
                        *self.trigger_remove_source = Some(sel_id.clone());
                    }
                }
            }

            if icon_button_enabled(
                ui,
                has_sel,
                IconKind::Sliders,
                "Configure source properties",
            )
            .clicked()
            {
                if let Some(ref sel_id) = *self.selected_source_id {
                    *self.trigger_open_edit_source = Some(sel_id.clone());
                }
            }

            if icon_button_enabled(
                ui,
                has_sel,
                IconKind::Filter,
                "Filters... (Configure Video/Audio filter chain)",
            )
            .clicked()
            {
                if let Some(ref sel_id) = *self.selected_source_id {
                    *self.trigger_open_filters = Some(sel_id.clone());
                }
            }

            ui.separator();

            let can_bring_forward = sel_idx.is_some_and(|i| i + 1 < num_sources);
            let can_send_backward = sel_idx.is_some_and(|i| i > 0);

            if icon_button_enabled(
                ui,
                can_bring_forward,
                IconKind::ChevronUp,
                "Bring Forward (Ctrl+])",
            )
            .clicked()
            {
                if let Some(i) = sel_idx {
                    if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                        if i + 1 < srcs.len() {
                            srcs.swap(i, i + 1);
                            *self.trigger_sync_source_order = Some(sc_id.clone());
                        }
                    }
                }
            }

            if icon_button_enabled(
                ui,
                can_send_backward,
                IconKind::ChevronDown,
                "Send Backward (Ctrl+[)",
            )
            .clicked()
            {
                if let Some(i) = sel_idx {
                    if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                        if i > 0 && i < srcs.len() {
                            srcs.swap(i, i - 1);
                            *self.trigger_sync_source_order = Some(sc_id.clone());
                        }
                    }
                }
            }

            if let Some((msg, time)) = self.source_warning_message.as_ref() {
                if time.elapsed().as_secs_f32() < 3.0 {
                    ui.separator();
                    ui.colored_label(AMBER_WARN, msg);
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

        // Keyboard shortcuts for layer reordering
        let (key_front, key_fwd, key_bwd, key_back) = ui.input(|i| {
            let reg = &self.project.settings.hotkeys.registry;
            (
                reg.action_triggered(crate::hotkey_registry::ActionId::BringToFront, i),
                reg.action_triggered(crate::hotkey_registry::ActionId::BringForward, i),
                reg.action_triggered(crate::hotkey_registry::ActionId::SendBackward, i),
                reg.action_triggered(crate::hotkey_registry::ActionId::SendToBack, i),
            )
        });

        if let Some(i) = sel_idx {
            if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                if key_front && i + 1 < srcs.len() {
                    let s = srcs.remove(i);
                    srcs.push(s);
                    *self.trigger_sync_source_order = Some(sc_id.clone());
                } else if key_fwd && i + 1 < srcs.len() {
                    srcs.swap(i, i + 1);
                    *self.trigger_sync_source_order = Some(sc_id.clone());
                } else if key_bwd && i > 0 && i < srcs.len() {
                    srcs.swap(i, i - 1);
                    *self.trigger_sync_source_order = Some(sc_id.clone());
                } else if key_back && i > 0 && i < srcs.len() {
                    let s = srcs.remove(i);
                    srcs.insert(0, s);
                    *self.trigger_sync_source_order = Some(sc_id.clone());
                }
            }
        }

        if sources.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.colored_label(
                    TEXT_MUTED,
                    "No sources yet Ã¢â‚¬â€ click + Add Source or drag media here",
                );
            });
        }

        let dnd_source_id_key = egui::Id::new("dnd_dragging_source_id");
        let dragging_source_id: Option<String> = ui.ctx().data(|d| d.get_temp(dnd_source_id_key));

        let mut row_rects: Vec<egui::Rect> = Vec::with_capacity(num_sources);

        let scroll_output = egui::ScrollArea::vertical().show(ui, |ui| {
            let list_width = ui.available_width();
            for display_idx in 0..num_sources {
                let src_idx = num_sources - 1 - display_idx;
                let src_id = sources[src_idx].id.clone();
                let src_name = sources[src_idx].name.clone();
                let src_visible = sources[src_idx].visible;
                let src_locked = sources[src_idx].locked;
                let is_sel = self.selected_source_id.as_deref() == Some(&src_id);
                let is_dragged = dragging_source_id.as_deref() == Some(&src_id);

                let (row_rect, _) =
                    ui.allocate_exact_size(egui::vec2(list_width, ROW_HEIGHT_LIST), Sense::hover());
                let is_hovered = ui.rect_contains_pointer(row_rect);

                let bg_color = if is_dragged {
                    SURFACE_2
                } else if is_sel {
                    if src_visible {
                        ACCENT
                    } else {
                        ACCENT.linear_multiply(0.7)
                    }
                } else if is_hovered {
                    SURFACE_2
                } else {
                    TRANSPARENT
                };

                if bg_color != TRANSPARENT {
                    ui.painter().rect_filled(
                        row_rect,
                        CornerRadius::same(CORNER_RADIUS as u8),
                        bg_color,
                    );
                }

                let mut grip_drag = false;
                let mut child_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(row_rect)
                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                );
                child_ui.add_space(4.0);

                // Text and icon color (dimmed when hidden, matching OBS)
                let text_color = if is_sel || src_visible {
                    TEXT
                } else {
                    TEXT_MUTED
                };

                // Drag handle grip
                let grip_resp = grip_handle_colored(&mut child_ui, text_color, "Drag to reorder");
                if grip_resp.hovered() {
                    child_ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
                if grip_resp.drag_started() {
                    grip_drag = true;
                }

                // Visibility toggle
                let eye_resp = icon_toggle_button_colored(
                    &mut child_ui,
                    text_color,
                    src_visible,
                    IconKind::Eye,
                    IconKind::EyeOff,
                    if src_visible {
                        "Hide Source (Ctrl+Shift+H)"
                    } else {
                        "Show Source (Ctrl+Shift+H)"
                    },
                );
                if eye_resp.clicked() {
                    *self.trigger_toggle_source_visibility = Some(src_id.clone());
                    *self.selected_source_id = Some(src_id.clone());
                }

                // Lock toggle
                let lock_resp = icon_toggle_button_colored(
                    &mut child_ui,
                    text_color,
                    src_locked,
                    IconKind::Lock,
                    IconKind::Unlock,
                    if src_locked {
                        "Unlock Source (Ctrl+Shift+L)"
                    } else {
                        "Lock Source (Ctrl+Shift+L)"
                    },
                );
                if lock_resp.clicked() {
                    *self.trigger_toggle_source_lock = Some(src_id.clone());
                    *self.selected_source_id = Some(src_id.clone());
                }

                child_ui.add(
                    egui::Label::new(egui::RichText::new(&src_name).color(text_color)).truncate(),
                );

                row_rects.push(row_rect);

                // Row interaction - starts after grip (16), eye (22), lock (22) + spacing (~76px)
                // so clicks on buttons are handled cleanly by the buttons themselves
                let label_interact_rect = egui::Rect::from_min_max(
                    egui::pos2((row_rect.min.x + 76.0).min(row_rect.max.x), row_rect.min.y),
                    row_rect.max,
                );
                let row_sense = ui.interact(
                    label_interact_rect,
                    egui::Id::new("source_row_interact").with(&src_id),
                    Sense::click_and_drag(),
                );

                if grip_drag || row_sense.drag_started() {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(dnd_source_id_key, src_id.clone()));
                }

                if row_sense.clicked() {
                    *self.selected_source_id = Some(src_id.clone());
                }
                if row_sense.double_clicked() {
                    *self.trigger_open_edit_source = Some(src_id.clone());
                }

                // Context menu with full OBS parity
                row_sense.context_menu(|ui| {
                    let canvas_w = (self.project.settings.video.base_width as f32).max(1.0);
                    let canvas_h = (self.project.settings.video.base_height as f32).max(1.0);
                    self.render_source_context_menu_items(ui, &src_id, canvas_w, canvas_h);
                });
            }
        });

        if let Some(ref dragged_id) = dragging_source_id {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);

            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let dragged_name = sources
                    .iter()
                    .find(|s| &s.id == dragged_id)
                    .map(|s| s.name.as_str())
                    .unwrap_or("Source");
                egui::show_tooltip(
                    ui.ctx(),
                    ui.layer_id(),
                    egui::Id::new("src_drag_tooltip"),
                    |ui| {
                        ui.horizontal(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(vec2(16.0, 16.0), Sense::hover());
                            paint_lucide_icon(
                                ui.painter(),
                                IconKind::GripVertical,
                                rect,
                                Stroke::new(1.5_f32, TEXT),
                            );
                            ui.label(dragged_name);
                        });
                    },
                );

                if !row_rects.is_empty() {
                    let mut slot_y = Vec::with_capacity(num_sources + 1);
                    slot_y.push(row_rects[0].top());
                    for i in 1..num_sources {
                        slot_y.push((row_rects[i - 1].bottom() + row_rects[i].top()) * 0.5);
                    }
                    slot_y.push(row_rects[num_sources - 1].bottom());

                    let target_slot = (0..=num_sources)
                        .min_by(|&a, &b| {
                            let dist_a = (slot_y[a] - pointer_pos.y).abs();
                            let dist_b = (slot_y[b] - pointer_pos.y).abs();
                            dist_a
                                .partial_cmp(&dist_b)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        })
                        .unwrap_or(0);

                    let line_y = slot_y[target_slot];
                    let min_x = scroll_output.inner_rect.left() + 4.0;
                    let max_x = scroll_output.inner_rect.right() - 4.0;
                    ui.painter()
                        .hline(min_x..=max_x, line_y, Stroke::new(2.5_f32, ACCENT));
                    ui.painter()
                        .circle_filled(egui::pos2(min_x, line_y), 3.5_f32, ACCENT);
                    ui.painter()
                        .circle_filled(egui::pos2(max_x, line_y), 3.5_f32, ACCENT);

                    if ui.input(|i| i.pointer.any_released()) {
                        let from_row = (0..num_sources).find(|&r| {
                            let s_idx = num_sources - 1 - r;
                            &sources[s_idx].id == dragged_id
                        });
                        if let Some(from_r) = from_row {
                            if target_slot != from_r && target_slot != from_r + 1 {
                                let new_row = if target_slot < from_r {
                                    target_slot
                                } else {
                                    target_slot - 1
                                };
                                let from_idx = num_sources - 1 - from_r;
                                let to_idx = num_sources - 1 - new_row;
                                if let Some(srcs) =
                                    self.project.editable_sources_mut(self.studio_mode)
                                {
                                    if from_idx < srcs.len() && to_idx < srcs.len() {
                                        let item = srcs.remove(from_idx);
                                        srcs.insert(to_idx, item);
                                        *self.trigger_sync_source_order = Some(sc_id.clone());
                                    }
                                }
                            }
                        }
                        ui.ctx()
                            .data_mut(|d| d.remove_temp::<String>(dnd_source_id_key));
                    }
                }
            } else if ui.input(|i| i.pointer.any_released()) {
                ui.ctx()
                    .data_mut(|d| d.remove_temp::<String>(dnd_source_id_key));
            }
        }

        let mut transform_rect = egui::Rect::NOTHING;
        if let Some(ref sel_id) = *self.selected_source_id {
            if let Some(srcs) = self.project.editable_sources_mut(self.studio_mode) {
                if let Some(src) = srcs.iter_mut().find(|s| &s.id == sel_id) {
                    ui.separator();
                    let collapsing_res = egui::CollapsingHeader::new("Transform")
                        .default_open(true)
                        .id_salt(format!("src_transform_{}", sel_id))
                        .show(ui, |ui| {
                            let mut changed = false;
                            egui::Grid::new("src_transform_grid")
                                .num_columns(4)
                                .spacing(vec2(GRID_UNIT, GRID_UNIT))
                                .show(ui, |ui| {
                                    ui.label("X:");
                                    let rx = ui.add(egui::DragValue::new(&mut src.x).speed(1.0));
                                    if rx.drag_started() {
                                        *self.trigger_snapshot_undo = true;
                                    }
                                    changed |= rx.changed();

                                    ui.label("Y:");
                                    let ry = ui.add(egui::DragValue::new(&mut src.y).speed(1.0));
                                    if ry.drag_started() {
                                        *self.trigger_snapshot_undo = true;
                                    }
                                    changed |= ry.changed();
                                    ui.end_row();

                                    ui.label("Width:");
                                    let rw = ui.add(
                                        egui::DragValue::new(&mut src.width)
                                            .speed(1.0)
                                            .range(1.0..=7680.0),
                                    );
                                    if rw.drag_started() {
                                        *self.trigger_snapshot_undo = true;
                                    }
                                    changed |= rw.changed();

                                    ui.label("Height:");
                                    let rh = ui.add(
                                        egui::DragValue::new(&mut src.height)
                                            .speed(1.0)
                                            .range(1.0..=4320.0),
                                    );
                                    if rh.drag_started() {
                                        *self.trigger_snapshot_undo = true;
                                    }
                                    changed |= rh.changed();
                                    ui.end_row();
                                });

                            ui.add_space(GRID_UNIT * 0.5);
                            ui.horizontal(|ui| {
                                ui.label("Rotation:");
                                let rr = ui.add(
                                    egui::DragValue::new(&mut src.rotation)
                                        .speed(0.5)
                                        .suffix("°"),
                                );
                                if rr.drag_started() {
                                    *self.trigger_snapshot_undo = true;
                                }
                                changed |= rr.changed();
                                if icon_button(ui, IconKind::RotateCcw, "Reset Rotation to 0 deg")
                                    .clicked()
                                {
                                    *self.trigger_snapshot_undo = true;
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

                    transform_rect = collapsing_res.header_response.rect;
                    if let Some(ref body_resp) = collapsing_res.body_response {
                        transform_rect = transform_rect.union(body_resp.rect);
                    }
                }
            }
        }

        // Section 2.2: Clicking empty space below source rows deselects (guarding transform controls)
        if scroll_output.inner_rect.is_positive() && ui.input(|i| i.pointer.primary_clicked()) {
            if let Some(pos) = ui.input(|i| i.pointer.interact_pos()) {
                let on_row = row_rects.iter().any(|r| r.contains(pos));
                let on_transform = transform_rect.is_positive() && transform_rect.contains(pos);
                let inside_empty_space = (scroll_output.inner_rect.contains(pos)
                    || (transform_rect.is_positive()
                        && pos.y > transform_rect.max.y
                        && pos.x >= scroll_output.inner_rect.min.x
                        && pos.x <= scroll_output.inner_rect.max.x))
                    && !on_row
                    && !on_transform;
                if inside_empty_space {
                    *self.selected_source_id = None;
                }
            }
        }
    }

    /// Converts linear volume [0.0..] to dBFS decibels.
    pub fn volume_to_db(vol: f32) -> f32 {
        if vol <= 0.0001 {
            -60.0
        } else {
            20.0 * vol.log10()
        }
    }

    /// Converts dBFS decibels [-60.0..] to linear volume.
    pub fn db_to_volume(db: f32) -> f32 {
        if db <= -59.9 {
            0.0
        } else {
            10.0_f32.powf(db / 20.0)
        }
    }

    /// Helper to render a single channel column in the audio mixer.
    fn render_audio_channel(
        ui: &mut Ui,
        name: &str,
        volume: &mut f32,
        muted: &mut bool,
        trigger_audio_change: &mut bool,
        tooltip: &str,
        channel_width: f32,
    ) {
        let mute_btn_width = (channel_width - 16.0).clamp(44.0, 72.0);
        ui.allocate_ui_with_layout(
            vec2(channel_width, ui.available_height()),
            egui::Layout::top_down(egui::Align::Center),
            |ui| {
                let name_label = egui::Label::new(egui::RichText::new(name).strong()).truncate();
                let lbl_resp = ui.add(name_label);
                crate::icon::show_aligned_tooltip(ui, &lbl_resp, tooltip);
                ui.add_space(GRID_UNIT * 0.5);

                let mut db = Self::volume_to_db(*volume);
                let vol_resp = ui.add(
                    egui::Slider::new(&mut db, -60.0..=6.0)
                        .vertical()
                        .show_value(false),
                );
                if vol_resp.changed() {
                    *volume = Self::db_to_volume(db);
                    *trigger_audio_change = true;
                }

                let db_text = if *volume <= 0.0001 || *muted {
                    "-inf dB".to_string()
                } else {
                    format!("{:.1} dB", Self::volume_to_db(*volume))
                };
                ui.add_space(GRID_UNIT * 0.5);
                ui.label(
                    egui::RichText::new(db_text)
                        .font(egui::FontId::monospace(11.0))
                        .color(TEXT_MUTED),
                );
                ui.add_space(GRID_UNIT * 0.5);

                let mute_btn = Button::new("Mute")
                    .secondary()
                    .selected(*muted)
                    .width(mute_btn_width)
                    .show(ui);
                if mute_btn.clicked() {
                    *muted = !*muted;
                    *trigger_audio_change = true;
                }
            },
        );
    }

    /// Renders 6-track audio mixer sliders, mute buttons, and live meters.
    fn ui_audio_mixer(&mut self, ui: &mut Ui) {
        const MIN_CHANNEL_WIDTH: f32 = 56.0;
        const MAX_CHANNEL_WIDTH: f32 = 220.0;

        let active_scene = self
            .project
            .active_scene_id
            .as_ref()
            .and_then(|id| self.project.scenes.iter_mut().find(|s| &s.id == id));

        let num_sources = active_scene.as_ref().map(|s| s.sources.len()).unwrap_or(0);
        let total_channels = 1 + num_sources;

        let avail_w = ui.available_width();
        let sep_width =
            (ui.visuals().window_stroke.width + ui.spacing().item_spacing.x * 2.0).max(12.0);
        let total_sep_width = total_channels.saturating_sub(1) as f32 * sep_width;
        let min_required_width = total_channels as f32 * MIN_CHANNEL_WIDTH + total_sep_width;

        // If all channels fit, do NOT invoke ScrollArea at all so no scrollbar can ever appear.
        let needs_scroll = min_required_width > avail_w;
        let remaining_w = (avail_w - total_sep_width).max(0.0);
        let channel_width = if needs_scroll {
            MIN_CHANNEL_WIDTH
        } else {
            ((remaining_w - 1.0) / total_channels as f32)
                .clamp(MIN_CHANNEL_WIDTH, MAX_CHANNEL_WIDTH)
        };

        let master_volume = &mut *self.master_volume;
        let master_muted = &mut *self.master_muted;
        let trigger_audio_change = &mut *self.trigger_audio_change;

        let render_tracks = |ui: &mut Ui| {
            // Master Track
            Self::render_audio_channel(
                ui,
                "Master",
                master_volume,
                master_muted,
                trigger_audio_change,
                "Master Output",
                channel_width,
            );

            // Source Tracks
            if let Some(scene) = active_scene {
                if scene.sources.is_empty() {
                    ui.separator();
                    ui.vertical_centered(|ui| {
                        ui.add_space(24.0);
                        ui.colored_label(TEXT_MUTED, "No audio sources in active scene");
                    });
                } else {
                    for src in &mut scene.sources {
                        ui.separator();
                        Self::render_audio_channel(
                            ui,
                            &src.name,
                            &mut src.volume,
                            &mut src.muted,
                            trigger_audio_change,
                            &src.name,
                            channel_width,
                        );
                    }
                }
            } else {
                ui.separator();
                ui.vertical_centered(|ui| {
                    ui.add_space(24.0);
                    ui.colored_label(TEXT_MUTED, "No active scene selected");
                });
            }
        };

        if needs_scroll {
            egui::ScrollArea::horizontal()
                .auto_shrink([false, false])
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        render_tracks(ui);
                    });
                });
        } else {
            ui.horizontal(|ui| {
                render_tracks(ui);
            });
        }
    }

    /// Renders live transition controls (kind, duration, trigger).
    fn ui_transitions(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
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
                if Button::new("Trigger Transition (Auto)")
                    .secondary()
                    .full_width()
                    .show(ui)
                    .clicked()
                {
                    *self.trigger_auto = true;
                }
            });
        });
    }

    /// Renders streaming and recording toggles and real-time broadcast statistics.
    fn ui_stream_controls(&mut self, ui: &mut Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            ui.vertical(|ui| {
                // Stream button: Primary action when idle, Danger when streaming
                let stream_text = if self.is_streaming {
                    "STOP STREAMING"
                } else {
                    "START STREAMING"
                };
                let stream_btn = if self.is_streaming {
                    Button::new(stream_text).danger().full_width().show(ui)
                } else {
                    Button::new(stream_text).primary().full_width().show(ui)
                };
                if stream_btn.clicked() {
                    *self.trigger_toggle_stream = true;
                }

                ui.add_space(GRID_UNIT);

                // Record button: Secondary action when idle, Danger when recording
                let record_text = if self.is_recording {
                    "STOP RECORDING"
                } else {
                    "START RECORDING"
                };
                let record_btn = if self.is_recording {
                    Button::new(record_text).danger().full_width().show(ui)
                } else {
                    Button::new(record_text).secondary().full_width().show(ui)
                };
                if record_btn.clicked() {
                    *self.trigger_toggle_record = true;
                }

                ui.add_space(GRID_UNIT);

                // Studio Mode toggle button (OBS Parity)
                let studio_mode_text = if self.studio_mode {
                    "Studio Mode (Active)"
                } else {
                    "Studio Mode"
                };
                let studio_mode_btn = Button::new(studio_mode_text)
                    .secondary()
                    .selected(self.studio_mode)
                    .full_width()
                    .tooltip(
                        "Toggle between Normal Mode (single monitor) and Studio Mode (Preview / Program split)",
                    )
                    .show(ui);
                if studio_mode_btn.clicked() {
                    *self.trigger_toggle_studio_mode = true;
                }

                ui.add_space(GRID_UNIT);
                ui.separator();
                ui.add_space(GRID_UNIT);

                // Real-Time Output Statistics with fixed-width label column and tabular monospace numerals
                let mut render_stat_row = |label: &str, val: String, val_color: egui::Color32| {
                    ui.horizontal(|ui| {
                        ui.allocate_ui_with_layout(
                            vec2(72.0, 20.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.colored_label(TEXT_MUTED, label);
                            },
                        );
                        ui.label(egui::RichText::new(val).color(val_color).monospace());
                    });
                };

                let (status_text, status_color) = if self.is_streaming {
                    ("LIVE".to_string(), RED_LIVE)
                } else {
                    ("OFFLINE".to_string(), TEXT)
                };
                render_stat_row("Status:", status_text, status_color);

                render_stat_row("FPS:", format!("{:.1}", self.fps), TEXT);

                let drop_color = if self.dropped_percent > 0.1 {
                    AMBER_WARN
                } else {
                    TEXT
                };
                render_stat_row("Dropped:", format!("{:.2}%", self.dropped_percent), drop_color);

                render_stat_row("Bitrate:", format!("{} kbps", self.bitrate_kbps), TEXT);

                render_stat_row("Duration:", self.stream_time.clone(), TEXT);
            });
        });
    }
}

/// Converts linear volume [0.0..] to dBFS decibels.
pub fn volume_to_db(vol: f32) -> f32 {
    if vol <= 0.0001 {
        -60.0
    } else {
        20.0 * vol.log10()
    }
}

/// Converts dBFS decibels [-60.0..] to linear volume.
pub fn db_to_volume(db: f32) -> f32 {
    if db <= -59.9 {
        0.0
    } else {
        10.0_f32.powf(db / 20.0)
    }
}
