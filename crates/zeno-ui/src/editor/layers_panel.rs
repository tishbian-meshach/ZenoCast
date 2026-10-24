//! Layers panel: hierarchy list, add/remove, reordering, visibility and lock toggling.

use egui::{Button, CornerRadius, Sense, Stroke, Ui};
use zeno_core::{Color, LayerId, Vec2};

use crate::editor::state::OverlayEditor;
use crate::theme::{ACCENT, GRID_UNIT, SURFACE_1, SURFACE_2, SURFACE_3, TEXT, TEXT_MUTED};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LayerOrderAction {
    BringToFront,
    BringForward,
    SendBackward,
    SendToBack,
}

/// Renders the layer hierarchy panel.
pub fn ui_layers_panel(editor: &mut OverlayEditor, ui: &mut Ui) {
    ui.vertical(|ui| {
        let layer_count = editor.composition.layers.len();
        let sel_idx = editor
            .selected_layer_id
            .and_then(|id| editor.composition.layers.iter().position(|l| l.id == id));

        // Toolbar: Add Layer, Delete Layer, Up, Down
        ui.horizontal_wrapped(|ui| {
            if ui.button("+ Text").clicked() {
                let name = format!("Text Layer {}", layer_count + 1);
                editor.add_text_layer(&name, "New Text");
            }

            if ui.button("+ Shape").clicked() {
                let name = format!("Shape Layer {}", layer_count + 1);
                editor.add_shape_layer(
                    &name,
                    Vec2::new(400.0, 80.0),
                    Color::rgba(0.1, 0.1, 0.1, 0.9),
                );
            }

            if ui.button("+ Camera").clicked() {
                let name = format!("Camera {}", layer_count + 1);
                editor.add_camera_layer(&name);
            }

            if ui.button("+ Particles").clicked() {
                let name = format!("Particles {}", layer_count + 1);
                editor.add_particle_layer(&name, zeno_engine::ParticleEmitter::sparks());
            }

            let can_delete = editor.selected_layer_id.is_some();
            if ui.add_enabled(can_delete, Button::new("Delete")).clicked() {
                editor.delete_selected_layer();
            }

            ui.separator();

            let can_bring_forward = sel_idx.is_some_and(|i| i + 1 < layer_count);
            let can_send_backward = sel_idx.is_some_and(|i| i > 0);

            if ui
                .add_enabled(can_bring_forward, Button::new("Up"))
                .on_hover_text("Bring Forward (Ctrl+])")
                .clicked()
            {
                if let Some(i) = sel_idx {
                    editor.reorder_layer(i, i + 1);
                }
            }

            if ui
                .add_enabled(can_send_backward, Button::new("Down"))
                .on_hover_text("Send Backward (Ctrl+[)")
                .clicked()
            {
                if let Some(i) = sel_idx {
                    editor.reorder_layer(i, i - 1);
                }
            }
        });

        ui.separator();

        // Keyboard shortcuts for layer reordering
        let (key_front, key_fwd, key_bwd, key_back) = ui.input(|i| {
            let ctrl = i.modifiers.command || i.modifiers.ctrl;
            let shift = i.modifiers.shift;
            let close_br = i.key_pressed(egui::Key::CloseBracket);
            let open_br = i.key_pressed(egui::Key::OpenBracket);
            (
                ctrl && shift && close_br,
                ctrl && !shift && close_br,
                ctrl && !shift && open_br,
                ctrl && shift && open_br,
            )
        });

        if let Some(i) = sel_idx {
            if key_front && i + 1 < layer_count {
                editor.reorder_layer(i, layer_count - 1);
            } else if key_fwd && i + 1 < layer_count {
                editor.reorder_layer(i, i + 1);
            } else if key_bwd && i > 0 {
                editor.reorder_layer(i, i - 1);
            } else if key_back && i > 0 {
                editor.reorder_layer(i, 0);
            }
        }

        if layer_count == 0 {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.colored_label(
                    TEXT_MUTED,
                    "No layers yet — click + Text, + Shape, or import media",
                );
            });
            return;
        }

        let dnd_layer_id_key = egui::Id::new("dnd_dragging_layer_id");
        let dragging_layer_id: Option<LayerId> = ui.ctx().data(|d| d.get_temp(dnd_layer_id_key));

        let mut row_rects: Vec<egui::Rect> = Vec::with_capacity(layer_count);
        let mut row_actions: Vec<(usize, LayerOrderAction)> = Vec::new();

        // Layer List (Front at top / display_idx 0, Back at bottom / display_idx layer_count - 1)
        let scroll_output = egui::ScrollArea::vertical().show(ui, |ui| {
            let list_width = ui.available_width();
            for display_idx in 0..layer_count {
                let l_idx = layer_count - 1 - display_idx;
                let layer_id = editor.composition.layers[l_idx].id;
                let layer_name = editor.composition.layers[l_idx].name.clone();
                let layer_visible = editor.composition.layers[l_idx].visible;
                let is_3d = editor.composition.layers[l_idx].is_3d;
                let is_selected = editor.selected_layer_id == Some(layer_id);
                let is_dragged = dragging_layer_id == Some(layer_id);

                let bg_color = if is_dragged {
                    SURFACE_2
                } else if is_selected {
                    SURFACE_3
                } else {
                    SURFACE_1
                };

                let row_frame = egui::Frame::NONE
                    .fill(bg_color)
                    .corner_radius(CornerRadius::same(3))
                    .inner_margin(egui::Margin::symmetric(GRID_UNIT as i8, 4));

                let resp = row_frame.show(ui, |ui| {
                    ui.set_width(list_width);
                    ui.horizontal(|ui| {
                        // Drag handle grip
                        let grip_resp = ui
                            .colored_label(TEXT_MUTED, "::")
                            .on_hover_text("Drag to reorder");
                        if grip_resp.hovered() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                        }

                        // Eye visibility toggle
                        let eye_label = if layer_visible { "Show" } else { "Hide" };
                        if ui.selectable_label(layer_visible, eye_label).clicked() {
                            editor.composition.layers[l_idx].visible = !layer_visible;
                        }

                        // 3D toggle
                        let mode_3d_label = if is_3d { "[3D]" } else { "[2D]" };
                        if ui.selectable_label(is_3d, mode_3d_label).clicked() {
                            editor.composition.layers[l_idx].is_3d = !is_3d;
                        }

                        // Layer Name
                        let text_color = if is_selected { ACCENT } else { TEXT };
                        ui.add(
                            egui::Label::new(egui::RichText::new(&layer_name).color(text_color))
                                .truncate(),
                        );
                    });
                });

                let row_rect = resp.response.rect;
                row_rects.push(row_rect);

                // Row interaction
                let row_sense = ui.interact(
                    row_rect,
                    egui::Id::new("layer_row_interact").with(layer_id.to_string()),
                    Sense::click_and_drag(),
                );

                if row_sense.drag_started() {
                    ui.ctx()
                        .data_mut(|d| d.insert_temp(dnd_layer_id_key, layer_id));
                }

                if row_sense.clicked() {
                    editor.selected_layer_id = Some(layer_id);
                }

                // Context menu
                row_sense.context_menu(|ui| {
                    let cur_idx_opt = editor
                        .composition
                        .layers
                        .iter()
                        .position(|l| l.id == layer_id);
                    let can_front = cur_idx_opt.is_some_and(|i| i + 1 < layer_count);
                    let can_back = cur_idx_opt.is_some_and(|i| i > 0);

                    if ui
                        .add_enabled(can_front, Button::new("Bring to Front\tCtrl+Shift+]"))
                        .clicked()
                    {
                        if let Some(i) = cur_idx_opt {
                            row_actions.push((i, LayerOrderAction::BringToFront));
                        }
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(can_front, Button::new("Bring Forward\tCtrl+]"))
                        .clicked()
                    {
                        if let Some(i) = cur_idx_opt {
                            row_actions.push((i, LayerOrderAction::BringForward));
                        }
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(can_back, Button::new("Send Backward\tCtrl+["))
                        .clicked()
                    {
                        if let Some(i) = cur_idx_opt {
                            row_actions.push((i, LayerOrderAction::SendBackward));
                        }
                        ui.close_menu();
                    }
                    if ui
                        .add_enabled(can_back, Button::new("Send to Back\tCtrl+Shift+["))
                        .clicked()
                    {
                        if let Some(i) = cur_idx_opt {
                            row_actions.push((i, LayerOrderAction::SendToBack));
                        }
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Delete Layer").clicked() {
                        editor.selected_layer_id = Some(layer_id);
                        editor.delete_selected_layer();
                        ui.close_menu();
                    }
                });
            }
        });

        // Drag-and-drop indicator & drop execution
        if let Some(dragged_id) = dragging_layer_id {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);

            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let dragged_name = editor
                    .composition
                    .layers
                    .iter()
                    .find(|l| l.id == dragged_id)
                    .map(|l| l.name.as_str())
                    .unwrap_or("Layer");
                egui::show_tooltip(
                    ui.ctx(),
                    ui.layer_id(),
                    egui::Id::new("layer_drag_tooltip"),
                    |ui| {
                        ui.label(format!(":: {}", dragged_name));
                    },
                );

                if !row_rects.is_empty() {
                    let mut slot_y = Vec::with_capacity(layer_count + 1);
                    slot_y.push(row_rects[0].top());
                    for i in 1..layer_count {
                        slot_y.push((row_rects[i - 1].bottom() + row_rects[i].top()) * 0.5);
                    }
                    slot_y.push(row_rects[layer_count - 1].bottom());

                    let target_slot = (0..=layer_count)
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
                        let from_row = (0..layer_count).find(|&r| {
                            let l_idx = layer_count - 1 - r;
                            editor.composition.layers[l_idx].id == dragged_id
                        });
                        if let Some(from_r) = from_row {
                            if target_slot != from_r && target_slot != from_r + 1 {
                                let new_row = if target_slot < from_r {
                                    target_slot
                                } else {
                                    target_slot - 1
                                };
                                let from_idx = layer_count - 1 - from_r;
                                let to_idx = layer_count - 1 - new_row;
                                editor.reorder_layer(from_idx, to_idx);
                            }
                        }
                        ui.ctx()
                            .data_mut(|d| d.remove_temp::<LayerId>(dnd_layer_id_key));
                    }
                }
            } else if ui.input(|i| i.pointer.any_released()) {
                ui.ctx()
                    .data_mut(|d| d.remove_temp::<LayerId>(dnd_layer_id_key));
            }
        }

        // Apply context menu row actions
        for (i, action) in row_actions {
            match action {
                LayerOrderAction::BringToFront => {
                    editor.reorder_layer(i, layer_count - 1);
                }
                LayerOrderAction::BringForward => {
                    if i + 1 < layer_count {
                        editor.reorder_layer(i, i + 1);
                    }
                }
                LayerOrderAction::SendBackward => {
                    if i > 0 {
                        editor.reorder_layer(i, i - 1);
                    }
                }
                LayerOrderAction::SendToBack => {
                    editor.reorder_layer(i, 0);
                }
            }
        }
    });
}
