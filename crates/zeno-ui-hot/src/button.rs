//! Unified Button component for Zenocast broadcast UI (Build Spec v1.8 Section 2).
//!
//! Provides strict hierarchy, variants, states, toggle active indicators,
//! and accessibility focus rings according to broadcast design tokens.

use egui::{pos2, vec2, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetText};

use crate::icon::{paint_lucide_icon, IconKind};
use crate::theme::{
    ACCENT, BORDER_WIDTH, BUTTON_HEIGHT, BUTTON_PADDING_H, CORNER_RADIUS, RED_LIVE, SURFACE_1,
    SURFACE_2, SURFACE_3, TEXT, TEXT_MUTED, TINT_NONE, TRANSPARENT,
};

/// Semantic visual hierarchy variants for Zenocast buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    /// Primary high-stakes action (filled accent blue `#4C8DFF`, white text).
    Primary,
    /// Standard routine action (neutral `#272B30` surface token, muted border).
    #[default]
    Secondary,
    /// Critical or destructive action (filled or outlined with `#E5484D` red, white text).
    Danger,
    /// Low-emphasis action (transparent background/border at rest).
    Ghost,
}

/// Unified button component with consistent styling, states, and sizing.
#[derive(Clone)]
pub struct Button {
    text: WidgetText,
    variant: ButtonVariant,
    selected: bool,
    enabled: bool,
    tooltip: Option<String>,
    height: f32,
    width: Option<f32>,
    full_width: bool,
    icon: Option<IconKind>,
}

impl Button {
    /// Creates a new button with secondary variant by default.
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            variant: ButtonVariant::Secondary,
            selected: false,
            enabled: true,
            tooltip: None,
            height: BUTTON_HEIGHT,
            width: None,
            full_width: false,
            icon: None,
        }
    }

    pub fn get_variant(&self) -> ButtonVariant {
        self.variant
    }

    pub fn get_height(&self) -> f32 {
        self.height
    }

    pub fn is_selected(&self) -> bool {
        self.selected
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Sets the button variant.
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Convenience builder for Primary variant (`#4C8DFF` accent fill, white text).
    pub fn primary(mut self) -> Self {
        self.variant = ButtonVariant::Primary;
        self
    }

    /// Convenience builder for Secondary variant (`#272B30` surface fill).
    pub fn secondary(mut self) -> Self {
        self.variant = ButtonVariant::Secondary;
        self
    }

    /// Convenience builder for Danger variant (`#E5484D` live red fill).
    pub fn danger(mut self) -> Self {
        self.variant = ButtonVariant::Danger;
        self
    }

    /// Convenience builder for Ghost variant (transparent at rest).
    pub fn ghost(mut self) -> Self {
        self.variant = ButtonVariant::Ghost;
        self
    }

    /// Sets toggle active state. When active, renders persistent accent indicator.
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Sets whether the button is interactable.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Attaches an informative hover tooltip.
    pub fn tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    /// Sets explicit height (defaults to standard 32px token).
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Sets explicit width.
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Expands button to fill available horizontal width.
    pub fn full_width(mut self) -> Self {
        self.full_width = true;
        self
    }

    /// Attaches an optional leading 16px Lucide vector icon.
    pub fn icon(mut self, icon: IconKind) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Renders the button and returns the egui response.
    pub fn show(self, ui: &mut Ui) -> Response {
        let text_layout = self.text.into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            egui::TextStyle::Button,
        );

        let icon_width = if self.icon.is_some() { 16.0 + 8.0 } else { 0.0 };
        let content_width = text_layout.size().x + icon_width;
        let natural_width = content_width + BUTTON_PADDING_H * 2.0;

        let desired_width = if self.full_width {
            ui.available_width()
        } else if let Some(w) = self.width {
            w
        } else {
            natural_width
        };

        let desired_size = vec2(desired_width, self.height);
        let (rect, response) = ui.allocate_exact_size(
            desired_size,
            if self.enabled {
                Sense::click()
            } else {
                Sense::hover()
            },
        );

        let is_down = self.enabled && response.is_pointer_button_down_on();
        let is_hovered = self.enabled && response.hovered();
        let has_focus = self.enabled && response.has_focus();

        // Compute fill and border colors based on variant and state
        let (bg_color, border_color, text_color) = match self.variant {
            ButtonVariant::Primary => {
                if !self.enabled {
                    (SURFACE_2, SURFACE_3, TEXT_MUTED.linear_multiply(0.6))
                } else if is_down {
                    (ACCENT.linear_multiply(0.85), ACCENT, TINT_NONE)
                } else if is_hovered {
                    (ACCENT.linear_multiply(1.1), TINT_NONE, TINT_NONE)
                } else {
                    (ACCENT, ACCENT, TINT_NONE)
                }
            }
            ButtonVariant::Secondary => {
                if !self.enabled {
                    (SURFACE_1, SURFACE_2, TEXT_MUTED.linear_multiply(0.5))
                } else if is_down || is_hovered || self.selected {
                    (SURFACE_3, ACCENT, TEXT)
                } else {
                    (SURFACE_2, SURFACE_3, TEXT)
                }
            }
            ButtonVariant::Danger => {
                if !self.enabled {
                    (SURFACE_2, SURFACE_3, TEXT_MUTED.linear_multiply(0.6))
                } else if is_down {
                    (RED_LIVE.linear_multiply(0.85), RED_LIVE, TINT_NONE)
                } else if is_hovered {
                    (RED_LIVE.linear_multiply(1.1), TINT_NONE, TINT_NONE)
                } else {
                    (RED_LIVE, RED_LIVE, TINT_NONE)
                }
            }
            ButtonVariant::Ghost => {
                if !self.enabled {
                    (TRANSPARENT, TRANSPARENT, TEXT_MUTED.linear_multiply(0.5))
                } else if is_down {
                    (SURFACE_3, ACCENT, TEXT)
                } else if is_hovered || self.selected {
                    (SURFACE_2, ACCENT, TEXT)
                } else {
                    (TRANSPARENT, TRANSPARENT, TEXT)
                }
            }
        };

        // Paint background and border
        let corner_radius = CornerRadius::same(CORNER_RADIUS as u8);
        if bg_color != TRANSPARENT || border_color != TRANSPARENT {
            ui.painter().rect(
                rect,
                corner_radius,
                bg_color,
                Stroke::new(BORDER_WIDTH, border_color),
                StrokeKind::Inside,
            );
        }

        // Accessibility Focus Ring
        if has_focus {
            ui.painter().rect_stroke(
                rect.expand(2.0),
                CornerRadius::same((CORNER_RADIUS + 2.0) as u8),
                Stroke::new(1.5_f32, ACCENT),
                StrokeKind::Outside,
            );
        }

        // Active-state persistent indicator for toggle buttons (e.g. Studio Mode, Mute)
        if self.selected {
            let indicator_width = 4.0;
            let left_stripe = Rect::from_min_max(
                rect.left_top(),
                pos2(rect.left() + indicator_width, rect.bottom()),
            );
            ui.painter().rect_filled(
                left_stripe,
                CornerRadius {
                    nw: CORNER_RADIUS as u8,
                    sw: CORNER_RADIUS as u8,
                    ne: 0,
                    se: 0,
                },
                ACCENT,
            );
        }

        // Layout icon + text centered inside rect
        let text_size = text_layout.size();
        let center_x = rect.center().x;
        let start_x = center_x - content_width * 0.5;

        let mut curr_x = start_x;
        if let Some(icon) = self.icon {
            let icon_rect =
                Rect::from_min_size(pos2(curr_x, rect.center().y - 8.0), vec2(16.0, 16.0));
            paint_lucide_icon(
                ui.painter(),
                icon,
                icon_rect,
                Stroke::new(1.5_f32, text_color),
            );
            curr_x += 16.0 + 8.0;
        }

        let text_pos = pos2(curr_x, rect.center().y - text_size.y * 0.5);
        ui.painter().galley(text_pos, text_layout, text_color);

        if let Some(ref tooltip) = self.tooltip {
            crate::icon::show_aligned_tooltip(ui, &response, tooltip);
        }
        response
    }
}

