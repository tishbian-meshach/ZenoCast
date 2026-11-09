//! Lucide line-icon rendering engine and compact icon-button widgets for Zenocast.
//!
//! Visual specifications (Build Spec v1.0 Section 12 & v1.4 Section 3):
//! - Line-icon set (Lucide style): strictly 16px icon box with 1.5px stroke width.
//! - Flat broadcast styling: zero emojis, flat neutral surfaces, strict theme tokens.
//! - Hover tooltips for all routine actions. Text retained on high-stakes actions.
//! - DPI-aware: direct vector rendering preserves razor-sharp edges at 100%, 150%, and 200% scaling.

use egui::{
    pos2, vec2, CornerRadius, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2,
};

use crate::theme::{ACCENT, BORDER_WIDTH, CORNER_RADIUS, SURFACE_2, SURFACE_3, TEXT, TEXT_MUTED};

/// Enumeration of vector Lucide icons used across broadcast toolbars.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
    Plus,
    Minus,
    Trash,
    Pencil,
    ChevronUp,
    ChevronDown,
    Eye,
    EyeOff,
    Lock,
    Unlock,
    Sliders,
    Filter,
    GripVertical,
    RotateCcw,
    Play,
    Pause,
    Square,
    Type,
    Camera,
    Sparkles,
    Volume2,
    VolumeX,
    Settings,
    Copy,
    X,
}

