//! Timeline track viewer, playhead scrubbing, keyframe manipulation,
//! timeline markers, In/Loop/Out region markers, and interactive Bezier Curve Editor.

use egui::{
    vec2, Align2, CornerRadius, CursorIcon, FontId, Id, Key, Pos2, Rect, Sense, Stroke, StrokeKind,
    Ui,
};
use zeno_core::{Easing, Keyframe, LayerId, Lifecycle, Property, TimeRange, Vec3};

use crate::editor::command::AnimProperty;
use crate::editor::state::{OverlayEditor, SelectedKeyframe, TimelineMarker};
use crate::icon::{icon_button, IconKind};
use crate::theme::{
    ACCENT, AMBER_WARN, CORNER_RADIUS, RED_LIVE, REGION_IN, REGION_LOOP, REGION_OUT,
    ROW_HEIGHT_LIST, SURFACE_0, SURFACE_1, SURFACE_2, SURFACE_3, TEXT, TEXT_MUTED,
};

/// Active mode of the bottom timeline area
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineMode {
    DopeSheet,
    CurveEditor,
}

#[derive(Debug, Clone, PartialEq)]
enum TimelineDragTarget {
    Playhead,
    InBoundary,
    LoopBoundary,
    Keyframe {
        layer_id: LayerId,
        prop: AnimProperty,
        initial_time: f64,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct ContextMenuData {
    layer_id: LayerId,
    property: Option<AnimProperty>,
    time: f64,
}

/// Renders the bottom timeline component.
pub fn ui_timeline(editor: &mut OverlayEditor, mode: &mut TimelineMode, ui: &mut Ui) {
    ui.vertical(|ui| {
        // Controls Toolbar: Play/Pause, Stop, Timecode, Add Marker, Mode Switch
        ui.horizontal(|ui| {
            let is_playing = editor.clock.is_playing();
            if icon_button(
                ui,
                if is_playing {
                    IconKind::Pause
                } else {
                    IconKind::Play
                },
                if is_playing {
                    "Pause (Space)"
                } else {
                    "Play (Space)"
                },
            )
            .clicked()
            {
                if is_playing {
                    editor.clock.pause();
                } else {
                    editor.clock.play();
                }
            }

            if icon_button(ui, IconKind::Square, "Stop Playback").clicked() {
                editor.clock.stop();
            }

            ui.separator();

            let cur_time = editor.clock.time();
            let dur = editor.composition.duration_seconds;
            ui.label(format!("Time: {:.2}s / {:.2}s", cur_time, dur));

            if ui
                .selectable_label(*mode == TimelineMode::DopeSheet, "Dope Sheet")
                .clicked()
            {
                *mode = TimelineMode::DopeSheet;
            }
            if ui
                .selectable_label(*mode == TimelineMode::CurveEditor, "Curve Editor")
                .clicked()
            {
                *mode = TimelineMode::CurveEditor;
            }

            ui.separator();

            if icon_button(ui, IconKind::Plus, "Add Marker").clicked() {
                editor.markers.push(TimelineMarker {
                    time_seconds: cur_time,
                    name: format!("Marker {}", editor.markers.len() + 1),
                    comment: None,
                });
            }
        });

        ui.separator();

        match mode {
            TimelineMode::DopeSheet => ui_dopesheet(editor, ui),
            TimelineMode::CurveEditor => ui_curve_editor(editor, ui),
        }
    });
}

/// Dope sheet track view with playhead, keyframe retiming, and draggable In/Loop/Out regions.
fn ui_dopesheet(editor: &mut OverlayEditor, ui: &mut Ui) {
    let dur = (editor.composition.duration_seconds as f32).max(0.1);

    // Ensure composition lifecycle exists
    if editor.composition.lifecycle.is_none() {
        editor.composition.lifecycle = Some(Lifecycle::default());
    }

    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, CornerRadius::same(CORNER_RADIUS as u8), SURFACE_0);

    // Track headers on left (160px width)
    let header_width = 160.0;
    let ruler_height = 28.0;
    let track_rect = Rect::from_min_max(
        Pos2::new(rect.min.x + header_width, rect.min.y + ruler_height),
        rect.max,
    );

    // Time ruler background
    let ruler_rect = Rect::from_min_max(
        Pos2::new(rect.min.x + header_width, rect.min.y),
        Pos2::new(rect.max.x, rect.min.y + ruler_height),
    );
    painter.rect_filled(ruler_rect, CornerRadius::ZERO, SURFACE_2);

    let pixels_per_second = track_rect.width() / dur;

    // Draw second markers on ruler
    let total_secs = dur.ceil() as i32;
    for s in 0..=total_secs {
        let x = track_rect.min.x + s as f32 * pixels_per_second;
        if x <= track_rect.max.x {
            painter.line_segment(
                [
                    Pos2::new(x, ruler_rect.min.y),
                    Pos2::new(x, ruler_rect.max.y),
                ],
                Stroke::new(1.0_f32, SURFACE_3),
            );
            painter.text(
                Pos2::new(x + 2.0, ruler_rect.min.y + 2.0),
                Align2::LEFT_TOP,
                format!("{}s", s),
                FontId::proportional(12.0),
                TEXT_MUTED,
            );
        }
    }

    // In / Loop / Out lifecycle regions on ruler
    let (in_end, loop_end) = if let Some(ref lc) = editor.composition.lifecycle {
        (
            lc.in_range.end_seconds as f32,
            lc.loop_range.end_seconds as f32,
        )
    } else {
        (1.0, 3.0)
    };

    let in_x = track_rect.min.x + in_end * pixels_per_second;
    let loop_x = track_rect.min.x + loop_end * pixels_per_second;

    // Draw In Region (Neutral surface tint)
    let in_rect = Rect::from_min_max(
        Pos2::new(track_rect.min.x, ruler_rect.min.y + 16.0),
        Pos2::new(in_x.min(track_rect.max.x), ruler_rect.max.y),
    );
    painter.rect_filled(in_rect, CornerRadius::ZERO, REGION_IN);
    painter.text(
        Pos2::new(track_rect.min.x + 4.0, ruler_rect.min.y + 16.0),
        Align2::LEFT_TOP,
        "Intro",
        FontId::proportional(12.0),
        ACCENT,
    );

    // Draw Loop Region (Neutral surface tint)
    let loop_rect = Rect::from_min_max(
        Pos2::new(
            in_x.clamp(track_rect.min.x, track_rect.max.x),
            ruler_rect.min.y + 16.0,
        ),
        Pos2::new(loop_x.min(track_rect.max.x), ruler_rect.max.y),
    );
    painter.rect_filled(loop_rect, CornerRadius::ZERO, REGION_LOOP);
    painter.text(
        Pos2::new(in_x + 4.0, ruler_rect.min.y + 16.0),
        Align2::LEFT_TOP,
        "Loop",
        FontId::proportional(12.0),
        AMBER_WARN,
    );

    // Draw Out Region (Neutral surface tint)
    let out_rect = Rect::from_min_max(
        Pos2::new(
            loop_x.clamp(track_rect.min.x, track_rect.max.x),
            ruler_rect.min.y + 16.0,
        ),
        Pos2::new(track_rect.max.x, ruler_rect.max.y),
    );
    painter.rect_filled(out_rect, CornerRadius::ZERO, REGION_OUT);
    painter.text(
        Pos2::new(loop_x + 4.0, ruler_rect.min.y + 16.0),
        Align2::LEFT_TOP,
        "Out",
        FontId::proportional(12.0),
        RED_LIVE,
    );

    // Draw In/Loop boundary marker handle
    painter.line_segment(
        [
            Pos2::new(in_x, ruler_rect.min.y),
            Pos2::new(in_x, rect.max.y),
        ],
        Stroke::new(1.5_f32, ACCENT),
    );
    let in_handle = [
        Pos2::new(in_x - 5.0, ruler_rect.min.y),
        Pos2::new(in_x + 5.0, ruler_rect.min.y),
        Pos2::new(in_x, ruler_rect.min.y + 8.0),
    ];
    painter.add(egui::Shape::convex_polygon(
        in_handle.to_vec(),
        ACCENT,
        Stroke::NONE,
    ));

    // Draw Loop/Out boundary marker handle
    painter.line_segment(
        [
            Pos2::new(loop_x, ruler_rect.min.y),
            Pos2::new(loop_x, rect.max.y),
        ],
        Stroke::new(1.5_f32, AMBER_WARN),
    );
    let loop_handle = [
        Pos2::new(loop_x - 5.0, ruler_rect.min.y),
        Pos2::new(loop_x + 5.0, ruler_rect.min.y),
        Pos2::new(loop_x, ruler_rect.min.y + 8.0),
    ];
    painter.add(egui::Shape::convex_polygon(
        loop_handle.to_vec(),
        AMBER_WARN,
        Stroke::NONE,
    ));

    // Draw Timeline Markers
    for marker in &editor.markers {
        let mx = track_rect.min.x + (marker.time_seconds as f32) * pixels_per_second;
        if mx >= track_rect.min.x && mx <= track_rect.max.x {
            painter.line_segment(
                [Pos2::new(mx, ruler_rect.min.y), Pos2::new(mx, rect.max.y)],
                Stroke::new(1.0_f32, AMBER_WARN),
            );
            painter.text(
                Pos2::new(mx + 2.0, ruler_rect.min.y + 12.0),
                Align2::LEFT_TOP,
                &marker.name,
                FontId::proportional(12.0),
                AMBER_WARN,
            );
        }
    }

    // Collect keyframe hitboxes for interactive selection and retiming
    struct KeyframeHit {
        rect: Rect,
        layer_id: LayerId,
        prop: AnimProperty,
        time: f64,
        color: egui::Color32,
    }

    let mut keyframe_hits = Vec::new();
    let track_height = ROW_HEIGHT_LIST;

    // Draw Tracks (Topmost/Foreground layer at top)
    for (idx, layer) in editor.composition.layers.iter().rev().enumerate() {
        let y_top = track_rect.min.y + idx as f32 * track_height;
        let y_bot = y_top + track_height;

        if y_bot > track_rect.max.y {
            break;
        }

        // Layer Name in header
        let is_selected = editor.selected_layer_id == Some(layer.id);
        let header_bg = if is_selected { SURFACE_3 } else { SURFACE_1 };
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(rect.min.x, y_top),
                Pos2::new(rect.min.x + header_width, y_bot),
            ),
            CornerRadius::ZERO,
            header_bg,
        );
        painter.text(
            Pos2::new(rect.min.x + 8.0, y_top + 6.0),
            Align2::LEFT_TOP,
            &layer.name,
            FontId::proportional(12.0),
            if is_selected { TEXT } else { TEXT_MUTED },
        );

