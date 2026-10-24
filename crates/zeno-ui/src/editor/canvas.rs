//! Interactive preview canvas and transform gizmos for Edit Overlay mode.

use egui::{vec2, Align2, CornerRadius, FontId, Rect, Sense, Stroke, StrokeKind, Ui};
use zeno_core::Vec3;
use zeno_engine::eval::{evaluate_at, ResolvedContent};

use crate::editor::state::OverlayEditor;
use crate::icon::{icon_button, IconKind};
use crate::theme::{
    color_to_egui, ACCENT, AMBER_WARN, BORDER_WIDTH, CORNER_RADIUS, RED_LIVE, SURFACE_0, SURFACE_1,
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

            if icon_button(ui, IconKind::Minus, "Zoom Out (-10%)").clicked() {
                editor.zoom = (editor.zoom - 0.1).max(0.2);
            }
            ui.label(format!("{:.0}%", editor.zoom * 100.0));
            if icon_button(ui, IconKind::Plus, "Zoom In (+10%)").clicked() {
                editor.zoom = (editor.zoom + 0.1).min(3.0);
            }
            if icon_button(ui, IconKind::RotateCcw, "Reset Viewport (100%, Origin)").clicked() {
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
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::hover());
        let canvas_rect = response.rect;

        // Middle-mouse drag to pan or mouse-wheel to zoom
        if response.hovered() {
            let scroll_delta = ui.input(|i| i.raw_scroll_delta);
            if scroll_delta.y != 0.0 {
                editor.zoom = (editor.zoom + scroll_delta.y * 0.001).clamp(0.1, 4.0);
            }
            if ui.input(|i| i.pointer.button_down(egui::PointerButton::Middle)) {
                let delta = ui.input(|i| i.pointer.delta());
                editor.pan.x += delta.x;
                editor.pan.y += delta.y;
            }
        }

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

        crate::theme::paint_checkerboard(&painter, comp_rect, 16.0);
        painter.rect_stroke(
            comp_rect,
            CornerRadius::ZERO,
            Stroke::new(BORDER_WIDTH, TEXT_MUTED),
            StrokeKind::Inside,
        );

        // Handle OS file drag-and-drop directly onto composition canvas
        let dropped_files = ui.input(|i| {
            if !i.raw.dropped_files.is_empty() {
                if let Some(pos) = i.pointer.hover_pos() {
                    if canvas_rect.contains(pos) {
                        return i.raw.dropped_files.clone();
                    }
                }
            }
            Vec::new()
        });

        for file in dropped_files {
            if let Some(ref path) = file.path {
                if let Some(asset_id) = editor.import_asset(path) {
                    let name = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Image Layer".into());
                    let (nw, nh) = editor
                        .asset_dimensions
                        .get(&asset_id)
                        .copied()
                        .unwrap_or((320, 180));
                    let max_w = editor.composition.width as f32;
                    let max_h = editor.composition.height as f32;
                    let mut w = nw as f32;
                    let mut h = nh as f32;
                    if w > max_w || h > max_h {
                        let s = (max_w / w).min(max_h / h);
                        w *= s;
                        h *= s;
                    }
                    editor.add_image_layer_from_asset(asset_id, &name, w, h);
                }
            }
        }

        // Handle internal drag-and-drop from Assets panel onto canvas
        let dragging_asset = ui.memory(|mem| {
            mem.data
                .get_temp::<String>(egui::Id::new("dnd_dragging_asset_id"))
        });

        if let Some(ref asset_id_str) = dragging_asset {
            if let Some(pos) = ui.ctx().pointer_latest_pos() {
                if canvas_rect.contains(pos) {
                    painter.rect_stroke(
                        comp_rect,
                        CornerRadius::ZERO,
                        Stroke::new(2.0_f32, ACCENT),
                        StrokeKind::Inside,
                    );
                    if !ui.input(|i| i.pointer.any_down()) {
                        if let Ok(uid) = uuid::Uuid::parse_str(asset_id_str) {
                            let asset_id = zeno_core::AssetId(uid);
                            let asset_name = editor
                                .composition
                                .assets
                                .iter()
                                .find(|a| a.id == asset_id)
                                .map(|a| {
                                    std::path::Path::new(&a.path)
                                        .file_name()
                                        .map(|f| f.to_string_lossy().to_string())
                                        .unwrap_or_else(|| "Image Layer".into())
                                })
                                .unwrap_or_else(|| "Image Layer".into());

                            let (nw, nh) = editor
                                .asset_dimensions
                                .get(&asset_id)
                                .copied()
                                .unwrap_or((320, 180));
                            let max_w = editor.composition.width as f32;
                            let max_h = editor.composition.height as f32;
                            let mut w = nw as f32;
                            let mut h = nh as f32;
                            if w > max_w || h > max_h {
                                let s = (max_w / w).min(max_h / h);
                                w *= s;
                                h *= s;
                            }
                            editor.add_image_layer_from_asset(asset_id, &asset_name, w, h);
                        }
                        ui.memory_mut(|mem| {
                            mem.data
                                .remove::<String>(egui::Id::new("dnd_dragging_asset_id"))
                        });
                    }
                }
            }
            if !ui.input(|i| i.pointer.any_down()) {
                ui.memory_mut(|mem| {
                    mem.data
                        .remove::<String>(egui::Id::new("dnd_dragging_asset_id"))
                });
            }
        }

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
                ResolvedContent::Image { asset } => {
                    let (nw, nh) = editor
                        .asset_dimensions
                        .get(asset)
                        .copied()
                        .unwrap_or((320, 180));
                    let scale = editor
                        .composition
                        .layers
                        .iter()
                        .find(|l| l.id == layer.id)
                        .map(|l| *l.transform.scale_percent.fallback_value())
                        .unwrap_or(Vec3::splat(100.0));
                    let img_w = (nw as f32 * (scale.x.abs() / 100.0) * editor.zoom).max(16.0);
                    let img_h = (nh as f32 * (scale.y.abs() / 100.0) * editor.zoom).max(16.0);
                    let img_rect = Rect::from_center_size(layer_pos, vec2(img_w, img_h));

                    painter.rect(
                        img_rect,
                        CornerRadius::same(CORNER_RADIUS as u8),
                        SURFACE_1,
                        Stroke::new(BORDER_WIDTH, SURFACE_3),
                        StrokeKind::Inside,
                    );

                    let filename = editor
                        .composition
                        .assets
                        .iter()
                        .find(|a| &a.id == asset)
                        .map(|a| {
                            std::path::Path::new(&a.path)
                                .file_name()
                                .map(|f| f.to_string_lossy().to_string())
                                .unwrap_or_else(|| a.path.clone())
                        })
                        .unwrap_or_else(|| "Image".into());

                    let badge_text = format!("[IMG] {}", filename);
                    painter.text(
                        img_rect.center(),
                        Align2::CENTER_CENTER,
                        badge_text,
                        FontId::proportional(13.0),
                        TEXT,
                    );
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
                zeno_engine::LayerContent::Image { asset } => {
                    if let Some(&(w, h)) = editor.asset_dimensions.get(asset) {
                        (w as f32, h as f32)
                    } else {
                        (320.0, 180.0)
                    }
                }
                zeno_engine::LayerContent::Video { .. } => (320.0, 180.0),
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
            canvas_rect,
            canvas_size,
            vec2(editor.pan.x, editor.pan.y),
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

                    painter.line_segment([layer_pos, y_tip], Stroke::new(2.0_f32, AMBER_WARN));
                    painter.circle_filled(y_tip, 4.0, AMBER_WARN);

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
                                zeno_engine::LayerContent::Image { asset } => {
                                    if let Some(&(w, h)) = editor.asset_dimensions.get(asset) {
                                        (w as f32, h as f32)
                                    } else {
                                        (320.0, 180.0)
                                    }
                                }
                                zeno_engine::LayerContent::Video { .. } => (320.0, 180.0),
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