/// Paints a 16px Lucide vector line icon inside `rect` using `stroke`.
pub fn paint_lucide_icon(painter: &Painter, icon: IconKind, rect: Rect, stroke: Stroke) {
    let min = rect.min;
    let w = rect.width();
    let h = rect.height();

    let pt = |x: f32, y: f32| pos2(min.x + x * w, min.y + y * h);

    match icon {
        IconKind::Plus => {
            painter.line_segment([pt(0.50, 0.16), pt(0.50, 0.84)], stroke);
            painter.line_segment([pt(0.16, 0.50), pt(0.84, 0.50)], stroke);
        }
        IconKind::Minus => {
            painter.line_segment([pt(0.16, 0.50), pt(0.84, 0.50)], stroke);
        }
        IconKind::Trash => {
            // Top lid line and handle
            painter.line_segment([pt(0.14, 0.26), pt(0.86, 0.26)], stroke);
            painter.line_segment([pt(0.36, 0.26), pt(0.36, 0.15)], stroke);
            painter.line_segment([pt(0.36, 0.15), pt(0.64, 0.15)], stroke);
            painter.line_segment([pt(0.64, 0.15), pt(0.64, 0.26)], stroke);
            // Can body outline
            painter.line_segment([pt(0.24, 0.26), pt(0.28, 0.86)], stroke);
            painter.line_segment([pt(0.28, 0.86), pt(0.72, 0.86)], stroke);
            painter.line_segment([pt(0.72, 0.86), pt(0.76, 0.26)], stroke);
            // Vertical slats
            painter.line_segment([pt(0.42, 0.40), pt(0.42, 0.72)], stroke);
            painter.line_segment([pt(0.58, 0.40), pt(0.58, 0.72)], stroke);
        }
        IconKind::Pencil => {
            // Angled pencil body and tip
            let p1 = pt(0.74, 0.16);
            let p2 = pt(0.84, 0.26);
            let p3 = pt(0.36, 0.74);
            let p4 = pt(0.18, 0.82);
            let p5 = pt(0.26, 0.64);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.line_segment([p3, p4], stroke);
            painter.line_segment([p4, p5], stroke);
            painter.line_segment([p5, p1], stroke);
            painter.line_segment([pt(0.62, 0.28), pt(0.72, 0.38)], stroke);
        }
        IconKind::ChevronUp => {
            painter.line_segment([pt(0.22, 0.62), pt(0.50, 0.36)], stroke);
            painter.line_segment([pt(0.50, 0.36), pt(0.78, 0.62)], stroke);
        }
        IconKind::ChevronDown => {
            painter.line_segment([pt(0.22, 0.38), pt(0.50, 0.64)], stroke);
            painter.line_segment([pt(0.50, 0.64), pt(0.78, 0.38)], stroke);
        }
        IconKind::Eye => {
            // Upper eye curve: smooth 5-point spline
            let top_spline = [
                pt(0.08, 0.50),
                pt(0.26, 0.30),
                pt(0.50, 0.22),
                pt(0.74, 0.30),
                pt(0.92, 0.50),
            ];
            for i in 0..4 {
                painter.line_segment([top_spline[i], top_spline[i + 1]], stroke);
            }
            // Lower eye curve: smooth 5-point spline
            let btm_spline = [
                pt(0.08, 0.50),
                pt(0.26, 0.70),
                pt(0.50, 0.78),
                pt(0.74, 0.70),
                pt(0.92, 0.50),
            ];
            for i in 0..4 {
                painter.line_segment([btm_spline[i], btm_spline[i + 1]], stroke);
            }
            // Central pupil circle
            painter.circle_stroke(pt(0.50, 0.50), w * 0.16, stroke);
        }
        IconKind::EyeOff => {
            // Upper and lower partial eye curves
            let top_spline = [
                pt(0.08, 0.50),
                pt(0.26, 0.30),
                pt(0.50, 0.22),
                pt(0.74, 0.30),
                pt(0.92, 0.50),
            ];
            for i in 0..4 {
                painter.line_segment([top_spline[i], top_spline[i + 1]], stroke);
            }
            let btm_spline = [
                pt(0.08, 0.50),
                pt(0.26, 0.70),
                pt(0.50, 0.78),
                pt(0.74, 0.70),
                pt(0.92, 0.50),
            ];
            for i in 0..4 {
                painter.line_segment([btm_spline[i], btm_spline[i + 1]], stroke);
            }
            painter.circle_stroke(pt(0.50, 0.50), w * 0.16, stroke);
            // Diagonal slash across eye
            painter.line_segment([pt(0.12, 0.12), pt(0.88, 0.88)], stroke);
        }
        IconKind::Lock => {
            // Body box with rounded corners
            let body_rect = Rect::from_min_max(pt(0.16, 0.44), pt(0.84, 0.88));
            painter.rect_stroke(body_rect, CornerRadius::same(2), stroke, StrokeKind::Inside);
            // Rounded shackle loop
            let shackle_pts = [
                pt(0.32, 0.44),
                pt(0.32, 0.28),
                pt(0.38, 0.18),
                pt(0.50, 0.14),
                pt(0.62, 0.18),
                pt(0.68, 0.28),
                pt(0.68, 0.44),
            ];
            for i in 0..6 {
                painter.line_segment([shackle_pts[i], shackle_pts[i + 1]], stroke);
            }
            // Keyhole
            painter.circle_filled(pt(0.50, 0.62), 1.3, stroke.color);
            painter.line_segment([pt(0.50, 0.62), pt(0.50, 0.72)], stroke);
        }
        IconKind::Unlock => {
            let body_rect = Rect::from_min_max(pt(0.16, 0.44), pt(0.84, 0.88));
            painter.rect_stroke(body_rect, CornerRadius::same(2), stroke, StrokeKind::Inside);
            // Detached open shackle loop
            let shackle_pts = [
                pt(0.32, 0.44),
                pt(0.32, 0.26),
                pt(0.38, 0.16),
                pt(0.50, 0.12),
                pt(0.62, 0.16),
                pt(0.68, 0.24),
                pt(0.68, 0.32),
            ];
            for i in 0..6 {
                painter.line_segment([shackle_pts[i], shackle_pts[i + 1]], stroke);
            }
            painter.circle_filled(pt(0.50, 0.62), 1.3, stroke.color);
            painter.line_segment([pt(0.50, 0.62), pt(0.50, 0.72)], stroke);
        }
        IconKind::Sliders => {
            // Track 1
            painter.line_segment([pt(0.14, 0.34), pt(0.86, 0.34)], stroke);
            painter.line_segment([pt(0.36, 0.20), pt(0.36, 0.48)], stroke);
            // Track 2
            painter.line_segment([pt(0.14, 0.66), pt(0.86, 0.66)], stroke);
            painter.line_segment([pt(0.64, 0.52), pt(0.64, 0.80)], stroke);
        }
        IconKind::Filter => {
            // Lucide Funnel
            let pts = [
                pt(0.12, 0.18),
                pt(0.88, 0.18),
                pt(0.58, 0.52),
                pt(0.58, 0.82),
                pt(0.42, 0.74),
                pt(0.42, 0.52),
            ];
            for i in 0..5 {
                painter.line_segment([pts[i], pts[i + 1]], stroke);
            }
            painter.line_segment([pts[5], pts[0]], stroke);
        }
        IconKind::GripVertical => {
            // Lucide 6-dot vertical drag grip
            let r = 1.3_f32;
            let col1 = 0.36;
            let col2 = 0.64;
            let row1 = 0.24;
            let row2 = 0.50;
            let row3 = 0.76;

            painter.circle_filled(pt(col1, row1), r, stroke.color);
            painter.circle_filled(pt(col2, row1), r, stroke.color);
            painter.circle_filled(pt(col1, row2), r, stroke.color);
            painter.circle_filled(pt(col2, row2), r, stroke.color);
            painter.circle_filled(pt(col1, row3), r, stroke.color);
            painter.circle_filled(pt(col2, row3), r, stroke.color);
        }
        IconKind::RotateCcw => {
            // Circular arc segments
            painter.line_segment([pt(0.24, 0.50), pt(0.32, 0.76)], stroke);
            painter.line_segment([pt(0.32, 0.76), pt(0.68, 0.76)], stroke);
            painter.line_segment([pt(0.68, 0.76), pt(0.80, 0.50)], stroke);
            painter.line_segment([pt(0.80, 0.50), pt(0.68, 0.24)], stroke);
            painter.line_segment([pt(0.68, 0.24), pt(0.40, 0.24)], stroke);
            // Arrow head
            painter.line_segment([pt(0.52, 0.14), pt(0.38, 0.24)], stroke);
            painter.line_segment([pt(0.52, 0.34), pt(0.38, 0.24)], stroke);
        }
        IconKind::Play => {
            let p1 = pt(0.28, 0.20);
            let p2 = pt(0.80, 0.50);
            let p3 = pt(0.28, 0.80);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.line_segment([p3, p1], stroke);
        }
        IconKind::Pause => {
            painter.line_segment([pt(0.32, 0.20), pt(0.32, 0.80)], stroke);
            painter.line_segment([pt(0.68, 0.20), pt(0.68, 0.80)], stroke);
        }
        IconKind::Square => {
            let sq_rect = Rect::from_min_max(pt(0.20, 0.20), pt(0.80, 0.80));
            painter.rect_stroke(sq_rect, CornerRadius::same(1), stroke, StrokeKind::Inside);
        }
        IconKind::Type => {
            painter.line_segment([pt(0.18, 0.22), pt(0.82, 0.22)], stroke);
            painter.line_segment([pt(0.50, 0.22), pt(0.50, 0.82)], stroke);
            painter.line_segment([pt(0.36, 0.82), pt(0.64, 0.82)], stroke);
        }
        IconKind::Camera => {
            let body = Rect::from_min_max(pt(0.15, 0.34), pt(0.85, 0.84));
            painter.rect_stroke(body, CornerRadius::same(2), stroke, StrokeKind::Inside);
            painter.line_segment([pt(0.36, 0.34), pt(0.42, 0.20)], stroke);
            painter.line_segment([pt(0.42, 0.20), pt(0.58, 0.20)], stroke);
            painter.line_segment([pt(0.58, 0.20), pt(0.64, 0.34)], stroke);
            painter.circle_stroke(pt(0.50, 0.59), w * 0.16, stroke);
        }
        IconKind::Sparkles => {
            painter.line_segment([pt(0.50, 0.14), pt(0.50, 0.86)], stroke);
            painter.line_segment([pt(0.14, 0.50), pt(0.86, 0.50)], stroke);
            painter.line_segment([pt(0.28, 0.28), pt(0.72, 0.72)], stroke);
            painter.line_segment([pt(0.28, 0.72), pt(0.72, 0.28)], stroke);
        }
        IconKind::Volume2 => {
            // Speaker cone
            let p1 = pt(0.44, 0.20);
            let p2 = pt(0.24, 0.36);
            let p3 = pt(0.12, 0.36);
            let p4 = pt(0.12, 0.64);
            let p5 = pt(0.24, 0.64);
            let p6 = pt(0.44, 0.80);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.line_segment([p3, p4], stroke);
            painter.line_segment([p4, p5], stroke);
            painter.line_segment([p5, p6], stroke);
            painter.line_segment([p6, p1], stroke);
            // Soundwaves
            painter.line_segment([pt(0.60, 0.36), pt(0.64, 0.50)], stroke);
            painter.line_segment([pt(0.64, 0.50), pt(0.60, 0.64)], stroke);
            painter.line_segment([pt(0.74, 0.26), pt(0.82, 0.50)], stroke);
            painter.line_segment([pt(0.82, 0.50), pt(0.74, 0.74)], stroke);
        }
        IconKind::VolumeX => {
            let p1 = pt(0.44, 0.20);
            let p2 = pt(0.24, 0.36);
            let p3 = pt(0.12, 0.36);
            let p4 = pt(0.12, 0.64);
            let p5 = pt(0.24, 0.64);
            let p6 = pt(0.44, 0.80);
            painter.line_segment([p1, p2], stroke);
            painter.line_segment([p2, p3], stroke);
            painter.line_segment([p3, p4], stroke);
            painter.line_segment([p4, p5], stroke);
            painter.line_segment([p5, p6], stroke);
            painter.line_segment([p6, p1], stroke);
            // Mute cross
            painter.line_segment([pt(0.62, 0.36), pt(0.84, 0.64)], stroke);
            painter.line_segment([pt(0.84, 0.36), pt(0.62, 0.64)], stroke);
        }
        IconKind::Settings => {
            // Gear center circle
            painter.circle_stroke(pt(0.50, 0.50), w * 0.22, stroke);
            // Gear teeth
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::PI / 4.0;
                let c = angle.cos();
                let s = angle.sin();
                let p_in = pt(0.50 + c * 0.28, 0.50 + s * 0.28);
                let p_out = pt(0.50 + c * 0.42, 0.50 + s * 0.42);
                painter.line_segment([p_in, p_out], stroke);
            }
        }
        IconKind::Copy => {
            // Background box
            let bg_box = Rect::from_min_max(pt(0.32, 0.16), pt(0.84, 0.68));
            painter.rect_stroke(bg_box, CornerRadius::same(1), stroke, StrokeKind::Inside);
            // Foreground box
            let fg_box = Rect::from_min_max(pt(0.16, 0.32), pt(0.68, 0.84));
            painter.rect_stroke(fg_box, CornerRadius::same(1), stroke, StrokeKind::Inside);
        }
        IconKind::X => {
            painter.line_segment([pt(0.24, 0.24), pt(0.76, 0.76)], stroke);
            painter.line_segment([pt(0.76, 0.24), pt(0.24, 0.76)], stroke);
        }
    }
}

