//! Strict Design Token Theme for Zenocast.
//!
//! Visual design guidelines (Section 7):
//! - Minimal, flat, professional broadcast style. No gradients, glassmorphism, or drop shadows.
//! - Flat neutral surfaces, high-contrast text, accent for selections, red for Live/Record, amber for warnings.
//! - 8px grid, 1px solid borders, corner radius 4 to 6px.
//! - Row height 28px in lists, 32px for buttons.
//!
//! CRITICAL: All UI colors are defined in this module. No other file in `zeno-ui`
//! may declare color literals.

use egui::{Color32, CornerRadius, Margin, Stroke, Style, Visuals};

// Surface Neutral Palette
pub const SURFACE_0: Color32 = Color32::from_rgb(0x0E, 0x0F, 0x11);
pub const SURFACE_1: Color32 = Color32::from_rgb(0x15, 0x17, 0x1A);
pub const SURFACE_2: Color32 = Color32::from_rgb(0x1D, 0x20, 0x24);
pub const SURFACE_3: Color32 = Color32::from_rgb(0x27, 0x2B, 0x30);

// Text Palette
pub const TEXT: Color32 = Color32::from_rgb(0xE7, 0xE9, 0xEC);
pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x8E, 0x95, 0x9D);

// Semantic & Status Palette
pub const ACCENT: Color32 = Color32::from_rgb(0x4C, 0x8D, 0xFF);
pub const RED_LIVE: Color32 = Color32::from_rgb(0xE5, 0x48, 0x4D);
pub const AMBER_WARN: Color32 = Color32::from_rgb(0xF5, 0xA5, 0x24);
pub const GREEN_OK: Color32 = Color32::from_rgb(0x46, 0xA7, 0x58);

// Spacing & Sizing Constants
pub const GRID_UNIT: f32 = 8.0;
pub const BORDER_WIDTH: f32 = 1.0;
pub const CORNER_RADIUS: f32 = 4.0;
pub const ROW_HEIGHT_LIST: f32 = 28.0;
pub const BUTTON_HEIGHT: f32 = 32.0;

/// Converts a `zeno_core::Color` into an `egui::Color32`.
pub fn color_to_egui(c: zeno_core::Color) -> Color32 {
    Color32::from_rgba_premultiplied(
        (c.r.clamp(0.0, 1.0) * 255.0) as u8,
        (c.g.clamp(0.0, 1.0) * 255.0) as u8,
        (c.b.clamp(0.0, 1.0) * 255.0) as u8,
        (c.a.clamp(0.0, 1.0) * 255.0) as u8,
    )
}

/// Applies the Zenocast broadcast production theme to an egui Context.
pub fn apply_theme(ctx: &egui::Context) {
    let mut visuals = Visuals::dark();

    // Dark flat background surfaces
    visuals.panel_fill = SURFACE_1;
    visuals.window_fill = SURFACE_1;
    visuals.faint_bg_color = SURFACE_0;
    visuals.extreme_bg_color = SURFACE_0;

    // Strict 1px solid borders
    visuals.window_stroke = Stroke::new(BORDER_WIDTH, SURFACE_3);

    // Widgets inactive
    visuals.widgets.inactive.bg_fill = SURFACE_2;
    visuals.widgets.inactive.weak_bg_fill = SURFACE_2;
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.inactive.bg_stroke = Stroke::new(BORDER_WIDTH, SURFACE_3);
    visuals.widgets.inactive.corner_radius = CornerRadius::same(CORNER_RADIUS as u8);

    // Widgets hovered
    visuals.widgets.hovered.bg_fill = SURFACE_3;
    visuals.widgets.hovered.weak_bg_fill = SURFACE_3;
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.hovered.bg_stroke = Stroke::new(BORDER_WIDTH, ACCENT);
    visuals.widgets.hovered.corner_radius = CornerRadius::same(CORNER_RADIUS as u8);

    // Widgets active / pressed
    visuals.widgets.active.bg_fill = ACCENT;
    visuals.widgets.active.weak_bg_fill = ACCENT;
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, SURFACE_0);
    visuals.widgets.active.bg_stroke = Stroke::new(BORDER_WIDTH, ACCENT);
    visuals.widgets.active.corner_radius = CornerRadius::same(CORNER_RADIUS as u8);

    // Non-interactive widgets
    visuals.widgets.noninteractive.bg_fill = SURFACE_1;
    visuals.widgets.noninteractive.weak_bg_fill = SURFACE_1;
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT_MUTED);
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(BORDER_WIDTH, SURFACE_3);
    visuals.widgets.noninteractive.corner_radius = CornerRadius::same(CORNER_RADIUS as u8);

    // Selection
    visuals.selection.bg_fill = ACCENT;
    visuals.selection.stroke = Stroke::new(1.0_f32, TEXT);

    ctx.set_visuals(visuals);

    // Spacing configuration (8px grid)
    let mut style = Style::default();
    style.spacing.item_spacing = egui::vec2(GRID_UNIT, GRID_UNIT);
    style.spacing.window_margin = Margin::same(GRID_UNIT as i8);
    style.spacing.button_padding = egui::vec2(GRID_UNIT * 1.5, GRID_UNIT * 0.75);
    style.spacing.indent = GRID_UNIT * 2.0;

    ctx.set_style(style);
}
