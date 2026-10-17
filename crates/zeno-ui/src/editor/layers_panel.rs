//! Layers panel: hierarchy list, add/remove, reordering, visibility and lock toggling.

use egui::{Button, Ui};
use zeno_core::{Color, Vec2};

use crate::editor::state::OverlayEditor;
use crate::theme::{GRID_UNIT, SURFACE_1, SURFACE_3};

/// Renders the layer hierarchy panel.
pub fn ui_layers_panel(editor: &mut OverlayEditor, ui: &mut Ui) {
    ui.vertical(|ui| {
        // Toolbar: Add Layer, Delete Layer, Up, Down
        ui.horizontal(|ui| {
            if ui.button("+ Text").clicked() {
                let name = format!("Text Layer {}", editor.composition.layers.len() + 1);
                editor.add_text_layer(&name, "New Text");
            }

            if ui.button("+ Shape").clicked() {
                let name = format!("Shape Layer {}", editor.composition.layers.len() + 1);
                editor.add_shape_layer(
                    &name,
                    Vec2::new(400.0, 80.0),
                    Color::rgba(0.1, 0.1, 0.1, 0.9),
                );
            }

            let can_delete = editor.selected_layer_id.is_some();
            if ui.add_enabled(can_delete, Button::new("Delete")).clicked() {
                editor.delete_selected_layer();
            }

            // Move Up / Down
            if let Some(selected_id) = editor.selected_layer_id {
                let current_idx = editor
                    .composition
                    .layers
                    .iter()
                    .position(|l| l.id == selected_id);
                if let Some(idx) = current_idx {
                    if idx > 0 && ui.button("Up").clicked() {
                        editor.reorder_layer(idx, idx - 1);
                    }
                    if idx + 1 < editor.composition.layers.len() && ui.button("Down").clicked() {
                        editor.reorder_layer(idx, idx + 1);
                    }
                }
            }
        });

        ui.separator();

        // Layer List
        egui::ScrollArea::vertical().show(ui, |ui| {
            let layer_count = editor.composition.layers.len();
            for i in 0..layer_count {
                let layer_id = editor.composition.layers[i].id;
                let is_selected = editor.selected_layer_id == Some(layer_id);

                let bg_color = if is_selected { SURFACE_3 } else { SURFACE_1 };

                egui::Frame::NONE
                    .fill(bg_color)
                    .inner_margin(egui::Margin::symmetric(GRID_UNIT as i8, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Eye visibility toggle
                            let eye_label = if editor.composition.layers[i].visible {
                                "[o]"
                            } else {
                                "[-]"
                            };
                            if ui
                                .selectable_label(editor.composition.layers[i].visible, eye_label)
                                .clicked()
                            {
                                editor.composition.layers[i].visible =
                                    !editor.composition.layers[i].visible;
                            }

                            // Layer Name click selects layer
                            let text = &editor.composition.layers[i].name;
                            let label = ui.selectable_label(is_selected, text);
                            if label.clicked() {
                                editor.selected_layer_id = Some(layer_id);
                            }
                        });
                    });
            }
        });
    });
}