/// Unified, interactive icon-button widget for all Zenocast broadcast panels.
#[derive(Debug, Clone)]
pub struct IconButton {
    icon: IconKind,
    tooltip: Option<String>,
    enabled: bool,
    selected: bool,
    size: egui::Vec2,
    icon_size: egui::Vec2,
    ghost: bool,
    sense: Sense,
}

impl IconButton {
    /// Creates a new 26x26 toolbar icon button with a 16x16 icon box.
    pub fn new(icon: IconKind) -> Self {
        Self {
            icon,
            tooltip: None,
            enabled: true,
            selected: false,
            size: vec2(26.0, 26.0),
            icon_size: vec2(16.0, 16.0),
            ghost: false,
            sense: Sense::click(),
        }
    }

    /// Attaches an informative hover tooltip.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Sets whether the control is interactable.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Sets toggle active state.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Sets custom hit target dimensions.
    pub fn size(mut self, size: egui::Vec2) -> Self {
        self.size = size;
        self
    }

    /// Sets custom icon box dimensions (defaults to strictly 16x16).
    pub fn icon_size(mut self, icon_size: egui::Vec2) -> Self {
        self.icon_size = icon_size;
        self
    }

    /// Sets ghost mode (no background or border at rest, e.g. for row items like eye/lock).
    pub fn ghost(mut self, ghost: bool) -> Self {
        self.ghost = ghost;
        self
    }

