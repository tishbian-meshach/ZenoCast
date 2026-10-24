//! Lucide line-icon rendering engine and compact icon-button widgets for Zenocast.
//!
//! Visual specifications (Build Spec v1.0 Section 12 & v1.3 Section 1):
//! - Line-icon set (Lucide style): strictly 16px icon box with 1.5px stroke width.
//! - Flat broadcast styling: zero emojis, flat neutral surfaces, strict theme tokens.
//! - Hover tooltips for all routine actions. Text retained on high-stakes actions.

use egui::{pos2, vec2, CornerRadius, Painter, Rect, Response, Sense, Stroke, StrokeKind, Ui};

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
    RotateCcw,
    Play,
    Pause,
    Square,
    Type,
    Camera,
    Sparkles,
}

/// Paints a 16px Lucide vector line icon inside `rect` using `stroke`.
pub fn paint_lucide_icon(painter: &Painter, icon: IconKind, rect: Rect, stroke: Stroke) {
    let min = rect.min;
    let w = rect.width();
    let h = rect.height();

    let pt = |x: f32, y: f32| pos2(min.x + x * w, min.y + y * h);

    match icon {
        IconKind::Plus => {
            painter.line_segment([pt(0.50, 0.18), pt(0.50, 0.82)], stroke);
            painter.line_segment([pt(0.18, 0.50), pt(0.82, 0.50)], stroke);
        }
        IconKind::Minus => {
            painter.line_segment([pt(0.18, 0.50), pt(0.82, 0.50)], stroke);
        }
        IconKind::Trash => {
            // Top lid line and handle
            painter.line_segment([pt(0.15, 0.28), pt(0.85, 0.28)], stroke);
            painter.line_segment([pt(0.38, 0.28), pt(0.38, 0.16)], stroke);
            painter.line_segment([pt(0.38, 0.16), pt(0.62, 0.16)], stroke);
            painter.line_segment([pt(0.62, 0.16), pt(0.62, 0.28)], stroke);
            // Can body outline
            painter.line_segment([pt(0.24, 0.28), pt(0.28, 0.86)], stroke);
            painter.line_segment([pt(0.28, 0.86), pt(0.72, 0.86)], stroke);
            painter.line_segment([pt(0.72, 0.86), pt(0.76, 0.28)], stroke);
            // Vertical slats
            painter.line_segment([pt(0.42, 0.42), pt(0.42, 0.74)], stroke);
            painter.line_segment([pt(0.58, 0.42), pt(0.58, 0.74)], stroke);
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
            painter.line_segment([pt(0.20, 0.64), pt(0.50, 0.34)], stroke);
            painter.line_segment([pt(0.50, 0.34), pt(0.80, 0.64)], stroke);
        }
        IconKind::ChevronDown => {
            painter.line_segment([pt(0.20, 0.36), pt(0.50, 0.66)], stroke);
            painter.line_segment([pt(0.50, 0.66), pt(0.80, 0.36)], stroke);
        }
        IconKind::Eye => {
            // Outer eye curve points
            let left = pt(0.12, 0.50);
            let top = pt(0.50, 0.24);
            let right = pt(0.88, 0.50);
            let btm = pt(0.50, 0.76);

            painter.line_segment([left, top], stroke);
            painter.line_segment([top, right], stroke);
            painter.line_segment([right, btm], stroke);
            painter.line_segment([btm, left], stroke);

            // Pupil circle
            painter.circle_stroke(pt(0.50, 0.50), w * 0.15, stroke);
        }
        IconKind::EyeOff => {
            let left = pt(0.12, 0.50);
            let top = pt(0.50, 0.24);
            let right = pt(0.88, 0.50);
            let btm = pt(0.50, 0.76);

            painter.line_segment([left, top], stroke);
            painter.line_segment([top, right], stroke);
            painter.line_segment([right, btm], stroke);
            painter.line_segment([btm, left], stroke);

            // Diagonal slash line
            painter.line_segment([pt(0.14, 0.14), pt(0.86, 0.86)], stroke);
        }
        IconKind::Lock => {
            // Body box
            let body_rect = Rect::from_min_max(pt(0.22, 0.44), pt(0.78, 0.86));
            painter.rect_stroke(body_rect, CornerRadius::same(1), stroke, StrokeKind::Inside);
            // Shackle loop
            painter.line_segment([pt(0.34, 0.44), pt(0.34, 0.28)], stroke);
            painter.line_segment([pt(0.34, 0.28), pt(0.66, 0.28)], stroke);
            painter.line_segment([pt(0.66, 0.28), pt(0.66, 0.44)], stroke);
            // Keyhole
            painter.circle_filled(pt(0.50, 0.62), 1.5, stroke.color);
        }
        IconKind::Unlock => {
            let body_rect = Rect::from_min_max(pt(0.22, 0.44), pt(0.78, 0.86));
            painter.rect_stroke(body_rect, CornerRadius::same(1), stroke, StrokeKind::Inside);
            // Open shackle loop (detached right leg)
            painter.line_segment([pt(0.34, 0.44), pt(0.34, 0.22)], stroke);
            painter.line_segment([pt(0.34, 0.22), pt(0.66, 0.22)], stroke);
            painter.line_segment([pt(0.66, 0.22), pt(0.66, 0.34)], stroke);
            painter.circle_filled(pt(0.50, 0.62), 1.5, stroke.color);
        }
        IconKind::Sliders => {
            // Track 1
            painter.line_segment([pt(0.15, 0.32), pt(0.85, 0.32)], stroke);
            painter.line_segment([pt(0.36, 0.20), pt(0.36, 0.44)], stroke);
            // Track 2
            painter.line_segment([pt(0.15, 0.68), pt(0.85, 0.68)], stroke);
            painter.line_segment([pt(0.64, 0.56), pt(0.64, 0.80)], stroke);
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
            let p1 = pt(0.26, 0.20);
            let p2 = pt(0.80, 0.50);
            let p3 = pt(0.26, 0.80);
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
            // Top T crossbar
            painter.line_segment([pt(0.18, 0.22), pt(0.82, 0.22)], stroke);
            // Vertical stem
            painter.line_segment([pt(0.50, 0.22), pt(0.50, 0.82)], stroke);
            // Bottom base serif
            painter.line_segment([pt(0.36, 0.82), pt(0.64, 0.82)], stroke);
        }
        IconKind::Camera => {
            // Body
            let body = Rect::from_min_max(pt(0.15, 0.34), pt(0.85, 0.84));
            painter.rect_stroke(body, CornerRadius::same(2), stroke, StrokeKind::Inside);
            // Ridge / pentaprism
            painter.line_segment([pt(0.36, 0.34), pt(0.42, 0.20)], stroke);
            painter.line_segment([pt(0.42, 0.20), pt(0.58, 0.20)], stroke);
            painter.line_segment([pt(0.58, 0.20), pt(0.64, 0.34)], stroke);
            // Lens circle
            painter.circle_stroke(pt(0.50, 0.59), w * 0.16, stroke);
        }
        IconKind::Sparkles => {
            // 4-point central star
            painter.line_segment([pt(0.45, 0.15), pt(0.45, 0.85)], stroke);
            painter.line_segment([pt(0.10, 0.50), pt(0.80, 0.50)], stroke);
            // Small diagonal rays
            painter.line_segment([pt(0.26, 0.31), pt(0.64, 0.69)], stroke);
            painter.line_segment([pt(0.26, 0.69), pt(0.64, 0.31)], stroke);
        }
    }
}

