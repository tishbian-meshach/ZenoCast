//! Shared interactive on-canvas transform gizmo component.
//!
//! Reused across Live Studio Preview Canvas and Overlay Editor Canvas.
//! Implements Section 4 of UX & Completeness Pass (v1.2):
//! - Selection bounding box with 8 resize handles + rotate handle
//! - Body dragging (position)
//! - Corner scaling (Shift = preserve aspect, Alt = scale from center)
//! - Edge stretching (single axis)
//! - Rotate handle (Shift = 15° snapping)
//! - Canvas hit testing (topmost wins, click empty deselects)
//! - Arrow key nudging (1px, Shift = 10px)
//! - Multi-selection (Ctrl+click, marquee drag)

use egui::{
    pos2, vec2, Color32, CornerRadius, CursorIcon, Key, Pos2, Rect, Sense, Stroke, StrokeKind, Ui,
    Vec2,
};

use crate::theme::{ACCENT, AMBER_WARN};

/// Represents an item on the canvas that can be transformed.
#[derive(Debug, Clone, PartialEq)]
pub struct TransformGizmoItem {
    pub id: String,
    pub name: String,
    /// Center X coordinate in logical canvas units (e.g. 1920x1080)
    pub center_x: f32,
    /// Center Y coordinate in logical canvas units
    pub center_y: f32,
    /// Width in logical canvas units
    pub width: f32,
    /// Height in logical canvas units
    pub height: f32,
    /// Clockwise rotation in degrees
    pub rotation_degrees: f32,
    pub locked: bool,
    pub visible: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GizmoHandle {
    Body,
    TopLeft,
    TopMid,
    TopRight,
    RightMid,
    BottomRight,
    BottomMid,
    BottomLeft,
    LeftMid,
    Rotate,
}

#[derive(Debug, Clone)]
pub struct ActiveGizmoDrag {
    pub handle: GizmoHandle,
    pub initial_pointer_logical: Vec2,
    pub initial_items: Vec<(String, TransformGizmoItem)>,
}

/// Internal state tracking active transform drags and marquee selection.
#[derive(Debug, Clone, Default)]
pub struct TransformGizmoState {
    pub active_drag: Option<ActiveGizmoDrag>,
    pub marquee_start_logical: Option<Vec2>,
    pub current_pointer_logical: Option<Vec2>,
}

/// Events produced during gizmo interaction to update editor or compositor state.
#[derive(Debug, Clone)]
pub enum TransformGizmoEvent {
    SelectOnly(String),
    ToggleSelect(String),
    SelectMultiple(Vec<String>),
    DeselectAll,
    Transformed {
        id: String,
        center_x: f32,
        center_y: f32,
        width: f32,
        height: f32,
        rotation_degrees: f32,
    },
}

impl TransformGizmoState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Renders gizmo overlays and handles mouse/keyboard events on the canvas.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_and_interact(
        &mut self,
        ui: &mut Ui,
        viewport_rect: Rect,
        canvas_size: Vec2,
        pan: Vec2,
        zoom: f32,
        items: &[TransformGizmoItem],
        selected_ids: &[String],
    ) -> Vec<TransformGizmoEvent> {
        let mut events = Vec::new();

        let response = ui.interact(
            viewport_rect,
            ui.id().with("transform_canvas_interact"),
            Sense::click_and_drag(),
        );

        // Helper functions for coordinate projections
        let screen_center = viewport_rect.center() + pan;
        let logical_to_screen = |lx: f32, ly: f32| -> Pos2 {
            let sx = screen_center.x + (lx - canvas_size.x * 0.5) * zoom;
            let sy = screen_center.y + (ly - canvas_size.y * 0.5) * zoom;
            pos2(sx, sy)
        };

        let screen_to_logical = |sp: Pos2| -> Vec2 {
            let lx = canvas_size.x * 0.5 + (sp.x - screen_center.x) / zoom;
            let ly = canvas_size.y * 0.5 + (sp.y - screen_center.y) / zoom;
            vec2(lx, ly)
        };

        let painter = ui.painter_at(viewport_rect);
        let pointer_pos = response.interact_pointer_pos();
        let pointer_logical = pointer_pos.map(screen_to_logical);
        self.current_pointer_logical = pointer_logical;

        let modifiers = ui.input(|i| i.modifiers);
        let shift_held = modifiers.shift;
        let alt_held = modifiers.alt;
        let ctrl_held = modifiers.ctrl || modifiers.command;

        // 1. Arrow key nudging when selected item exists and canvas is hovered/focused
        if (response.hovered() || response.has_focus()) && !selected_ids.is_empty() {
            let nudge_step = if shift_held { 10.0 } else { 1.0 };
            let mut nudge_x = 0.0;
            let mut nudge_y = 0.0;

            if ui.input(|i| i.key_pressed(Key::ArrowLeft)) {
                nudge_x -= nudge_step;
            }
            if ui.input(|i| i.key_pressed(Key::ArrowRight)) {
                nudge_x += nudge_step;
            }
            if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                nudge_y -= nudge_step;
            }
            if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                nudge_y += nudge_step;
            }

