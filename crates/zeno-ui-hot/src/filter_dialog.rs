//! Dual-list Filter Modal for configuring source Video Effects and Audio DSP filters.
//!
//! Conforms to Section 1.2 of v1.5:
//! - Dual-list modal: left list is ordered filter chain, right list is typed properties.
//! - Add Filter grouped by FilterCategory (Video vs Audio).
//! - Reorder (move up/down), enable/disable toggle, remove.
//! - Flat broadcast theme tokens from `theme.rs`.

use egui::{vec2, Align2, Key, Window};
use zeno_core::{FilterChain, FilterDefinition, FilterKind, FilterSettings};

use crate::button::Button;
use crate::icon::{icon_toggle_button, IconButton, IconKind};
use crate::theme::{ACCENT, GRID_UNIT, TEXT, TEXT_MUTED};

/// Operating state for the source filter dialog modal.
#[derive(Debug, Clone, Default)]
pub struct FilterDialogState {
    pub is_open: bool,
    pub source_id: Option<String>,
    pub source_name: String,
    pub filters: FilterChain,
    pub selected_filter_idx: Option<usize>,
    pub copy_feedback: Option<String>,
}

/// Action emitted when user modifies or closes the filter dialog.
#[derive(Debug, Clone, PartialEq)]
pub enum FilterDialogAction {
    /// Live update of the filter chain for the given source.
    Update {
        source_id: String,
        filters: FilterChain,
    },
    /// Copy the current source's filter chain to clipboard.
    CopyFilters { filters: FilterChain },
    /// Paste filters from clipboard onto this source.
    PasteFilters { source_id: String },
}

impl FilterDialogState {
    /// Opens the filter modal for the specified source.
    pub fn open(&mut self, source_id: String, source_name: String, filters: FilterChain) {
        self.is_open = true;
        self.source_id = Some(source_id);
        self.source_name = source_name;
        self.filters = filters;
        self.selected_filter_idx = if self.filters.is_empty() {
            None
        } else {
            Some(0)
        };
        self.copy_feedback = None;
    }

    /// Closes the modal.
    pub fn close(&mut self) {
        self.is_open = false;
        self.source_id = None;
        self.selected_filter_idx = None;
    }

    /// Returns true if the dialog is currently visible.
    pub fn is_open(&self) -> bool {
        self.is_open
    }

    /// Renders the modal and returns any triggered action.
    pub fn ui(
        &mut self,
        ctx: &egui::Context,
        has_clipboard_filters: bool,
    ) -> Option<FilterDialogAction> {
        if !self.is_open {
            return None;
        }

        let mut emitted_action = None;
        let mut filters_mutated = false;
        let mut close_modal = false;

        // Escape closes modal
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            close_modal = true;
        }

