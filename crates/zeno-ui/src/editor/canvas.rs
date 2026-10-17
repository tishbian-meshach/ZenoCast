//! Interactive preview canvas and transform gizmos for Edit Overlay mode.

use egui::{vec2, Align2, CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind, Ui};
use zeno_core::Vec3;
use zeno_engine::eval::{evaluate_at, ResolvedContent};

use crate::editor::state::OverlayEditor;
use crate::theme::{
    color_to_egui, ACCENT, AMBER_WARN, BORDER_WIDTH, CORNER_RADIUS, GREEN_OK, RED_LIVE, SURFACE_0,
    SURFACE_2, SURFACE_3, TEXT, TEXT_MUTED,
};

/// Renders the central motion graphics canvas with interactive transform gizmos.
pub fn ui_canvas(editor: &mut OverlayEditor, ui: &mut Ui) {
    ui.vertical(|ui| {
        // Viewport Header Controls (Zoom, Pan, Fit, Timecode)
        ui.horizontal(|ui| {
            ui.label(format!(
                "Canvas: {}x{}",
                editor.composition.width, editor.composition.height
            ));
            ui.separator();

            if ui.button(" Zoom - ").clicked() {
                editor.zoom = (editor.zoom - 0.1).max(0.2);
            }
            ui.label(format!("{:.0}%", editor.zoom * 100.0));
            if ui.button(" Zoom + ").clicked() {
                editor.zoom = (editor.zoom + 0.1).min(3.0);
            }
            if ui.button(" Reset ").clicked() {
                editor.zoom = 1.0;
                editor.pan = zeno_core::Vec2::ZERO;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let tc = zeno_core::Timecode::from_seconds(
                    editor.clock.time(),
                    editor.composition.frames_per_second,
                );
                ui.colored_label(TEXT, format!("Time: {}", tc));
            });
        });

        ui.separator();

        // Canvas Area
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let canvas_rect = response.rect;

        // Background dark surface
        painter.rect(
            canvas_rect,
            CornerRadius::same(CORNER_RADIUS as u8),
            SURFACE_0,
            Stroke::new(BORDER_WIDTH, SURFACE_3),
            StrokeKind::Inside,
        );

        // Composition Frame Box (scaled by zoom)
        let comp_w = editor.composition.width as f32 * editor.zoom;
        let comp_h = editor.composition.height as f32 * editor.zoom;
        let comp_center = canvas_rect.center() + vec2(editor.pan.x, editor.pan.y);
        let comp_rect = Rect::from_center_size(comp_center, vec2(comp_w, comp_h));

        painter.rect(
            comp_rect,
            CornerRadius::ZERO,
            SURFACE_2,
            Stroke::new(BORDER_WIDTH, TEXT_MUTED),
            StrokeKind::Inside,
        );

        // Evaluate and Draw Layers at current playhead time
        let frame = evaluate_at(&editor.composition, editor.clock.time());

        for layer in &frame.layers {
            let col3 = layer.world_matrix.col(3);
            let layer_pos = comp_rect.min + vec2(col3.x * editor.zoom, col3.y * editor.zoom);

            match &layer.content {
                ResolvedContent::Text(text) => {
                    let font_size = text.font_size * editor.zoom;
                    let text_color = color_to_egui(text.color);
                    painter.text(
                        layer_pos,
                        Align2::CENTER_CENTER,
                        &text.text,
                        FontId::proportional(font_size),
                        text_color,
                    );
                }
                ResolvedContent::Shape(shape) => {
                    if let zeno_engine::eval::ResolvedShapeGeometry::Rectangle { size } =
                        shape.geometry
                    {
                        let rect_size = vec2(size.x * editor.zoom, size.y * editor.zoom);
                        let shape_rect = Rect::from_center_size(layer_pos, rect_size);

                        let fill_col = if let Some(ref fill) = shape.fill {
                            match fill {
                                zeno_engine::eval::ResolvedFill::Solid(c) => color_to_egui(*c),
                                _ => SURFACE_3,
                            }
                        } else {
                            SURFACE_3
                        };

                        painter.rect(
                            shape_rect,
                            CornerRadius::ZERO,
                            fill_col,
                            Stroke::NONE,
                            StrokeKind::Inside,
                        );
                    }
                }
                _ => {}
            }

            // Draw Transform Gizmo if this layer is selected
            if Some(layer.id) == editor.selected_layer_id {
                let gizmo_size = vec2(120.0 * editor.zoom, 40.0 * editor.zoom);
                let gizmo_rect = Rect::from_center_size(layer_pos, gizmo_size);

                // Bounding outline
                painter.rect_stroke(
                    gizmo_rect,
                    CornerRadius::ZERO,
                    Stroke::new(1.0_f32, ACCENT),
                    StrokeKind::Inside,
                );

                if layer.is_3d {
                    // 3D Gizmo coordinate axes: X (Red), Y (Green), Z (Blue)
                    let axis_len = 50.0 * editor.zoom;
                    let x_tip = layer_pos + vec2(axis_len, 0.0);
                    let y_tip = layer_pos + vec2(0.0, axis_len);
                    let z_tip = layer_pos + vec2(-axis_len * 0.7, axis_len * 0.7);

                    // X Axis (Red)
                    painter.line_segment([layer_pos, x_tip], Stroke::new(2.0_f32, RED_LIVE));
                    painter.circle_filled(x_tip, 4.0, RED_LIVE);

                    // Y Axis (Green)
                    painter.line_segment([layer_pos, y_tip], Stroke::new(2.0_f32, GREEN_OK));
                    painter.circle_filled(y_tip, 4.0, GREEN_OK);

                    // Z Axis (Blue / ACCENT)
                    painter.line_segment([layer_pos, z_tip], Stroke::new(2.0_f32, ACCENT));
                    painter.circle_filled(z_tip, 4.0, ACCENT);
                } else {
                    // Standard 2D corner handles
                    let handle_radius = 4.0;
                    let corners = [
                        gizmo_rect.left_top(),
                        gizmo_rect.right_top(),
                        gizmo_rect.left_bottom(),
                        gizmo_rect.right_bottom(),
                    ];
                    for corner in corners {
                        painter.rect_filled(
                            Rect::from_center_size(
                                corner,
                                vec2(handle_radius * 2.0, handle_radius * 2.0),
                            ),
                            CornerRadius::same(1),
                            ACCENT,
                        );
                    }
                }

                // Center Move Handle
                painter.circle_filled(layer_pos, 4.0, AMBER_WARN);

                // Interactive Dragging on Canvas
                if response.dragged() {
                    let delta = response.drag_delta();
                    if let Some(target_layer) = editor
                        .composition
                        .layers
                        .iter_mut()
                        .find(|l| l.id == layer.id)
                    {
                        let old_pos = *target_layer.transform.position.fallback_value();
                        let new_pos =
                            old_pos + Vec3::new(delta.x / editor.zoom, delta.y / editor.zoom, 0.0);
                        target_layer.transform.position = zeno_core::Property::constant(new_pos);
                    }
                }
            }
        }
    });
}