    /// Sets interaction sense (e.g. `Sense::click()`, `Sense::click_and_drag()`).
    pub fn sense(mut self, sense: Sense) -> Self {
        self.sense = sense;
        self
    }

    /// Renders the button and returns the egui response.
    pub fn show(self, ui: &mut Ui) -> Response {
        let (rect, response) = ui.allocate_exact_size(
            self.size,
            if self.enabled {
                self.sense
            } else {
                Sense::hover()
            },
        );

        let is_down = self.enabled && response.is_pointer_button_down_on();
        let is_hovered = self.enabled && response.hovered();

        let (bg_color, border_color) = if self.ghost {
            if self.selected {
                (SURFACE_3, ACCENT)
            } else if is_hovered {
                (SURFACE_2, ACCENT)
            } else {
                (crate::theme::TRANSPARENT, crate::theme::TRANSPARENT)
            }
        } else {
            let bg = if is_down || self.selected {
                SURFACE_3
            } else {
                SURFACE_2
            };
            let border = if is_down || is_hovered || self.selected {
                ACCENT
            } else {
                SURFACE_3
            };
            (bg, border)
        };

        let stroke_color = if !self.enabled {
            TEXT_MUTED.linear_multiply(0.6)
        } else if self.selected {
            ACCENT
        } else if is_down || is_hovered {
            TEXT
        } else {
            TEXT_MUTED
        };

        if bg_color != crate::theme::TRANSPARENT || border_color != crate::theme::TRANSPARENT {
            ui.painter().rect(
                rect,
                CornerRadius::same(CORNER_RADIUS as u8),
                bg_color,
                Stroke::new(BORDER_WIDTH, border_color),
                StrokeKind::Inside,
            );
        }

        let icon_rect = Rect::from_center_size(rect.center(), self.icon_size);
        paint_lucide_icon(
            ui.painter(),
            self.icon,
            icon_rect,
            Stroke::new(1.5_f32, stroke_color),
        );

        if let Some(ref tip) = self.tooltip {
            show_aligned_tooltip(ui, &response, tip);
        }
        response
    }
}

