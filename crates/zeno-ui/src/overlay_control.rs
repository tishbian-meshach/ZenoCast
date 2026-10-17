//! Overlay lifecycle control panel: per-overlay status indicators,
//! progress meters, ping-pong options, and Play In / Play Out / Toggle triggers.

use egui::{Button, ProgressBar, Ui};
use zeno_core::LifecyclePhase;
use zeno_engine::LifecycleManager;

use crate::theme::{ACCENT, AMBER_WARN, GRID_UNIT, RED_LIVE, SURFACE_1, TEXT, TEXT_MUTED};

/// Renders the overlay lifecycle control panel.
pub fn ui_overlay_lifecycle_panel(manager: &mut LifecycleManager, ui: &mut Ui) {
    ui.vertical(|ui| {
        // Global Actions Toolbar
        ui.horizontal(|ui| {
            if ui.button("All In [>>]").clicked() {
                manager.play_in_all();
            }
            if ui.button("All Out [<<]").clicked() {
                manager.play_out_all();
            }
        });

        ui.separator();

        if manager.controllers().is_empty() {
            ui.colored_label(TEXT_MUTED, "No active overlays registered");
            return;
        }

        egui::ScrollArea::vertical().show(ui, |ui| {
            for controller in manager.controllers_mut() {
                egui::Frame::NONE
                    .fill(SURFACE_1)
                    .inner_margin(egui::Margin::same(GRID_UNIT as i8))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Name & Phase Status Indicator
                            ui.colored_label(TEXT, &controller.name);

                            let (status_text, status_color) = match controller.phase {
                                LifecyclePhase::Hidden => ("HIDDEN", TEXT_MUTED),
                                LifecyclePhase::In => ("IN", ACCENT),
                                LifecyclePhase::Loop => ("LOOP", AMBER_WARN),
                                LifecyclePhase::Out => ("OUT", RED_LIVE),
                            };
                            ui.colored_label(status_color, format!("[{}]", status_text));

                            // Effective Alpha Meter
                            let alpha = controller.effective_alpha();
                            ui.add(
                                ProgressBar::new(alpha)
                                    .desired_width(80.0)
                                    .text(format!("{:.0}%", alpha * 100.0)),
                            );

                            ui.separator();

                            // Control Buttons
                            let can_in = matches!(
                                controller.phase,
                                LifecyclePhase::Hidden | LifecyclePhase::Out
                            );
                            if ui.add_enabled(can_in, Button::new("Play In")).clicked() {
                                controller.play_in();
                            }

                            let can_out = matches!(
                                controller.phase,
                                LifecyclePhase::In | LifecyclePhase::Loop
                            );
                            if ui.add_enabled(can_out, Button::new("Play Out")).clicked() {
                                controller.play_out();
                            }

                            if ui.button("Toggle").clicked() {
                                controller.toggle();
                            }

                            // Ping-pong checkbox
                            ui.checkbox(&mut controller.ping_pong, "Ping-Pong");
                        });
                    });
            }
        });
    });
}