            if nudge_x != 0.0 || nudge_y != 0.0 {
                for id in selected_ids {
                    if let Some(item) = items.iter().find(|it| &it.id == id && !it.locked) {
                        events.push(TransformGizmoEvent::Transformed {
                            id: item.id.clone(),
                            center_x: item.center_x + nudge_x,
                            center_y: item.center_y + nudge_y,
                            width: item.width,
                            height: item.height,
                            rotation_degrees: item.rotation_degrees,
                        });
                    }
                }
            }
        }

        // 2. Determine hovered handle if an item is selected
        let mut hovered_handle: Option<(String, GizmoHandle)> = None;
        let handle_radius_screen = 6.0;

        if let Some(primary_id) = selected_ids.first() {
            if let Some(primary_item) = items.iter().find(|it| &it.id == primary_id && !it.locked) {
                let center_screen = logical_to_screen(primary_item.center_x, primary_item.center_y);
                let half_w = (primary_item.width * 0.5 * zoom).max(4.0);
                let half_h = (primary_item.height * 0.5 * zoom).max(4.0);
                let rad = primary_item.rotation_degrees.to_radians();
                let (cos_a, sin_a) = (rad.cos(), rad.sin());

                let rotate_local = |lx: f32, ly: f32| -> Pos2 {
                    let rx = lx * cos_a - ly * sin_a;
                    let ry = lx * sin_a + ly * cos_a;
                    pos2(center_screen.x + rx, center_screen.y + ry)
                };

                // Handle locations in screen space
                let h_tl = rotate_local(-half_w, -half_h);
                let h_tm = rotate_local(0.0, -half_h);
                let h_tr = rotate_local(half_w, -half_h);
                let h_rm = rotate_local(half_w, 0.0);
                let h_br = rotate_local(half_w, half_h);
                let h_bm = rotate_local(0.0, half_h);
                let h_bl = rotate_local(-half_w, half_h);
                let h_lm = rotate_local(-half_w, 0.0);
                let h_rot = rotate_local(0.0, -half_h - 24.0);

                if let Some(ptr) = pointer_pos {
                    let hit_dist = handle_radius_screen + 4.0;
                    if (ptr - h_rot).length() <= hit_dist {
                        hovered_handle = Some((primary_item.id.clone(), GizmoHandle::Rotate));
                    } else if (ptr - h_tl).length() <= hit_dist {
                        hovered_handle = Some((primary_item.id.clone(), GizmoHandle::TopLeft));
                    } else if (ptr - h_tr).length() <= hit_dist {
                        hovered_handle = Some((primary_item.id.clone(), GizmoHandle::TopRight));
                    } else if (ptr - h_br).length() <= hit_dist {
                        hovered_handle = Some((primary_item.id.clone(), GizmoHandle::BottomRight));
                    } else if (ptr - h_bl).length() <= hit_dist {
                        hovered_handle = Some((primary_item.id.clone(), GizmoHandle::BottomLeft));
                    } else if (ptr - h_tm).length() <= hit_dist {
                        hovered_handle = Some((primary_item.id.clone(), GizmoHandle::TopMid));
                    } else if (ptr - h_bm).length() <= hit_dist {
                        hovered_handle = Some((primary_item.id.clone(), GizmoHandle::BottomMid));
                    } else if (ptr - h_lm).length() <= hit_dist {
                        hovered_handle = Some((primary_item.id.clone(), GizmoHandle::LeftMid));
                    } else if (ptr - h_rm).length() <= hit_dist {
                        hovered_handle = Some((primary_item.id.clone(), GizmoHandle::RightMid));
                    } else {
                        // Check body hit
                        let diff = ptr - center_screen;
                        let unrot_x = diff.x * cos_a + diff.y * sin_a;
                        let unrot_y = -diff.x * sin_a + diff.y * cos_a;
                        if unrot_x.abs() <= half_w && unrot_y.abs() <= half_h {
                            hovered_handle = Some((primary_item.id.clone(), GizmoHandle::Body));
                        }
                    }
                }
            }
        }

        // Set appropriate cursor icon when hovering handles
        if let Some((_, handle)) = hovered_handle {
            let cursor = match handle {
                GizmoHandle::Rotate => CursorIcon::Crosshair,
                GizmoHandle::TopLeft | GizmoHandle::BottomRight => CursorIcon::ResizeNwSe,
                GizmoHandle::TopRight | GizmoHandle::BottomLeft => CursorIcon::ResizeNeSw,
                GizmoHandle::TopMid | GizmoHandle::BottomMid => CursorIcon::ResizeVertical,
                GizmoHandle::LeftMid | GizmoHandle::RightMid => CursorIcon::ResizeHorizontal,
                GizmoHandle::Body => CursorIcon::Move,
            };
            ui.ctx().set_cursor_icon(cursor);
        }

        // 3. Handle Mouse Drag Initiation
        if response.drag_started() {
            if let Some(ptr_log) = pointer_logical {
                if let Some((_, handle)) = hovered_handle {
                    // Start dragging active gizmo handle
                    let snapshot: Vec<(String, TransformGizmoItem)> = items
                        .iter()
                        .filter(|it| selected_ids.contains(&it.id) && !it.locked)
                        .map(|it| (it.id.clone(), it.clone()))
                        .collect();

                    self.active_drag = Some(ActiveGizmoDrag {
                        handle,
                        initial_pointer_logical: ptr_log,
                        initial_items: snapshot,
                    });
                } else {
                    // Test if clicked directly on another layer (topmost wins)
                    let mut clicked_topmost: Option<&TransformGizmoItem> = None;
                    for item in items.iter().rev() {
                        if !item.visible {
                            continue;
                        }
                        let dx = ptr_log.x - item.center_x;
                        let dy = ptr_log.y - item.center_y;
                        let rad = item.rotation_degrees.to_radians();
                        let unrot_x = dx * rad.cos() + dy * rad.sin();
                        let unrot_y = -dx * rad.sin() + dy * rad.cos();
                        if unrot_x.abs() <= item.width * 0.5 && unrot_y.abs() <= item.height * 0.5 {
                            clicked_topmost = Some(item);
                            break;
                        }
                    }

                    if let Some(hit_item) = clicked_topmost {
                        if ctrl_held {
                            events.push(TransformGizmoEvent::ToggleSelect(hit_item.id.clone()));
                        } else {
                            events.push(TransformGizmoEvent::SelectOnly(hit_item.id.clone()));
                        }
                        // Start dragging body immediately
                        self.active_drag = Some(ActiveGizmoDrag {
                            handle: GizmoHandle::Body,
                            initial_pointer_logical: ptr_log,
                            initial_items: vec![(hit_item.id.clone(), hit_item.clone())],
                        });
                    } else {
                        // Clicked empty canvas: start marquee drag
                        if !ctrl_held {
                            events.push(TransformGizmoEvent::DeselectAll);
                        }
                        self.marquee_start_logical = Some(ptr_log);
                    }
                }
            }
        }

        // 4. Handle Ongoing Drag
        if response.dragged() {
            if let Some(ref drag) = self.active_drag {
                if let Some(ptr_log) = pointer_logical {
                    let delta = ptr_log - drag.initial_pointer_logical;

                    for (id, initial_item) in &drag.initial_items {
                        match drag.handle {
                            GizmoHandle::Body => {
                                events.push(TransformGizmoEvent::Transformed {
                                    id: id.clone(),
                                    center_x: initial_item.center_x + delta.x,
                                    center_y: initial_item.center_y + delta.y,
                                    width: initial_item.width,
                                    height: initial_item.height,
                                    rotation_degrees: initial_item.rotation_degrees,
                                });
                            }
                            GizmoHandle::Rotate => {
                                let orig_center =
                                    vec2(initial_item.center_x, initial_item.center_y);
                                let dir = ptr_log - orig_center;
                                let angle_rad = dir.y.atan2(dir.x);
                                let mut angle_deg = angle_rad.to_degrees() + 90.0;
                                if shift_held {
                                    angle_deg = (angle_deg / 15.0).round() * 15.0;
                                }
                                events.push(TransformGizmoEvent::Transformed {
                                    id: id.clone(),
                                    center_x: initial_item.center_x,
                                    center_y: initial_item.center_y,
                                    width: initial_item.width,
                                    height: initial_item.height,
                                    rotation_degrees: angle_deg,
                                });
                            }
                            handle => {
                                // Scale & stretch handles
                                let rad = initial_item.rotation_degrees.to_radians();
                                let (cos_a, sin_a) = (rad.cos(), rad.sin());
                                let unrot_delta_x = delta.x * cos_a + delta.y * sin_a;
                                let unrot_delta_y = -delta.x * sin_a + delta.y * cos_a;

                                let mut new_w = initial_item.width;
                                let mut new_h = initial_item.height;
                                let mut shift_cx = 0.0;
                                let mut shift_cy = 0.0;

                                let scale_mult = if alt_held { 2.0 } else { 1.0 };

                                match handle {
                                    GizmoHandle::RightMid => {
                                        new_w = (initial_item.width + unrot_delta_x * scale_mult)
                                            .max(10.0);
                                        if !alt_held {
                                            shift_cx = (new_w - initial_item.width) * 0.5;
                                        }
                                    }
                                    GizmoHandle::LeftMid => {
                                        new_w = (initial_item.width - unrot_delta_x * scale_mult)
                                            .max(10.0);
                                        if !alt_held {
                                            shift_cx = -(new_w - initial_item.width) * 0.5;
                                        }
                                    }
                                    GizmoHandle::BottomMid => {
                                        new_h = (initial_item.height + unrot_delta_y * scale_mult)
                                            .max(10.0);
                                        if !alt_held {
                                            shift_cy = (new_h - initial_item.height) * 0.5;
                                        }
                                    }
                                    GizmoHandle::TopMid => {
                                        new_h = (initial_item.height - unrot_delta_y * scale_mult)
                                            .max(10.0);
                                        if !alt_held {
                                            shift_cy = -(new_h - initial_item.height) * 0.5;
                                        }
                                    }
                                    GizmoHandle::BottomRight => {
                                        new_w = (initial_item.width + unrot_delta_x * scale_mult)
                                            .max(10.0);
                                        new_h = (initial_item.height + unrot_delta_y * scale_mult)
                                            .max(10.0);
                                        if shift_held && initial_item.height > 0.0 {
                                            let aspect = initial_item.width / initial_item.height;
                                            let scale = (new_w / initial_item.width)
                                                .max(new_h / initial_item.height);
                                            new_w = initial_item.width * scale;
                                            new_h = new_w / aspect;
                                        }
                                        if !alt_held {
                                            shift_cx = (new_w - initial_item.width) * 0.5;
                                            shift_cy = (new_h - initial_item.height) * 0.5;
                                        }
                                    }
                                    GizmoHandle::TopLeft => {
                                        new_w = (initial_item.width - unrot_delta_x * scale_mult)
                                            .max(10.0);
                                        new_h = (initial_item.height - unrot_delta_y * scale_mult)
                                            .max(10.0);
                                        if shift_held && initial_item.height > 0.0 {
                                            let aspect = initial_item.width / initial_item.height;
                                            let scale = (new_w / initial_item.width)
                                                .max(new_h / initial_item.height);
                                            new_w = initial_item.width * scale;
                                            new_h = new_w / aspect;
                                        }
                                        if !alt_held {
                                            shift_cx = -(new_w - initial_item.width) * 0.5;
                                            shift_cy = -(new_h - initial_item.height) * 0.5;
                                        }
                                    }
                                    GizmoHandle::TopRight => {
                                        new_w = (initial_item.width + unrot_delta_x * scale_mult)
                                            .max(10.0);
                                        new_h = (initial_item.height - unrot_delta_y * scale_mult)
                                            .max(10.0);
                                        if shift_held && initial_item.height > 0.0 {
                                            let aspect = initial_item.width / initial_item.height;
                                            let scale = (new_w / initial_item.width)
                                                .max(new_h / initial_item.height);
                                            new_w = initial_item.width * scale;
                                            new_h = new_w / aspect;
                                        }
                                        if !alt_held {
                                            shift_cx = (new_w - initial_item.width) * 0.5;
                                            shift_cy = -(new_h - initial_item.height) * 0.5;
                                        }
                                    }
                                    GizmoHandle::BottomLeft => {
                                        new_w = (initial_item.width - unrot_delta_x * scale_mult)
                                            .max(10.0);
                                        new_h = (initial_item.height + unrot_delta_y * scale_mult)
                                            .max(10.0);
                                        if shift_held && initial_item.height > 0.0 {
                                            let aspect = initial_item.width / initial_item.height;
                                            let scale = (new_w / initial_item.width)
                                                .max(new_h / initial_item.height);
                                            new_w = initial_item.width * scale;
                                            new_h = new_w / aspect;
                                        }
                                        if !alt_held {
                                            shift_cx = -(new_w - initial_item.width) * 0.5;
                                            shift_cy = (new_h - initial_item.height) * 0.5;
                                        }
                                    }
                                    _ => {}
                                }

                                // Rotate offset delta back to world canvas space
                                let world_shift_x = shift_cx * cos_a - shift_cy * sin_a;
                                let world_shift_y = shift_cx * sin_a + shift_cy * cos_a;

                                events.push(TransformGizmoEvent::Transformed {
                                    id: id.clone(),
                                    center_x: initial_item.center_x + world_shift_x,
                                    center_y: initial_item.center_y + world_shift_y,
                                    width: new_w,
                                    height: new_h,
                                    rotation_degrees: initial_item.rotation_degrees,
                                });
                            }
                        }
                    }
                }
            }
        }

        // 5. Handle Drag Completion
        if response.drag_stopped() {
            if let (Some(m_start), Some(m_end)) = (self.marquee_start_logical, pointer_logical) {
                let min_x = m_start.x.min(m_end.x);
                let max_x = m_start.x.max(m_end.x);
                let min_y = m_start.y.min(m_end.y);
                let max_y = m_start.y.max(m_end.y);
                let m_rect = Rect::from_min_max(pos2(min_x, min_y), pos2(max_x, max_y));

                // Find all items whose bounds intersect marquee
                let mut found_ids = Vec::new();
                for item in items {
                    if item.visible && !item.locked {
                        let item_r = Rect::from_center_size(
                            pos2(item.center_x, item.center_y),
                            vec2(item.width, item.height),
                        );
                        if m_rect.intersects(item_r) {
                            found_ids.push(item.id.clone());
                        }
                    }
                }

                if !found_ids.is_empty() {
                    events.push(TransformGizmoEvent::SelectMultiple(found_ids));
                }
            }

            self.active_drag = None;
            self.marquee_start_logical = None;
        }

        // 6. Draw Marquee Box if dragging on canvas
        if let (Some(m_start), Some(m_curr)) = (self.marquee_start_logical, pointer_logical) {
            let sp1 = logical_to_screen(m_start.x, m_start.y);
            let sp2 = logical_to_screen(m_curr.x, m_curr.y);
            let screen_m_rect = Rect::from_two_pos(sp1, sp2);

            painter.rect(
                screen_m_rect,
                CornerRadius::ZERO,
                ACCENT.linear_multiply(0.15),
                Stroke::new(1.0_f32, ACCENT),
                StrokeKind::Inside,
            );
        }

        // 7. Paint Transform Gizmos for Selected Items
        for sel_id in selected_ids {
            if let Some(item) = items.iter().find(|it| &it.id == sel_id) {
                let center_screen = logical_to_screen(item.center_x, item.center_y);
                let half_w = (item.width * 0.5 * zoom).max(4.0);
                let half_h = (item.height * 0.5 * zoom).max(4.0);
                let rad = item.rotation_degrees.to_radians();
                let (cos_a, sin_a) = (rad.cos(), rad.sin());

                let rotate_local = |lx: f32, ly: f32| -> Pos2 {
                    let rx = lx * cos_a - ly * sin_a;
                    let ry = lx * sin_a + ly * cos_a;
                    pos2(center_screen.x + rx, center_screen.y + ry)
                };

                let p_tl = rotate_local(-half_w, -half_h);
                let p_tr = rotate_local(half_w, -half_h);
                let p_br = rotate_local(half_w, half_h);
                let p_bl = rotate_local(-half_w, half_h);

                // Bounding box border
                let stroke = Stroke::new(1.5_f32, ACCENT);
                painter.line_segment([p_tl, p_tr], stroke);
                painter.line_segment([p_tr, p_br], stroke);
                painter.line_segment([p_br, p_bl], stroke);
                painter.line_segment([p_bl, p_tl], stroke);

                // Rotate handle & stem
                let p_tm = rotate_local(0.0, -half_h);
                let p_rot = rotate_local(0.0, -half_h - 24.0);
                painter.line_segment([p_tm, p_rot], Stroke::new(1.0_f32, ACCENT));
                painter.circle_filled(p_rot, 5.0, ACCENT);
                painter.circle_stroke(p_rot, 5.0, Stroke::new(1.0_f32, Color32::WHITE));

                // 8 Resize handle squares
                let h_size = vec2(handle_radius_screen * 2.0, handle_radius_screen * 2.0);
                let handles = [
                    p_tl,
                    p_tm,
                    p_tr,
                    rotate_local(half_w, 0.0),
                    p_br,
                    rotate_local(0.0, half_h),
                    p_bl,
                    rotate_local(-half_w, 0.0),
                ];

                for h_pos in handles {
                    painter.rect(
                        Rect::from_center_size(h_pos, h_size),
                        CornerRadius::same(1),
                        Color32::WHITE,
                        Stroke::new(1.0_f32, ACCENT),
                        StrokeKind::Inside,
                    );
                }

                // Center pivot anchor indicator
                painter.circle_filled(center_screen, 3.5, AMBER_WARN);
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gizmo_item_creation() {
        let item = TransformGizmoItem {
            id: "src_1".into(),
            name: "Camera".into(),
            center_x: 960.0,
            center_y: 540.0,
            width: 640.0,
            height: 360.0,
            rotation_degrees: 0.0,
            locked: false,
            visible: true,
        };

        assert_eq!(item.id, "src_1");
        assert_eq!(item.center_x, 960.0);
        assert_eq!(item.width, 640.0);
        assert!(!item.locked);
        assert!(item.visible);
    }

    #[test]
    fn test_rotation_15_degree_snapping() {
        let snap_15 = |angle: f32| -> f32 { (angle / 15.0).round() * 15.0 };

        assert_eq!(snap_15(14.2), 15.0);
        assert_eq!(snap_15(1.5), 0.0);
        assert_eq!(snap_15(44.0), 45.0);
        assert_eq!(snap_15(89.0), 90.0);
        assert_eq!(snap_15(-16.0), -15.0);
    }

    #[test]
    fn test_aspect_ratio_preservation_calc() {
        let orig_w = 400.0;
        let orig_h = 200.0;
        let aspect = orig_w / orig_h; // 2.0

        let new_w_raw = 600.0_f32;
        let new_h_raw = 250.0_f32;

        let scale = f32::max(new_w_raw / orig_w, new_h_raw / orig_h); // 1.5
        let final_w = orig_w * scale;
        let final_h = final_w / aspect;

        assert_eq!(final_w, 600.0);
        assert_eq!(final_h, 300.0);
        assert_eq!(final_w / final_h, 2.0);
    }

    #[test]
    fn test_hit_test_unrotated() {
        let item = TransformGizmoItem {
            id: "src_a".into(),
            name: "Display".into(),
            center_x: 500.0,
            center_y: 500.0,
            width: 200.0,
            height: 100.0,
            rotation_degrees: 0.0,
            locked: false,
            visible: true,
        };

        // Point inside
        let pt_in = vec2(550.0, 520.0);
        let dx = pt_in.x - item.center_x;
        let dy = pt_in.y - item.center_y;
        assert!(dx.abs() <= item.width * 0.5 && dy.abs() <= item.height * 0.5);

        // Point outside
        let pt_out = vec2(650.0, 500.0);
        let dx_out = pt_out.x - item.center_x;
        let dy_out = pt_out.y - item.center_y;
        assert!(!(dx_out.abs() <= item.width * 0.5 && dy_out.abs() <= item.height * 0.5));
    }
}