/// Pure geometric calculation for tooltip placement relative to a trigger control (Build Spec v1.8 Section 3).
///
/// Features:
/// - Places tooltip directly below trigger with a 4px gap, avoiding trigger occlusion.
/// - Flips directly above trigger if bottom screen boundary would overflow.
/// - Clamps horizontally within screen viewport bounds.
pub fn compute_tooltip_position(
    trigger_rect: Rect,
    screen_rect: Rect,
    tooltip_size: Vec2,
    gap: f32,
) -> Pos2 {
    let y = if trigger_rect.bottom() + gap + tooltip_size.y <= screen_rect.bottom() - 4.0 {
        trigger_rect.bottom() + gap
    } else {
        (trigger_rect.top() - gap - tooltip_size.y).max(screen_rect.top() + 4.0)
    };

    let x = (trigger_rect.left()).clamp(
        screen_rect.left() + 4.0,
        (screen_rect.right() - tooltip_size.x - 4.0).max(screen_rect.left() + 4.0),
    );

    pos2(x, y)
}

/// Positions and renders a tooltip cleanly relative to the trigger control (Build Spec v1.8 Section 3).
///
/// Features:
/// - Places tooltip directly below trigger with a 4px gap, avoiding trigger occlusion.
/// - Flips directly above trigger if bottom screen boundary would overflow.
/// - Clamps horizontally within screen viewport bounds.
pub fn show_aligned_tooltip(ui: &Ui, response: &Response, tooltip: &str) {
    if !response.hovered() {
        return;
    }
    let rect = response.rect;
    let screen = ui.ctx().screen_rect();
    let gap = 4.0_f32;

    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let galley = ui
        .painter()
        .layout_no_wrap(tooltip.to_string(), font_id, TEXT);
    let padding_x = 16.0_f32;
    let padding_y = 10.0_f32;
    let tip_w = galley.size().x + padding_x;
    let tip_h = galley.size().y + padding_y;

    let tip_pos = compute_tooltip_position(rect, screen, vec2(tip_w, tip_h), gap);
    let layer_id = egui::LayerId::new(egui::Order::Tooltip, response.id.with("tooltip_layer"));
    egui::show_tooltip_at(
        ui.ctx(),
        layer_id,
        response.id.with("positioned_tip"),
        tip_pos,
        |ui| {
            ui.label(tooltip);
        },
    );
}

