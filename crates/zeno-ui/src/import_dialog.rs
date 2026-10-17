//! Modal dialog presenting an import summary report for PSD and Lottie assets.

use egui::{Align, Layout, RichText, ScrollArea, Window};
use zeno_import::report::ImportReport;

use crate::theme::{
    ACCENT, AMBER_WARN, BUTTON_HEIGHT, GREEN_OK, GRID_UNIT, RED_LIVE, TEXT, TEXT_MUTED,
};

/// State for the Import Report modal dialog.
#[derive(Debug, Clone, Default)]
pub struct ImportReportDialog {
    /// Whether the dialog is currently visible.
    pub is_open: bool,
    /// The report being viewed.
    pub report: Option<ImportReport>,
}

impl ImportReportDialog {
    /// Opens the dialog with the provided report.
    pub fn open(&mut self, report: ImportReport) {
        self.report = Some(report);
        self.is_open = true;
    }

    /// Closes the dialog.
    pub fn close(&mut self) {
        self.is_open = false;
        self.report = None;
    }

    /// Renders the modal window if `is_open`. Returns `true` if accepted/confirmed.
    pub fn show(&mut self, ctx: &egui::Context) -> bool {
        if !self.is_open {
            return false;
        }

        let mut accepted = false;
        let mut close_requested = false;

        if let Some(report) = &self.report {
            Window::new("Asset Import Report")
                .collapsible(false)
                .resizable(true)
                .min_width(480.0)
                .default_width(560.0)
                .open(&mut self.is_open)
                .show(ctx, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(GRID_UNIT, GRID_UNIT);

                    // Header: File name and importer badge
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Source:").color(TEXT_MUTED).size(13.0));
                        ui.label(RichText::new(&report.file_name).color(TEXT).strong());
                        ui.label(
                            RichText::new(format!("[{}]", report.importer))
                                .color(ACCENT)
                                .size(12.0),
                        );
                    });

                    ui.separator();

                    // Summary statistics
                    ui.horizontal(|ui| {
                        let status_color = if report.success && !report.has_issues() {
                            GREEN_OK
                        } else if report.success {
                            AMBER_WARN
                        } else {
                            RED_LIVE
                        };

                        let status_text = if report.success && !report.has_issues() {
                            "Status: 100% Compatible"
                        } else if report.success {
                            "Status: Imported with Warnings"
                        } else {
                            "Status: Failed"
                        };

                        ui.label(RichText::new(status_text).color(status_color).strong());
                        ui.separator();
                        ui.label(
                            RichText::new(format!("Layers: {}", report.imported_layers))
                                .color(TEXT),
                        );
                        ui.label(
                            RichText::new(format!("Groups: {}", report.groups_count))
                                .color(TEXT_MUTED),
                        );
                        ui.label(
                            RichText::new(format!("Bindings: {}", report.bindings_created.len()))
                                .color(ACCENT),
                        );
                    });

                    // Scrollable report details
                    ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                        if !report.bindings_created.is_empty() {
                            ui.add_space(4.0);
                            ui.label(RichText::new("Auto-Generated Data Bindings:").color(ACCENT).strong());
                            for (layer_name, path) in &report.bindings_created {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(format!(" • {}", layer_name)).color(TEXT));
                                    ui.label(RichText::new(format!("-> {}", path)).color(TEXT_MUTED));
                                });
                            }
                        }

                        if !report.unsupported_features.is_empty() {
                            ui.add_space(4.0);
                            ui.label(RichText::new("Unsupported Features / Substitutions:").color(AMBER_WARN).strong());
                            for feature in &report.unsupported_features {
                                ui.label(RichText::new(format!(" ! {}", feature)).color(AMBER_WARN));
                            }
                        }

                        if !report.warnings.is_empty() {
                            ui.add_space(4.0);
                            ui.label(RichText::new("Warnings:").color(TEXT_MUTED).strong());
                            for warn in &report.warnings {
                                ui.label(RichText::new(format!(" - {}", warn)).color(TEXT_MUTED));
                            }
                        }

                        if !report.has_issues() && report.bindings_created.is_empty() {
                            ui.add_space(8.0);
                            ui.label(RichText::new("All vector shapes, transforms, and timing curves imported cleanly with no warnings.").color(GREEN_OK));
                        }
                    });

                    ui.separator();

                    // Footer action buttons
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui
                            .add_sized([100.0, BUTTON_HEIGHT], egui::Button::new(RichText::new("Confirm").color(TEXT)))
                            .clicked()
                        {
                            accepted = true;
                            close_requested = true;
                        }

                        if ui
                            .add_sized([80.0, BUTTON_HEIGHT], egui::Button::new(RichText::new("Dismiss").color(TEXT_MUTED)))
                            .clicked()
                        {
                            close_requested = true;
                        }
                    });
                });
        }

        if close_requested {
            self.close();
        }

        accepted
    }
}
