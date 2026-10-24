//! Timeline track viewer, playhead scrubbing, keyframe manipulation,
//! timeline markers, and interactive Bezier Curve Editor.

use egui::{Align2, CornerRadius, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui};
use zeno_core::{Easing, Property};

use crate::editor::state::{OverlayEditor, TimelineMarker};
use crate::theme::{
    ACCENT, AMBER_WARN, CORNER_RADIUS, RED_LIVE, ROW_HEIGHT_LIST, SURFACE_0, SURFACE_1, SURFACE_2,
    SURFACE_3, TEXT, TEXT_MUTED,
};

/// Active mode of the bottom timeline area
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimelineMode {
    DopeSheet,
    CurveEditor,
}

/// Renders the bottom timeline component.
pub fn ui_timeline(editor: &mut OverlayEditor, mode: &mut TimelineMode, ui: &mut Ui) {
    ui.vertical(|ui| {
        // Controls Toolbar: Play/Pause, Stop, Timecode, Add Marker, Mode Switch
        ui.horizontal(|ui| {
            let is_playing = editor.clock.is_playing();
            let play_label = if is_playing { "Pause [||]" } else { "Play [>]" };
            if ui.button(play_label).clicked() {
                if is_playing {
                    editor.clock.pause();
                } else {
                    editor.clock.play();
                }
            }

            if ui.button("Stop [[]]").clicked() {
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

            if ui.button("+ Marker").clicked() {
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

/// Dope sheet track view with playhead and keyframe diamonds
fn ui_dopesheet(editor: &mut OverlayEditor, ui: &mut Ui) {
    let dur = (editor.composition.duration_seconds as f32).max(0.1);

    let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, CornerRadius::same(CORNER_RADIUS as u8), SURFACE_0);

    // Track headers on left (160px width)
    let header_width = 160.0;
    let ruler_height = 24.0;
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

    // Draw second markers on ruler
    let pixels_per_second = track_rect.width() / dur;
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
                FontId::proportional(11.0),
                TEXT_MUTED,
            );
        }
    }

    // Draw Timeline Markers
    for marker in &editor.markers {
        let mx = track_rect.min.x + (marker.time_seconds as f32) * pixels_per_second;
        if mx >= track_rect.min.x && mx <= track_rect.max.x {
            painter.line_segment(
                [Pos2::new(mx, rect.min.y), Pos2::new(mx, rect.max.y)],
                Stroke::new(1.0_f32, AMBER_WARN),
            );
            painter.text(
                Pos2::new(mx + 2.0, ruler_rect.min.y + 12.0),
                Align2::LEFT_TOP,
                &marker.name,
                FontId::proportional(10.0),
                AMBER_WARN,
            );
        }
    }

    // Draw Tracks (Topmost/Foreground layer at top)
    let track_height = ROW_HEIGHT_LIST;
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
        let in_x = (track_rect.min.x + (layer.in_point_seconds as f32) * pixels_per_second)
            .clamp(track_rect.min.x, track_rect.max.x);
        let out_x = (track_rect.min.x + (layer.out_point_seconds as f32) * pixels_per_second)
            .clamp(track_rect.min.x, track_rect.max.x);
        let span_rect =
            Rect::from_min_max(Pos2::new(in_x, y_top + 4.0), Pos2::new(out_x, y_bot - 4.0));

        painter.rect_filled(span_rect, CornerRadius::same(2), SURFACE_2);
        painter.rect_stroke(
            span_rect,
            CornerRadius::same(2),
            Stroke::new(1.0_f32, SURFACE_3),
            StrokeKind::Inside,
        );

        // Draw Keyframe Diamonds for Position and Opacity
        let draw_diamond = |painter: &egui::Painter, kf_time: f64, color: egui::Color32| {
            let kf_x = track_rect.min.x + (kf_time as f32) * pixels_per_second;
            let kf_y = y_top + track_height * 0.5;
            let d_size = 4.0;
            let points = [
                Pos2::new(kf_x, kf_y - d_size),
                Pos2::new(kf_x + d_size, kf_y),
                Pos2::new(kf_x, kf_y + d_size),
                Pos2::new(kf_x - d_size, kf_y),
            ];
            painter.add(egui::Shape::convex_polygon(
                points.to_vec(),
                color,
                Stroke::NONE,
            ));
        };

        for kf in layer.transform.position.as_keyframes() {
            draw_diamond(&painter, kf.time_seconds, ACCENT);
        }
        for kf in layer.transform.opacity_percent.as_keyframes() {
            draw_diamond(&painter, kf.time_seconds, RED_LIVE);
        }
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

    // Scrubbing Interaction
    if response.dragged() || response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            if pos.x >= track_rect.min.x && pos.x <= track_rect.max.x {
                let frac = (pos.x - track_rect.min.x) / track_rect.width();
                let target_time =
                    ((frac * dur) as f64).clamp(0.0, editor.composition.duration_seconds);
                editor.clock.seek(target_time);
            }
        }
    }
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
        // Cubic bezier easing evaluation (default standard smooth curve)
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
            if let Property::Keyframes(ref mut keys) = layer.transform.opacity_percent {
                for kf in keys.iter_mut() {
                    kf.easing_to_next = easing;
                }
            }
        }
    }
}