impl egui::Widget for IconButton {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui)
    }
}

/// Convenience helper: renders a compact 26x26 toolbar icon button with hover tooltip.
pub fn icon_button(ui: &mut Ui, icon: IconKind, tooltip: &str) -> Response {
    IconButton::new(icon).tooltip(tooltip).show(ui)
}

/// Convenience helper: renders an icon button with enabled/disabled state and hover tooltip.
pub fn icon_button_enabled(ui: &mut Ui, enabled: bool, icon: IconKind, tooltip: &str) -> Response {
    IconButton::new(icon)
        .enabled(enabled)
        .tooltip(tooltip)
        .show(ui)
}

/// Convenience helper: renders an icon toggle button (e.g. visibility or lock) with alternating icons and hover tooltip.
pub fn icon_toggle_button(
    ui: &mut Ui,
    selected: bool,
    icon_selected: IconKind,
    icon_unselected: IconKind,
    tooltip: &str,
) -> Response {
    let icon = if selected {
        icon_selected
    } else {
        icon_unselected
    };
    IconButton::new(icon)
        .selected(selected)
        .ghost(true)
        .size(vec2(22.0, 22.0))
        .icon_size(vec2(16.0, 16.0))
        .tooltip(tooltip)
        .show(ui)
}

pub fn grip_handle(ui: &mut Ui, tooltip: &str) -> Response {
    IconButton::new(IconKind::GripVertical)
        .ghost(true)
        .size(vec2(16.0, 22.0))
        .icon_size(vec2(16.0, 16.0))
        .sense(Sense::click_and_drag())
        .tooltip(tooltip)
        .show(ui)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_tooltip_position_below_trigger() {
        let trigger = Rect::from_min_size(pos2(100.0, 100.0), vec2(32.0, 32.0));
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1920.0, 1080.0));
        let tip_size = vec2(120.0, 24.0);
        let gap = 4.0;

        let pos = compute_tooltip_position(trigger, screen, tip_size, gap);
        assert_eq!(pos.y, 136.0);
        assert_eq!(pos.x, 100.0);
        let tip_rect = Rect::from_min_size(pos, tip_size);
        assert!(!tip_rect.intersects(trigger));
    }

    #[test]
    fn test_compute_tooltip_position_flips_above_at_screen_bottom() {
        let trigger = Rect::from_min_size(pos2(100.0, 1050.0), vec2(32.0, 32.0));
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1920.0, 1080.0));
        let tip_size = vec2(120.0, 30.0);
        let gap = 4.0;

        let pos = compute_tooltip_position(trigger, screen, tip_size, gap);
        assert_eq!(pos.y, 1016.0);
        assert_eq!(pos.x, 100.0);
        let tip_rect = Rect::from_min_size(pos, tip_size);
        assert!(!tip_rect.intersects(trigger));
    }

    #[test]
    fn test_compute_tooltip_position_horizontal_clamping() {
        let trigger_right = Rect::from_min_size(pos2(1900.0, 100.0), vec2(32.0, 32.0));
        let screen = Rect::from_min_size(pos2(0.0, 0.0), vec2(1920.0, 1080.0));
        let tip_size = vec2(120.0, 24.0);
        let gap = 4.0;

        let pos_right = compute_tooltip_position(trigger_right, screen, tip_size, gap);
        assert!(pos_right.x + tip_size.x <= screen.right() - 4.0);

        let trigger_left = Rect::from_min_size(pos2(1.0, 100.0), vec2(32.0, 32.0));
        let pos_left = compute_tooltip_position(trigger_left, screen, tip_size, gap);
        assert_eq!(pos_left.x, 4.0);
    }
}