        // Layer Track span
        let l_in_x = (track_rect.min.x + (layer.in_point_seconds as f32) * pixels_per_second)
            .clamp(track_rect.min.x, track_rect.max.x);
        let l_out_x = (track_rect.min.x + (layer.out_point_seconds as f32) * pixels_per_second)
            .clamp(track_rect.min.x, track_rect.max.x);
        let span_rect = Rect::from_min_max(
            Pos2::new(l_in_x, y_top + 4.0),
            Pos2::new(l_out_x, y_bot - 4.0),
        );

        painter.rect_filled(span_rect, CornerRadius::same(2), SURFACE_2);
        painter.rect_stroke(
            span_rect,
            CornerRadius::same(2),
            Stroke::new(1.0_f32, SURFACE_3),
            StrokeKind::Inside,
        );

        // Draw and register Keyframe Diamonds
        let mut add_kf_hits = |prop: AnimProperty, kfs: &[Keyframe<Vec3>], color: egui::Color32| {
            for kf in kfs {
                let kf_x = track_rect.min.x + (kf.time_seconds as f32) * pixels_per_second;
                let kf_y = y_top + track_height * 0.5;
                let hit_rect = Rect::from_center_size(Pos2::new(kf_x, kf_y), vec2(14.0, 14.0));
                keyframe_hits.push(KeyframeHit {
                    rect: hit_rect,
                    layer_id: layer.id,
                    prop,
                    time: kf.time_seconds,
                    color,
                });
            }
        };