impl egui::Widget for Button {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_button_defaults_and_variants() {
        let b1 = Button::new("Click me");
        assert_eq!(b1.variant, ButtonVariant::Secondary);
        assert_eq!(b1.height, BUTTON_HEIGHT);
        assert!(!b1.selected);
        assert!(b1.enabled);

        let b_pri = Button::new("Primary").primary();
        assert_eq!(b_pri.variant, ButtonVariant::Primary);

        let b_dang = Button::new("Danger").danger();
        assert_eq!(b_dang.variant, ButtonVariant::Danger);

        let b_ghost = Button::new("Ghost").ghost();
        assert_eq!(b_ghost.variant, ButtonVariant::Ghost);

        let b_toggle = Button::new("Toggle").selected(true);
        assert!(b_toggle.selected);
    }

    #[test]
    fn test_button_rendering_in_egui_pass() {
        let ctx = egui::Context::default();
        crate::theme::apply_theme(&ctx);

        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let resp1 = Button::new("Start Streaming").primary().show(ui);
                assert_eq!(resp1.rect.height(), BUTTON_HEIGHT);

                let resp2 = Button::new("Studio Mode")
                    .secondary()
                    .selected(true)
                    .show(ui);
                assert_eq!(resp2.rect.height(), BUTTON_HEIGHT);

                let resp3 = Button::new("Stop").danger().show(ui);
                assert_eq!(resp3.rect.height(), BUTTON_HEIGHT);

                let resp4 = Button::new("Cancel").ghost().show(ui);
                assert_eq!(resp4.rect.height(), BUTTON_HEIGHT);
            });
        });
    }
}