        let title = format!("Filters: {}", self.source_name);
        Window::new(title)
            .collapsible(false)
            .resizable(true)
            .default_size(vec2(680.0, 480.0))
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Left Column: Filter Stack List (width ~240px)
                    ui.allocate_ui_with_layout(
                        vec2(240.0, ui.available_height() - 48.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.label(egui::RichText::new("Filter Chain").strong());
                            ui.separator();

                            egui::ScrollArea::vertical()
                                .max_height(ui.available_height() - 40.0)
                                .show(ui, |ui| {
                                    if self.filters.is_empty() {
                                        ui.add_space(GRID_UNIT);
                                        ui.label(
                                            egui::RichText::new("No filters applied")
                                                .color(TEXT_MUTED),
                                        );
                                    } else {
                                        let mut move_up_idx = None;
                                        let mut move_down_idx = None;
                                        let mut remove_idx = None;
                                        let total_filters = self.filters.len();

                                        for (idx, filter) in
                                            self.filters.filters.iter_mut().enumerate()
                                        {
                                            let is_selected = self.selected_filter_idx == Some(idx);
                                            ui.horizontal(|ui| {
                                                // Enable/disable toggle
                                                if icon_toggle_button(
                                                    ui,
                                                    filter.enabled,
                                                    IconKind::Eye,
                                                    IconKind::EyeOff,
                                                    if filter.enabled {
                                                        "Disable Filter"
                                                    } else {
                                                        "Enable Filter"
                                                    },
                                                )
                                                .clicked()
                                                {
                                                    filter.enabled = !filter.enabled;
                                                    filters_mutated = true;
                                                }

                                                // Selectable row name
                                                let text = egui::RichText::new(&filter.name);
                                                let text = if is_selected {
                                                    text.strong().color(ACCENT)
                                                } else if !filter.enabled {
                                                    text.color(TEXT_MUTED)
                                                } else {
                                                    text.color(TEXT)
                                                };

                                                if ui
                                                    .add(egui::SelectableLabel::new(
                                                        is_selected,
                                                        text,
                                                    ))
                                                    .clicked()
                                                {
                                                    self.selected_filter_idx = Some(idx);
                                                }

                                                // Reorder & remove controls
                                                if idx > 0
                                                    && IconButton::new(IconKind::ChevronUp)
                                                        .ghost(true)
                                                        .size(vec2(18.0, 18.0))
                                                        .tooltip("Move Filter Up")
                                                        .show(ui)
                                                        .clicked()
                                                {
                                                    move_up_idx = Some(idx);
                                                }
                                                if idx + 1 < total_filters
                                                    && IconButton::new(IconKind::ChevronDown)
                                                        .ghost(true)
                                                        .size(vec2(18.0, 18.0))
                                                        .tooltip("Move Filter Down")
                                                        .show(ui)
                                                        .clicked()
                                                {
                                                    move_down_idx = Some(idx);
                                                }
                                                if IconButton::new(IconKind::Trash)
                                                    .ghost(true)
                                                    .size(vec2(18.0, 18.0))
                                                    .tooltip("Remove Filter")
                                                    .show(ui)
                                                    .clicked()
                                                {
                                                    remove_idx = Some(idx);
                                                }
                                            });
                                        }

                                        if let Some(idx) = move_up_idx {
                                            self.filters.move_up(idx);
                                            self.selected_filter_idx = Some(idx - 1);
                                            filters_mutated = true;
                                        } else if let Some(idx) = move_down_idx {
                                            self.filters.move_down(idx);
                                            self.selected_filter_idx = Some(idx + 1);
                                            filters_mutated = true;
                                        } else if let Some(idx) = remove_idx {
                                            self.filters.filters.remove(idx);
                                            if self.filters.is_empty() {
                                                self.selected_filter_idx = None;
                                            } else if let Some(sel) = self.selected_filter_idx {
                                                if sel >= self.filters.len() {
                                                    self.selected_filter_idx =
                                                        Some(self.filters.len() - 1);
                                                }
                                            }
                                            filters_mutated = true;
                                        }
                                    }
                                });

                            ui.separator();
                            // Add Filter Dropdown Menu
                            ui.menu_button("+ Add Filter", |ui| {
                                ui.label(
                                    egui::RichText::new("Video Effect Filters")
                                        .strong()
                                        .color(ACCENT),
                                );
                                for kind in &[
                                    FilterKind::ColorCorrection,
                                    FilterKind::ChromaKey,
                                    FilterKind::Blur,
                                    FilterKind::Sharpen,
                                    FilterKind::CropPad,
                                    FilterKind::Mask,
                                ] {
                                    if ui.button(kind.default_name()).clicked() {
                                        let new_f = FilterDefinition::new(*kind);
                                        self.filters.add(new_f);
                                        self.selected_filter_idx = Some(self.filters.len() - 1);
                                        filters_mutated = true;
                                        ui.close_menu();
                                    }
                                }

                                ui.separator();
                                ui.label(
                                    egui::RichText::new("Audio DSP Filters")
                                        .strong()
                                        .color(ACCENT),
                                );
                                for kind in &[
                                    FilterKind::Gain,
                                    FilterKind::NoiseGate,
                                    FilterKind::Compressor,
                                    FilterKind::Limiter,
                                    FilterKind::Eq,
                                    FilterKind::NoiseSuppression,
                                ] {
                                    if ui.button(kind.default_name()).clicked() {
                                        let new_f = FilterDefinition::new(*kind);
                                        self.filters.add(new_f);
                                        self.selected_filter_idx = Some(self.filters.len() - 1);
                                        filters_mutated = true;
                                        ui.close_menu();
                                    }
                                }
                            });
                        },
                    );

