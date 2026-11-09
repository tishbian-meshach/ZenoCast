//! Lucide icon rendering engine powered by prebuilt `icondata` and vector rasterization for Zenocast.
//!
//! Visual specifications (Build Spec v1.0 Section 12, v1.4 Section 3, v1.8 Section 1):
//! - Authentic Lucide icon set provided via the `icondata` crate.
//! - High-resolution vector rasterization via `resvg`/`tiny_skia` into GPU textures.
//! - Dynamic GPU tinting using theme tokens (`TEXT`, `TEXT_MUTED`, `ACCENT`, etc.) with zero per-frame CPU overhead.
//! - Consistent 26x26 button hit targets with strictly 16x16 icon boxes.
//! - Fully DPI-aware: razor-sharp at 100%, 150%, and 200% scaling.

use egui::{
    pos2, vec2, CornerRadius, Painter, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2,
};
use usvg::TreeParsing;

use crate::theme::{
    ACCENT, BORDER_WIDTH, CORNER_RADIUS, SURFACE_2, SURFACE_3, TEXT, TEXT_MUTED, TRANSPARENT,
};

/// Enumeration of vector Lucide icons used across broadcast toolbars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl IconKind {
    /// Returns the prebuilt Lucide `Icon` from the `icondata` crate.
    pub fn icon_data(&self) -> &'static icondata::Icon {
        match self {
            IconKind::Plus => &icondata::LuPlus,
            IconKind::Minus => &icondata::LuMinus,
            IconKind::Trash => &icondata::LuTrash2,
            IconKind::Pencil => &icondata::LuPencil,
            IconKind::ChevronUp => &icondata::LuChevronUp,
            IconKind::ChevronDown => &icondata::LuChevronDown,
            IconKind::Eye => &icondata::LuEye,
            IconKind::EyeOff => &icondata::LuEyeOff,
            IconKind::Lock => &icondata::LuLock,
            IconKind::Unlock => &icondata::LuLockOpen,
            IconKind::Sliders => &icondata::LuSlidersHorizontal,
            IconKind::Filter => &icondata::LuFunnel,
            IconKind::GripVertical => &icondata::LuGripVertical,
            IconKind::RotateCcw => &icondata::LuRotateCcw,
            IconKind::Play => &icondata::LuPlay,
            IconKind::Pause => &icondata::LuPause,
            IconKind::Square => &icondata::LuSquare,
            IconKind::Type => &icondata::LuType,
            IconKind::Camera => &icondata::LuCamera,
            IconKind::Sparkles => &icondata::LuSparkles,
            IconKind::Volume2 => &icondata::LuVolume2,
            IconKind::VolumeX => &icondata::LuVolumeX,
            IconKind::Settings => &icondata::LuSettings,
            IconKind::Copy => &icondata::LuCopy,
            IconKind::X => &icondata::LuX,
        }
    }
}

/// Rasterizes an authentic Lucide vector icon from `icondata` into a high-resolution premultiplied `egui::ColorImage`.
pub fn rasterize_icon(icon: IconKind, size_px: u32) -> egui::ColorImage {
    let data = icon.icon_data();
    let view_box = data.view_box.unwrap_or("0 0 24 24");
    let stroke_width = data.stroke_width.unwrap_or("2");
    let fill = if data.fill == Some("currentColor") {
        "#ffffff"
    } else {
        data.fill.unwrap_or("none")
    };

    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="{}" fill="{}" stroke="#ffffff" stroke-width="{}" stroke-linecap="round" stroke-linejoin="round">{}</svg>"##,
        view_box, fill, stroke_width, data.data
    );

    let opt = usvg::Options::default();
    let utree = match usvg::Tree::from_str(&svg, &opt) {
        Ok(tree) => tree,
        Err(err) => {
            tracing::warn!("Failed to parse Lucide SVG for {:?}: {}", icon, err);
            return egui::ColorImage::new([size_px as usize, size_px as usize], TRANSPARENT);
        }
    };

    let rtree = resvg::Tree::from_usvg(&utree);
    let mut pixmap = match resvg::tiny_skia::Pixmap::new(size_px, size_px) {
        Some(pm) => pm,
        None => return egui::ColorImage::new([size_px as usize, size_px as usize], TRANSPARENT),
    };

    let scale_x = size_px as f32 / utree.size.width();
    let scale_y = size_px as f32 / utree.size.height();
    let transform = resvg::tiny_skia::Transform::from_scale(scale_x, scale_y);

    rtree.render(transform, &mut pixmap.as_mut());

    egui::ColorImage::from_rgba_premultiplied([size_px as usize, size_px as usize], pixmap.data())
}

/// Retrieves or loads the GPU texture for the specified `IconKind` in the current `egui::Context`.
pub fn get_or_load_icon_texture(ctx: &egui::Context, icon: IconKind) -> egui::TextureId {
    let id = egui::Id::new("zeno_icondata_tex").with(icon);
    if let Some(handle) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        handle.id()
    } else {
        let image = rasterize_icon(icon, 64);
        let handle = ctx.load_texture(
            format!("zeno_icon_{:?}", icon),
            image,
            egui::TextureOptions::LINEAR,
        );
        let tex_id = handle.id();
        ctx.data_mut(|d| d.insert_temp(id, handle));
        tex_id
    }
}

/// Paints a 16px Lucide vector line icon inside `rect` using `stroke.color` for dynamic GPU tinting.
pub fn paint_lucide_icon(painter: &Painter, icon: IconKind, rect: Rect, stroke: Stroke) {
    let texture_id = get_or_load_icon_texture(painter.ctx(), icon);
    let uv = Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0));
    painter.image(texture_id, rect, uv, stroke.color);
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
                (TRANSPARENT, TRANSPARENT)
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

        if bg_color != TRANSPARENT || border_color != TRANSPARENT {
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
    fn test_all_icon_variants_rasterize() {
        let all_icons = [
            IconKind::Plus,
            IconKind::Minus,
            IconKind::Trash,
            IconKind::Pencil,
            IconKind::ChevronUp,
            IconKind::ChevronDown,
            IconKind::Eye,
            IconKind::EyeOff,
            IconKind::Lock,
            IconKind::Unlock,
            IconKind::Sliders,
            IconKind::Filter,
            IconKind::GripVertical,
            IconKind::RotateCcw,
            IconKind::Play,
            IconKind::Pause,
            IconKind::Square,
            IconKind::Type,
            IconKind::Camera,
            IconKind::Sparkles,
            IconKind::Volume2,
            IconKind::VolumeX,
            IconKind::Settings,
            IconKind::Copy,
            IconKind::X,
        ];

        for icon in all_icons {
            let img = rasterize_icon(icon, 32);
            assert_eq!(img.width(), 32);
            assert_eq!(img.height(), 32);
            let has_visible_pixels = img.pixels.iter().any(|c| c.a() > 0);
            assert!(
                has_visible_pixels,
                "icon {:?} should render visible pixels",
                icon
            );
        }
    }

    #[test]
    fn test_icon_texture_caching() {
        let ctx = egui::Context::default();
        let tex_id1 = get_or_load_icon_texture(&ctx, IconKind::Plus);
        let tex_id2 = get_or_load_icon_texture(&ctx, IconKind::Plus);
        assert_eq!(tex_id1, tex_id2);
    }

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