        add_kf_hits(
            AnimProperty::Position,
            layer.transform.position.as_keyframes(),
            ACCENT,
        );
        add_kf_hits(
            AnimProperty::Scale,
            layer.transform.scale_percent.as_keyframes(),
            TEXT,
        );
        add_kf_hits(
            AnimProperty::Rotation,
            layer.transform.rotation_degrees.as_keyframes(),
            AMBER_WARN,
        );

        for kf in layer.transform.opacity_percent.as_keyframes() {
            let kf_x = track_rect.min.x + (kf.time_seconds as f32) * pixels_per_second;
            let kf_y = y_top + track_height * 0.5;
            let hit_rect = Rect::from_center_size(Pos2::new(kf_x, kf_y), vec2(14.0, 14.0));
            keyframe_hits.push(KeyframeHit {
                rect: hit_rect,
                layer_id: layer.id,
                prop: AnimProperty::Opacity,
                time: kf.time_seconds,
                color: RED_LIVE,
            });
        }
    }

    // Render keyframe diamonds with selection styling
    for hit in &keyframe_hits {
        let center = hit.rect.center();
        let d_size = 5.0;
        let points = [
            Pos2::new(center.x, center.y - d_size),
            Pos2::new(center.x + d_size, center.y),
            Pos2::new(center.x, center.y + d_size),
            Pos2::new(center.x - d_size, center.y),
        ];

        let is_selected_kf = if let Some(ref sel) = editor.selected_keyframe {
            sel.layer_id == hit.layer_id
                && sel.property == hit.prop
                && (sel.time - hit.time).abs() < 1e-3
        } else {
            false
        };

        let stroke = if is_selected_kf {
            Stroke::new(2.0_f32, TEXT)
        } else {
            Stroke::new(1.0_f32, SURFACE_0)
        };

        painter.add(egui::Shape::convex_polygon(
            points.to_vec(),
            hit.color,
            stroke,
        ));
    }

    // Playhead Scrubber line
    let cur_time = editor.clock.time() as f32;
    let playhead_x =
        (track_rect.min.x + cur_time * pixels_per_second).clamp(track_rect.min.x, track_rect.max.x);

    // Vertical red playhead line
    painter.line_segment(
        [
            Pos2::new(playhead_x, rect.min.y),
            Pos2::new(playhead_x, rect.max.y),
        ],
        Stroke::new(2.0_f32, RED_LIVE),
    );

    // Playhead handle
    let handle_pts = [
        Pos2::new(playhead_x - 6.0, rect.min.y),
        Pos2::new(playhead_x + 6.0, rect.min.y),
        Pos2::new(playhead_x, rect.min.y + 12.0),
    ];
    painter.add(egui::Shape::convex_polygon(
        handle_pts.to_vec(),
        RED_LIVE,
        Stroke::NONE,
    ));

    // Handle Delete key for selected keyframe
    if ui.input(|i| i.key_pressed(Key::Delete)) {
        if let Some(ref sel) = editor.selected_keyframe {
            editor.delete_keyframe(sel.layer_id, sel.property, sel.time);
        }
    }

    // Hover cursor feedback
    if let Some(hover_pos) = response.hover_pos() {
        if ruler_rect.contains(hover_pos) {
            if (hover_pos.x - in_x).abs() <= 6.0 || (hover_pos.x - loop_x).abs() <= 6.0 {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
            }
        } else {
            for hit in &keyframe_hits {
                if hit.rect.contains(hover_pos) {
                    ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    break;
                }
            }
        }
    }

    // Drag Interaction State Handling
    let drag_id = Id::new("timeline_drag_target");
    let mut current_drag: Option<TimelineDragTarget> = ui.memory(|mem| mem.data.get_temp(drag_id));

    if response.drag_started() {
        if let Some(pos) = response.interact_pointer_pos() {
            if ruler_rect.contains(pos) && (pos.x - in_x).abs() <= 8.0 {
                current_drag = Some(TimelineDragTarget::InBoundary);
            } else if ruler_rect.contains(pos) && (pos.x - loop_x).abs() <= 8.0 {
                current_drag = Some(TimelineDragTarget::LoopBoundary);
            } else {
                let mut found_kf = None;
                for hit in &keyframe_hits {
                    if hit.rect.contains(pos) {
                        found_kf = Some((hit.layer_id, hit.prop, hit.time));
                        break;
                    }
                }
                if let Some((layer_id, prop, time)) = found_kf {
                    editor.selected_keyframe = Some(SelectedKeyframe {
                        layer_id,
                        property: prop,
                        time,
                    });
                    editor.selected_layer_id = Some(layer_id);
                    current_drag = Some(TimelineDragTarget::Keyframe {
                        layer_id,
                        prop,
                        initial_time: time,
                    });
                } else {
                    current_drag = Some(TimelineDragTarget::Playhead);
                }
            }
            ui.memory_mut(|mem| mem.data.insert_temp(drag_id, current_drag.clone()));
        }
    }

    if response.dragged() {
        if let Some(pos) = response.interact_pointer_pos() {
            let frac = ((pos.x - track_rect.min.x) / track_rect.width()).clamp(0.0, 1.0);
            let target_time = (frac * dur) as f64;

            match current_drag {
                Some(TimelineDragTarget::InBoundary) => {
                    if let Some(ref mut lc) = editor.composition.lifecycle {
                        let clamped_in = target_time.clamp(0.1, lc.loop_range.end_seconds - 0.1);
                        lc.in_range = TimeRange::new(0.0, clamped_in);
                        lc.loop_range = TimeRange::new(clamped_in, lc.loop_range.end_seconds);
                        if let Some(s) = editor
                            .composition
                            .states
                            .iter_mut()
                            .find(|s| s.name == "in")
                        {
                            s.end_seconds = clamped_in;
                        }
                        if let Some(s) = editor
                            .composition
                            .states
                            .iter_mut()
                            .find(|s| s.name == "loop")
                        {
                            s.start_seconds = clamped_in;
                        }
                    }
                }
                Some(TimelineDragTarget::LoopBoundary) => {
                    if let Some(ref mut lc) = editor.composition.lifecycle {
                        let clamped_loop =
                            target_time.clamp(lc.in_range.end_seconds + 0.1, dur as f64);
                        lc.loop_range = TimeRange::new(lc.loop_range.start_seconds, clamped_loop);
                        lc.out_range = TimeRange::new(clamped_loop, dur as f64);
                        if let Some(s) = editor
                            .composition
                            .states
                            .iter_mut()
                            .find(|s| s.name == "loop")
                        {
                            s.end_seconds = clamped_loop;
                        }
                        if let Some(s) = editor
                            .composition
                            .states
                            .iter_mut()
                            .find(|s| s.name == "out")
                        {
                            s.start_seconds = clamped_loop;
                        }
                    }
                }
                Some(TimelineDragTarget::Keyframe {
                    layer_id,
                    prop,
                    ref mut initial_time,
                }) => {
                    let clamped_t = target_time.clamp(0.0, dur as f64);
                    if (clamped_t - *initial_time).abs() > 1e-3 {
                        editor.move_keyframe(layer_id, prop, *initial_time, clamped_t);
                        *initial_time = clamped_t;
                        editor.clock.seek(clamped_t);
                    }
                }
                Some(TimelineDragTarget::Playhead) | None => {
                    editor.clock.seek(target_time);
                }
            }
        }
    }

    if response.drag_stopped() || !ui.input(|i| i.pointer.any_down()) {
        ui.memory_mut(|mem| mem.data.remove::<Option<TimelineDragTarget>>(drag_id));
    }

    // Click selection (when not dragged)
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let mut found_kf = None;
            for hit in &keyframe_hits {
                if hit.rect.contains(pos) {
                    found_kf = Some((hit.layer_id, hit.prop, hit.time));
                    break;
                }
            }
            if let Some((layer_id, prop, time)) = found_kf {
                editor.selected_keyframe = Some(SelectedKeyframe {
                    layer_id,
                    property: prop,
                    time,
                });
                editor.selected_layer_id = Some(layer_id);
            } else if pos.x >= track_rect.min.x && pos.x <= track_rect.max.x {
                editor.selected_keyframe = None;
                let frac = (pos.x - track_rect.min.x) / track_rect.width();
                let target_time =
                    ((frac * dur) as f64).clamp(0.0, editor.composition.duration_seconds);
                editor.clock.seek(target_time);
            }
        }
    }

    // Right-click context menu handling
    let menu_id = Id::new("timeline_context_menu");
    let mut menu_data: Option<ContextMenuData> = ui.memory(|mem| mem.data.get_temp(menu_id));

    if response.secondary_clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            let mut hit_kf = None;
            for hit in &keyframe_hits {
                if hit.rect.contains(pos) {
                    hit_kf = Some((hit.layer_id, hit.prop, hit.time));
                    break;
                }
            }

            if let Some((layer_id, prop, time)) = hit_kf {
                menu_data = Some(ContextMenuData {
                    layer_id,
                    property: Some(prop),
                    time,
                });
                editor.selected_keyframe = Some(SelectedKeyframe {
                    layer_id,
                    property: prop,
                    time,
                });
                editor.selected_layer_id = Some(layer_id);
            } else if let Some(selected_layer) = editor.selected_layer_id {
                menu_data = Some(ContextMenuData {
                    layer_id: selected_layer,
                    property: None,
                    time: editor.clock.time(),
                });
            }
            ui.memory_mut(|mem| mem.data.insert_temp(menu_id, menu_data.clone()));
        }
    }

    // Render context menu if active
    response.context_menu(|ui| {
        if let Some(ref data) = menu_data {
            if let Some(prop) = data.property {
                ui.label(format!("Keyframe at {:.2}s ({:?})", data.time, prop));
                ui.separator();

                if ui.button("Delete Keyframe").clicked() {
                    editor.delete_keyframe(data.layer_id, prop, data.time);
                    ui.close_menu();
                }

                ui.separator();
                ui.label("Easing:");
                if ui.button("Linear").clicked() {
                    editor.set_keyframe_easing(data.layer_id, prop, data.time, Easing::Linear);
                    ui.close_menu();
                }
                if ui.button("Ease In").clicked() {
                    editor.set_keyframe_easing(data.layer_id, prop, data.time, Easing::ease_in());
                    ui.close_menu();
                }
                if ui.button("Ease Out").clicked() {
                    editor.set_keyframe_easing(data.layer_id, prop, data.time, Easing::ease_out());
                    ui.close_menu();
                }
                if ui.button("Ease In-Out").clicked() {
                    editor.set_keyframe_easing(
                        data.layer_id,
                        prop,
                        data.time,
                        Easing::ease_in_out(),
                    );
                    ui.close_menu();
                }
            } else {
                ui.label("Add Keyframe at Playhead:");
                if ui.button("Add Position Keyframe").clicked() {
                    if let Some(layer) = editor
                        .composition
                        .layers
                        .iter()
                        .find(|l| l.id == data.layer_id)
                    {
                        let pos = *layer.transform.position.fallback_value();
                        editor.add_position_keyframe(pos, Easing::ease());
                    }
                    ui.close_menu();
                }
                if ui.button("Add Opacity Keyframe").clicked() {
                    if let Some(layer) = editor
                        .composition
                        .layers
                        .iter()
                        .find(|l| l.id == data.layer_id)
                    {
                        let op = *layer.transform.opacity_percent.fallback_value();
                        editor.add_opacity_keyframe(op, Easing::ease());
                    }
                    ui.close_menu();
                }
            }
        }
    });
}