                    ui.separator();

                    // Right Column: Filter Properties Inspector
                    ui.allocate_ui_with_layout(
                        vec2(ui.available_width(), ui.available_height() - 48.0),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            if let Some(idx) = self.selected_filter_idx {
                                if let Some(filter) = self.filters.filters.get_mut(idx) {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("Name:").strong());
                                        if ui.text_edit_singleline(&mut filter.name).changed() {
                                            filters_mutated = true;
                                        }
                                        if ui.checkbox(&mut filter.enabled, "Enabled").changed() {
                                            filters_mutated = true;
                                        }
                                    });
                                    ui.separator();

                                    egui::ScrollArea::vertical().show(ui, |ui| {
                                        let mutated =
                                            render_filter_settings(ui, &mut filter.settings);
                                        if mutated {
                                            filters_mutated = true;
                                        }
                                    });
                                } else {
                                    ui.label(
                                        egui::RichText::new("Select a filter to adjust settings.")
                                            .color(TEXT_MUTED),
                                    );
                                }
                            } else {
                                ui.label(
                                    egui::RichText::new("Select a filter to adjust settings.")
                                        .color(TEXT_MUTED),
                                );
                            }
                        },
                    );
                });

                ui.separator();

                // Bottom Controls Toolbar
                ui.horizontal(|ui| {
                    if Button::new("Copy Filters")
                        .secondary()
                        .icon(IconKind::Copy)
                        .show(ui)
                        .clicked()
                    {
                        emitted_action = Some(FilterDialogAction::CopyFilters {
                            filters: self.filters.clone(),
                        });
                        self.copy_feedback = Some("Filters copied to clipboard".to_string());
                    }

                    if Button::new("Paste Filters")
                        .secondary()
                        .enabled(has_clipboard_filters)
                        .show(ui)
                        .clicked()
                    {
                        if let Some(ref sid) = self.source_id {
                            emitted_action = Some(FilterDialogAction::PasteFilters {
                                source_id: sid.clone(),
                            });
                        }
                    }

                    if let Some(ref msg) = self.copy_feedback {
                        ui.colored_label(ACCENT, msg);
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if Button::new("Close").primary().show(ui).clicked() {
                            close_modal = true;
                        }
                    });
                });
            });

        if close_modal {
            self.close();
        }

        if filters_mutated {
            if let Some(ref sid) = self.source_id {
                emitted_action = Some(FilterDialogAction::Update {
                    source_id: sid.clone(),
                    filters: self.filters.clone(),
                });
            }
        }

        emitted_action
    }
}

