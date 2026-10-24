//! Interactive preview canvas and transform gizmos for Edit Overlay mode.

use egui::{vec2, Align2, CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind, Ui};
use zeno_core::Vec3;
use zeno_engine::eval::{evaluate_at, ResolvedContent};

use crate::editor::state::OverlayEditor;
use crate::theme::{
    color_to_egui, ACCENT, BORDER_WIDTH, CORNER_RADIUS, GREEN_OK, RED_LIVE, SURFACE_0, SURFACE_2,
    SURFACE_3, TEXT, TEXT_MUTED,
};
use crate::transform_gizmo::{TransformGizmoEvent, TransformGizmoItem};

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
                ResolvedContent::Particles { ref particles, .. } => {
                    for p in particles {
                        let p_pos = layer_pos
                            + vec2(p.position[0] * editor.zoom, p.position[1] * editor.zoom);
                        let col = color_to_egui(zeno_core::Color::rgba(
                            p.color[0], p.color[1], p.color[2], p.color[3],
                        ));
                        painter.circle_filled(p_pos, (p.size * 0.5 * editor.zoom).max(1.0), col);
                    }
                }
                _ => {}
            }
        }

        // Interactive on-canvas Transform Gizmo for Overlay Composition
        let mut gizmo_items = Vec::new();
        for layer in &editor.composition.layers {
            let pos = *layer.transform.position.fallback_value();
            let rot = *layer.transform.rotation_degrees.fallback_value();
            let scale = *layer.transform.scale_percent.fallback_value();

            let (base_w, base_h) = match &layer.content {
                zeno_engine::LayerContent::Shape(s) => match &s.geometry {
                    zeno_engine::ShapeGeometry::Rectangle { size } => {
                        let s_val = *size.fallback_value();
                        (s_val.x, s_val.y)
                    }
                    zeno_engine::ShapeGeometry::Ellipse { radius } => {
                        let r_val = *radius.fallback_value();
                        (r_val.x * 2.0, r_val.y * 2.0)
                    }
                    _ => (120.0, 80.0),
                },
                zeno_engine::LayerContent::Text(t) => {
                    let fs = *t.font_size.fallback_value();
                    let txt = t.text.fallback_value();
                    let approx_char_w = fs * 0.55;
                    let w = (txt.len() as f32 * approx_char_w).max(60.0);
                    let h = (fs * 1.3).max(24.0);
                    (w, h)
                }
                zeno_engine::LayerContent::Particles(..) => (200.0, 200.0),
                zeno_engine::LayerContent::Image { .. }
                | zeno_engine::LayerContent::Video { .. } => (320.0, 180.0),
                _ => (160.0, 90.0),
            };

            let w = (base_w * (scale.x.abs() / 100.0)).max(10.0);
            let h = (base_h * (scale.y.abs() / 100.0)).max(10.0);

            gizmo_items.push(TransformGizmoItem {
                id: layer.id.0.to_string(),
                name: layer.name.clone(),
                center_x: pos.x,
                center_y: pos.y,
                width: w,
                height: h,
                rotation_degrees: rot.z,
                locked: false,
                visible: layer.visible,
            });
        }

        let selected_ids = if let Some(sel_id) = editor.selected_layer_id {
            vec![sel_id.0.to_string()]
        } else {
            Vec::new()
        };

        let canvas_size = vec2(
            editor.composition.width as f32,
            editor.composition.height as f32,
        );

        let events = editor.transform_gizmo_state.draw_and_interact(
            ui,
            comp_rect,
            canvas_size,
            egui::Vec2::ZERO,
            editor.zoom,
            &gizmo_items,
            &selected_ids,
        );

        // Draw 3D coordinate axes if selected layer is 3D
        if let Some(sel_id) = editor.selected_layer_id {
            if let Some(layer) = editor.composition.layers.iter().find(|l| l.id == sel_id) {
                if layer.is_3d {
                    let pos = *layer.transform.position.fallback_value();
                    let screen_center = comp_rect.center();
                    let sx = screen_center.x + (pos.x - canvas_size.x * 0.5) * editor.zoom;
                    let sy = screen_center.y + (pos.y - canvas_size.y * 0.5) * editor.zoom;
                    let layer_pos = egui::pos2(sx, sy);

                    let axis_len = 50.0 * editor.zoom;
                    let x_tip = layer_pos + vec2(axis_len, 0.0);
                    let y_tip = layer_pos + vec2(0.0, axis_len);
                    let z_tip = layer_pos + vec2(-axis_len * 0.7, axis_len * 0.7);

                    painter.line_segment([layer_pos, x_tip], Stroke::new(2.0_f32, RED_LIVE));
                    painter.circle_filled(x_tip, 4.0, RED_LIVE);

                    painter.line_segment([layer_pos, y_tip], Stroke::new(2.0_f32, GREEN_OK));
                    painter.circle_filled(y_tip, 4.0, GREEN_OK);

                    painter.line_segment([layer_pos, z_tip], Stroke::new(2.0_f32, ACCENT));
                    painter.circle_filled(z_tip, 4.0, ACCENT);
                }
            }
        }

        for event in events {
            match event {
                TransformGizmoEvent::SelectOnly(id_str) => {
                    if let Ok(uid) = uuid::Uuid::parse_str(&id_str) {
                        editor.selected_layer_id = Some(zeno_core::LayerId(uid));
                    }
                }
                TransformGizmoEvent::ToggleSelect(id_str) => {
                    if let Ok(uid) = uuid::Uuid::parse_str(&id_str) {
                        let id = zeno_core::LayerId(uid);
                        if editor.selected_layer_id == Some(id) {
                            editor.selected_layer_id = None;
                        } else {
                            editor.selected_layer_id = Some(id);
                        }
                    }
                }
                TransformGizmoEvent::SelectMultiple(ids) => {
                    if let Some(first) = ids.first() {
                        if let Ok(uid) = uuid::Uuid::parse_str(first) {
                            editor.selected_layer_id = Some(zeno_core::LayerId(uid));
                        }
                    }
                }
                TransformGizmoEvent::DeselectAll => {
                    editor.selected_layer_id = None;
                }
                TransformGizmoEvent::Transformed {
                    id,
                    center_x,
                    center_y,
                    width,
                    height,
                    rotation_degrees,
                } => {
                    if let Ok(uid) = uuid::Uuid::parse_str(&id) {
                        let lid = zeno_core::LayerId(uid);
                        if let Some(layer) =
                            editor.composition.layers.iter_mut().find(|l| l.id == lid)
                        {
                            let old_pos = *layer.transform.position.fallback_value();
                            let old_scale = *layer.transform.scale_percent.fallback_value();
                            let old_rot = *layer.transform.rotation_degrees.fallback_value();

                            let (base_w, base_h) = match &layer.content {
                                zeno_engine::LayerContent::Shape(s) => match &s.geometry {
                                    zeno_engine::ShapeGeometry::Rectangle { size } => {
                                        let s_val = *size.fallback_value();
                                        (s_val.x, s_val.y)
                                    }
                                    zeno_engine::ShapeGeometry::Ellipse { radius } => {
                                        let r_val = *radius.fallback_value();
                                        (r_val.x * 2.0, r_val.y * 2.0)
                                    }
                                    _ => (120.0, 80.0),
                                },
                                zeno_engine::LayerContent::Text(t) => {
                                    let fs = *t.font_size.fallback_value();
                                    let txt = t.text.fallback_value();
                                    let approx_char_w = fs * 0.55;
                                    let w = (txt.len() as f32 * approx_char_w).max(60.0);
                                    let h = (fs * 1.3).max(24.0);
                                    (w, h)
                                }
                                zeno_engine::LayerContent::Particles(..) => (200.0, 200.0),
                                zeno_engine::LayerContent::Image { .. }
                                | zeno_engine::LayerContent::Video { .. } => (320.0, 180.0),
                                _ => (160.0, 90.0),
                            };

                            let new_scale_x = (width / base_w.max(1.0)) * 100.0;
                            let new_scale_y = (height / base_h.max(1.0)) * 100.0;

                            layer.transform.position = zeno_core::Property::constant(Vec3::new(
                                center_x, center_y, old_pos.z,
                            ));
                            layer.transform.scale_percent = zeno_core::Property::constant(
                                Vec3::new(new_scale_x, new_scale_y, old_scale.z),
                            );
                            layer.transform.rotation_degrees = zeno_core::Property::constant(
                                Vec3::new(old_rot.x, old_rot.y, rotation_degrees),
                            );
                        }
                    }
                }
            }
        }
    });
}