/// Interactive Bezier Curve Editor for keyframe easing
fn ui_curve_editor(editor: &mut OverlayEditor, ui: &mut Ui) {
    ui.horizontal(|ui| {
        ui.label("Easing Presets:");
        if ui.button("Linear").clicked() {
            set_selected_easing(editor, Easing::Linear);
        }
        if ui.button("Ease In").clicked() {
            set_selected_easing(editor, Easing::ease_in());
        }
        if ui.button("Ease Out").clicked() {
            set_selected_easing(editor, Easing::ease_out());
        }
        if ui.button("Ease In-Out").clicked() {
            set_selected_easing(editor, Easing::ease_in_out());
        }
    });

    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = response.rect;

    // Background grid
    painter.rect_filled(rect, CornerRadius::same(CORNER_RADIUS as u8), SURFACE_0);
    painter.rect_stroke(
        rect,
        CornerRadius::same(CORNER_RADIUS as u8),
        Stroke::new(1.0_f32, SURFACE_3),
        StrokeKind::Inside,
    );

    // Draw curve plot representation (0,0 bottom-left to 1,1 top-right)
    let margin = 40.0;
    let plot_rect = Rect::from_min_max(
        Pos2::new(rect.min.x + margin, rect.min.y + margin),
        Pos2::new(rect.max.x - margin, rect.max.y - margin),
    );

    // Plot axis
    painter.rect_stroke(
        plot_rect,
        CornerRadius::ZERO,
        Stroke::new(1.0_f32, SURFACE_2),
        StrokeKind::Inside,
    );

    // Draw evaluated Bezier curve points
    let steps = 60;
    let mut prev_pt = Pos2::new(plot_rect.min.x, plot_rect.max.y);

    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let eased_y =
            3.0 * (1.0 - t) * (1.0 - t) * t * 0.42 + 3.0 * (1.0 - t) * t * t * 1.0 + t * t * t;
        let x = plot_rect.min.x + t * plot_rect.width();
        let y = plot_rect.max.y - eased_y * plot_rect.height();
        let current_pt = Pos2::new(x, y);

        painter.line_segment([prev_pt, current_pt], Stroke::new(2.0_f32, ACCENT));
        prev_pt = current_pt;
    }

    // Draw control handles
    let p0 = Pos2::new(plot_rect.min.x, plot_rect.max.y);
    let p1 = Pos2::new(
        plot_rect.min.x + 0.25 * plot_rect.width(),
        plot_rect.max.y - 0.1 * plot_rect.height(),
    );
    let p2 = Pos2::new(plot_rect.min.x + 0.25 * plot_rect.width(), plot_rect.min.y);
    let p3 = Pos2::new(plot_rect.max.x, plot_rect.min.y);

    painter.line_segment([p0, p1], Stroke::new(1.0_f32, AMBER_WARN));
    painter.line_segment([p3, p2], Stroke::new(1.0_f32, AMBER_WARN));
    painter.circle_filled(p1, 4.0, AMBER_WARN);
    painter.circle_filled(p2, 4.0, AMBER_WARN);
}

fn set_selected_easing(editor: &mut OverlayEditor, easing: Easing) {
    if let Some(selected_id) = editor.selected_layer_id {
        if let Some(layer) = editor
            .composition
            .layers
            .iter_mut()
            .find(|l| l.id == selected_id)
        {
            if let Property::Keyframes(ref mut keys) = layer.transform.position {
                for kf in keys.iter_mut() {
                    kf.easing_to_next = easing;
                }
            }
            if let Property::Keyframes(ref mut keys) = layer.transform.scale_percent {
                for kf in keys.iter_mut() {
                    kf.easing_to_next = easing;
                }
            }
            if let Property::Keyframes(ref mut keys) = layer.transform.rotation_degrees {
                for kf in keys.iter_mut() {
                    kf.easing_to_next = easing;
                }
            }
            if let Property::Keyframes(ref mut keys) = layer.transform.opacity_percent {
                for kf in keys.iter_mut() {
                    kf.easing_to_next = easing;
                }
            }
        }
    }
}