/// Renders the property controls for a specific filter kind. Returns true if settings were mutated.
fn render_filter_settings(ui: &mut egui::Ui, settings: &mut FilterSettings) -> bool {
    let mut mutated = false;

    match settings {
        FilterSettings::ColorCorrection {
            brightness,
            contrast,
            saturation,
            hue_shift,
            gamma,
            opacity,
        } => {
            ui.label(egui::RichText::new("Color Correction").strong());
            ui.add_space(GRID_UNIT);

            ui.horizontal(|ui| {
                ui.label("Brightness:");
                if ui
                    .add(egui::Slider::new(brightness, -1.0..=1.0).step_by(0.01))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Contrast:");
                if ui
                    .add(egui::Slider::new(contrast, 0.0..=2.0).step_by(0.01))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Saturation:");
                if ui
                    .add(egui::Slider::new(saturation, 0.0..=2.0).step_by(0.01))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Hue Shift (°):");
                if ui
                    .add(egui::Slider::new(hue_shift, -180.0..=180.0).step_by(1.0))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Gamma:");
                if ui
                    .add(egui::Slider::new(gamma, 0.1..=3.0).step_by(0.01))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Opacity:");
                if ui
                    .add(egui::Slider::new(opacity, 0.0..=1.0).step_by(0.01))
                    .changed()
                {
                    mutated = true;
                }
            });
        }

        FilterSettings::ChromaKey {
            key_color,
            similarity,
            smoothness,
            spill_reduction,
        } => {
            ui.label(egui::RichText::new("Chroma Key (Green/Blue Screen)").strong());
            ui.add_space(GRID_UNIT);

            ui.horizontal(|ui| {
                ui.label("Key Color:");
                let mut rgb = [key_color[0], key_color[1], key_color[2]];
                if ui.color_edit_button_rgb(&mut rgb).changed() {
                    key_color[0] = rgb[0];
                    key_color[1] = rgb[1];
                    key_color[2] = rgb[2];
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Similarity:");
                if ui
                    .add(egui::Slider::new(similarity, 0.0..=1.0).step_by(0.01))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Smoothness:");
                if ui
                    .add(egui::Slider::new(smoothness, 0.0..=1.0).step_by(0.01))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Key Color Spill Reduction:");
                if ui
                    .add(egui::Slider::new(spill_reduction, 0.0..=1.0).step_by(0.01))
                    .changed()
                {
                    mutated = true;
                }
            });
        }

        FilterSettings::Blur { radius } => {
            ui.label(egui::RichText::new("Gaussian Blur").strong());
            ui.add_space(GRID_UNIT);

            ui.horizontal(|ui| {
                ui.label("Radius (px):");
                if ui
                    .add(egui::Slider::new(radius, 1.0..=64.0).step_by(1.0))
                    .changed()
                {
                    mutated = true;
                }
            });
        }

        FilterSettings::Sharpen { sharpness } => {
            ui.label(egui::RichText::new("Sharpen").strong());
            ui.add_space(GRID_UNIT);

            ui.horizontal(|ui| {
                ui.label("Sharpness:");
                if ui
                    .add(egui::Slider::new(sharpness, 0.0..=1.0).step_by(0.01))
                    .changed()
                {
                    mutated = true;
                }
            });
        }

        FilterSettings::CropPad {
            left,
            top,
            right,
            bottom,
        } => {
            ui.label(egui::RichText::new("Crop / Pad").strong());
            ui.add_space(GRID_UNIT);

            ui.horizontal(|ui| {
                ui.label("Left (px):");
                if ui.add(egui::DragValue::new(left).speed(1.0)).changed() {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Top (px):");
                if ui.add(egui::DragValue::new(top).speed(1.0)).changed() {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Right (px):");
                if ui.add(egui::DragValue::new(right).speed(1.0)).changed() {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Bottom (px):");
                if ui.add(egui::DragValue::new(bottom).speed(1.0)).changed() {
                    mutated = true;
                }
            });
        }

        FilterSettings::Mask { path, invert } => {
            ui.label(egui::RichText::new("Image Mask / Blend").strong());
            ui.add_space(GRID_UNIT);

            ui.horizontal(|ui| {
                ui.label("Mask Image Path:");
                if ui.text_edit_singleline(path).changed() {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                if ui.checkbox(invert, "Invert Mask").changed() {
                    mutated = true;
                }
            });
        }

        FilterSettings::Gain { gain_db } => {
            ui.label(egui::RichText::new("Audio Gain").strong());
            ui.add_space(GRID_UNIT);

            ui.horizontal(|ui| {
                ui.label("Gain (dB):");
                if ui
                    .add(egui::Slider::new(gain_db, -30.0..=30.0).step_by(0.1))
                    .changed()
                {
                    mutated = true;
                }
            });
        }

        FilterSettings::NoiseGate {
            threshold_db,
            attack_ms,
            hold_ms,
            release_ms,
            reduction_db,
        } => {
            ui.label(egui::RichText::new("Noise Gate").strong());
            ui.add_space(GRID_UNIT);

            ui.horizontal(|ui| {
                ui.label("Close Threshold (dB):");
                if ui
                    .add(egui::Slider::new(threshold_db, -60.0..=0.0).step_by(0.5))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Attack (ms):");
                if ui
                    .add(egui::Slider::new(attack_ms, 0.1..=50.0).step_by(0.1))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Hold (ms):");
                if ui
                    .add(egui::Slider::new(hold_ms, 0.0..=500.0).step_by(5.0))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Release (ms):");
                if ui
                    .add(egui::Slider::new(release_ms, 10.0..=1000.0).step_by(10.0))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Reduction (dB):");
                if ui
                    .add(egui::Slider::new(reduction_db, -80.0..=0.0).step_by(1.0))
                    .changed()
                {
                    mutated = true;
                }
            });
        }

        FilterSettings::Compressor {
            threshold_db,
            ratio,
            attack_ms,
            release_ms,
            makeup_gain_db,
        } => {
            ui.label(egui::RichText::new("Compressor").strong());
            ui.add_space(GRID_UNIT);

            ui.horizontal(|ui| {
                ui.label("Threshold (dB):");
                if ui
                    .add(egui::Slider::new(threshold_db, -40.0..=0.0).step_by(0.5))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Ratio (x:1):");
                if ui
                    .add(egui::Slider::new(ratio, 1.0..=32.0).step_by(0.5))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Attack (ms):");
                if ui
                    .add(egui::Slider::new(attack_ms, 0.1..=50.0).step_by(0.1))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Release (ms):");
                if ui
                    .add(egui::Slider::new(release_ms, 10.0..=1000.0).step_by(10.0))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Output Gain (dB):");
                if ui
                    .add(egui::Slider::new(makeup_gain_db, 0.0..=30.0).step_by(0.5))
                    .changed()
                {
                    mutated = true;
                }
            });
        }

        FilterSettings::Limiter {
            threshold_db,
            release_ms,
        } => {
            ui.label(egui::RichText::new("Limiter").strong());
            ui.add_space(GRID_UNIT);

            ui.horizontal(|ui| {
                ui.label("Threshold (dB):");
                if ui
                    .add(egui::Slider::new(threshold_db, -20.0..=0.0).step_by(0.1))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Release (ms):");
                if ui
                    .add(egui::Slider::new(release_ms, 10.0..=500.0).step_by(5.0))
                    .changed()
                {
                    mutated = true;
                }
            });
        }

        FilterSettings::Eq {
            low_gain_db,
            mid_gain_db,
            high_gain_db,
        } => {
            ui.label(egui::RichText::new("3-Band Equalizer").strong());
            ui.add_space(GRID_UNIT);

            ui.horizontal(|ui| {
                ui.label("Low Gain (100 Hz, dB):");
                if ui
                    .add(egui::Slider::new(low_gain_db, -15.0..=15.0).step_by(0.5))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("Mid Gain (1 kHz, dB):");
                if ui
                    .add(egui::Slider::new(mid_gain_db, -15.0..=15.0).step_by(0.5))
                    .changed()
                {
                    mutated = true;
                }
            });
            ui.horizontal(|ui| {
                ui.label("High Gain (8 kHz, dB):");
                if ui
                    .add(egui::Slider::new(high_gain_db, -15.0..=15.0).step_by(0.5))
                    .changed()
                {
                    mutated = true;
                }
            });
        }

        FilterSettings::NoiseSuppression { suppress_level_db } => {
            ui.label(egui::RichText::new("Noise Suppression (Spectral Subtraction)").strong());
            ui.add_space(GRID_UNIT);

            ui.horizontal(|ui| {
                ui.label("Suppression Level (dB):");
                if ui
                    .add(egui::Slider::new(suppress_level_db, -60.0..=0.0).step_by(1.0))
                    .changed()
                {
                    mutated = true;
                }
            });
        }
    }

    mutated
}