/// Renders a compact, interactive 26x26 toolbar icon button with hover tooltip.
pub fn icon_button(ui: &mut Ui, icon: IconKind, tooltip: &str) -> Response {
    icon_button_enabled(ui, true, icon, tooltip)
}

/// Renders an icon button with enabled/disabled state and hover tooltip.
pub fn icon_button_enabled(ui: &mut Ui, enabled: bool, icon: IconKind, tooltip: &str) -> Response {
    let size = vec2(26.0, 26.0);
    let (rect, response) = ui.allocate_exact_size(
        size,
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );

    let is_down = enabled && response.is_pointer_button_down_on();
    let is_hovered = enabled && response.hovered();

    let bg_color = if is_down { SURFACE_3 } else { SURFACE_2 };

    let border_color = if is_down || is_hovered {
        ACCENT
    } else {
        SURFACE_3
    };

    let stroke_color = if !enabled {
        TEXT_MUTED
    } else if is_down {
        ACCENT
    } else {
        TEXT
    };

    ui.painter().rect(
        rect,
        CornerRadius::same(CORNER_RADIUS as u8),
        bg_color,
        Stroke::new(BORDER_WIDTH, border_color),
        StrokeKind::Inside,
    );

    let icon_rect = Rect::from_center_size(rect.center(), vec2(16.0, 16.0));
    paint_lucide_icon(
        ui.painter(),
        icon,
        icon_rect,
        Stroke::new(1.5_f32, stroke_color),
    );

    response.on_hover_text(tooltip)
}

/// Renders an icon toggle button (e.g. visibility or lock) with alternating icons and hover tooltip.
pub fn icon_toggle_button(
    ui: &mut Ui,
    selected: bool,
    icon_selected: IconKind,
    icon_unselected: IconKind,
    tooltip: &str,
) -> Response {
    let size = vec2(22.0, 22.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());

    let is_down = response.is_pointer_button_down_on();
    let is_hovered = response.hovered();

    let bg_color = if selected {
        SURFACE_3
    } else if is_hovered {
        SURFACE_2
    } else {
        crate::theme::TRANSPARENT
    };

    let border_color = if is_down || is_hovered {
        ACCENT
    } else if selected {
        SURFACE_3
    } else {
        crate::theme::TRANSPARENT
    };

    let stroke_color = if selected || is_hovered {
        TEXT
    } else {
        TEXT_MUTED
    };

    ui.painter().rect(
        rect,
        CornerRadius::same(CORNER_RADIUS as u8),
        bg_color,
        Stroke::new(BORDER_WIDTH, border_color),
        StrokeKind::Inside,
    );

    let active_icon = if selected {
        icon_selected
    } else {
        icon_unselected
    };
    let icon_rect = Rect::from_center_size(rect.center(), vec2(14.0, 14.0));
    paint_lucide_icon(
        ui.painter(),
        active_icon,
        icon_rect,
        Stroke::new(1.5_f32, stroke_color),
    );

    response.on_hover_text(tooltip)
}
