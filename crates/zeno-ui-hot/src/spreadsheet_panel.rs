//! Spreadsheet links panel (Data mode): configuration of Excel/CSV/Google Sheets sources,
//! live cell bindings, warning badges, test sync triggers, and interactive grid cell picker.

use egui::{Button, Grid, RichText, ScrollArea, TextEdit, Ui};
use zeno_data::sheet::{
    CellBinding, CellTarget, SheetCoordinate, SheetLink, SheetLinkStatus, SheetSource,
    SheetSyncEngine, VariableKind,
};
use zeno_data::DataStore;

use crate::theme::{
    ACCENT, AMBER_WARN, GRID_UNIT, RED_LIVE, SURFACE_0, SURFACE_2, TEXT, TEXT_MUTED,
};

/// UI state for the spreadsheet links panel.
#[derive(Debug, Default)]
pub struct SpreadsheetPanelState {
    pub selected_link_index: usize,
    pub new_path_input: String,
    pub new_cell_target: String,
    pub selected_grid_cell: Option<SheetCoordinate>,
}

/// Renders the complete spreadsheet links and grid picker panel.
pub fn ui_spreadsheet_links_panel(
    engine: &mut SheetSyncEngine,
    store: &DataStore,
    state: &mut SpreadsheetPanelState,
    ui: &mut Ui,
) {
    ui.horizontal(|ui| {
        // --- Left Sidebar: Links List ---
        ui.vertical(|ui| {
            ui.set_width(260.0);

            ui.horizontal(|ui| {
                ui.heading(RichText::new("Data Sources").color(TEXT).strong());
                if ui.button("+ Add Link").clicked() {
                    let new_link = SheetLink::new(
                        SheetSource::LocalFile {
                            path: std::path::PathBuf::from("data/match_stats.xlsx"),
                        },
                        "Sheet1",
                    );
                    let new_idx = engine.add_link(new_link);
                    state.selected_link_index = new_idx;
                }
            });

            ui.add_space(GRID_UNIT * 0.5);

            ScrollArea::vertical()
                .id_salt("sheet_links_scroll")
                .max_height(200.0)
                .show(ui, |ui| {
                    let link_count = engine.links().len();
                    for i in 0..link_count {
                        let is_selected = state.selected_link_index == i;
                        let link = &engine.links()[i];

                        let status_chip = match &link.status {
                            SheetLinkStatus::Ok => RichText::new("[OK]").color(TEXT),
                            SheetLinkStatus::Warning(_) => {
                                RichText::new("[WARN]").color(AMBER_WARN)
                            }
                            SheetLinkStatus::Error(_) => RichText::new("[ERR]").color(RED_LIVE),
                        };

                        let label_text = match &link.source {
                            SheetSource::LocalFile { path } => path
                                .file_name()
                                .and_then(|f| f.to_str())
                                .unwrap_or("spreadsheet.xlsx")
                                .to_string(),
                            SheetSource::GoogleSheet { spreadsheet_id } => {
                                format!("GSheet: {}", spreadsheet_id)
                            }
                            SheetSource::CsvUrl { url } => format!("URL: {}", url),
                        };

                        ui.horizontal(|ui| {
                            ui.label(status_chip);
                            if ui
                                .selectable_label(
                                    is_selected,
                                    RichText::new(label_text).color(TEXT),
                                )
                                .clicked()
                            {
                                state.selected_link_index = i;
                            }
                        });
                    }
                });

            ui.separator();

            // Sync All Button
            if ui
                .button(RichText::new("Sync All Sources").color(ACCENT))
                .clicked()
            {
                engine.sync_all(store);
            }
        });

        ui.separator();

        // --- Center / Right: Selected Link Details & Bindings ---
        ui.vertical(|ui| {
            if state.selected_link_index >= engine.links().len() {
                ui.label(RichText::new("No spreadsheet source selected.").color(TEXT_MUTED));
                return;
            }

            let sel_idx = state.selected_link_index;

            // Header & Status Info
            ui.horizontal(|ui| {
                ui.heading(
                    RichText::new(format!("Link #{}", sel_idx + 1))
                        .color(TEXT)
                        .strong(),
                );

                if let Some(link) = engine.links().get(sel_idx) {
                    match &link.status {
                        SheetLinkStatus::Ok => {
                            ui.label(RichText::new("Live Synced").color(TEXT));
                        }
                        SheetLinkStatus::Warning(msg) => {
                            ui.label(RichText::new(format!("Warning: {}", msg)).color(AMBER_WARN));
                        }
                        SheetLinkStatus::Error(msg) => {
                            ui.label(RichText::new(format!("Error: {}", msg)).color(RED_LIVE));
                        }
                    }
                }

                if ui.button("Test Sync").clicked() {
                    let _ = engine.sync_link(sel_idx, store);
                }
            });

            ui.add_space(GRID_UNIT * 0.5);

            // Bindings Table
            ui.label(RichText::new("Cell Bindings:").color(TEXT).strong());

            let mut to_remove = None;
            let bindings_len = engine.links()[sel_idx].bindings.len();

            Grid::new("bindings_grid")
                .striped(true)
                .min_col_width(120.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("Data Path").color(TEXT_MUTED));
                    ui.label(RichText::new("Cell Target").color(TEXT_MUTED));
                    ui.label(RichText::new("Type").color(TEXT_MUTED));
                    ui.label(RichText::new("Live Value").color(TEXT_MUTED));
                    ui.label(RichText::new("Locked").color(TEXT_MUTED));
                    ui.label(RichText::new("Action").color(TEXT_MUTED));
                    ui.end_row();

                    for b_idx in 0..bindings_len {
                        let link = &engine.links()[sel_idx];
                        let binding = &link.bindings[b_idx];

                        ui.label(RichText::new(&binding.data_path).color(TEXT));
                        ui.label(RichText::new(binding.target.to_string()).color(TEXT));
                        ui.label(RichText::new(binding.value_kind.to_string()).color(TEXT_MUTED));

                        // Live value from store
                        let live_val = store
                            .get(&binding.data_path)
                            .map(|v| v.to_display_string())
                            .unwrap_or_else(|| "<empty>".to_string());
                        ui.label(RichText::new(live_val).color(ACCENT));

                        let is_locked = engine.is_path_locked(&binding.data_path);
                        ui.label(
                            RichText::new(if is_locked { "[Lock]" } else { "-" }).color(TEXT_MUTED),
                        );

                        if ui.small_button("Del").clicked() {
                            to_remove = Some(b_idx);
                        }
                        ui.end_row();
                    }
                });

            if let Some(del_idx) = to_remove {
                engine.links_mut()[sel_idx].bindings.remove(del_idx);
            }

            ui.add_space(GRID_UNIT * 0.5);

            // Add new binding controls
            ui.horizontal(|ui| {
                ui.label(RichText::new("Add:").color(TEXT_MUTED));
                ui.add(
                    TextEdit::singleline(&mut state.new_path_input)
                        .hint_text("Path (e.g. scoreboard.team1.score)"),
                );
                ui.add(
                    TextEdit::singleline(&mut state.new_cell_target).hint_text("Target (e.g. B3)"),
                );

                if ui.button("+ Add Binding").clicked() && !state.new_path_input.is_empty() {
                    let target_str = state.new_cell_target.trim();
                    let target = if target_str.contains(':') {
                        CellTarget::Range(target_str.to_string())
                    } else {
                        CellTarget::Cell(target_str.to_string())
                    };

                    let binding =
                        CellBinding::new(state.new_path_input.trim(), target, VariableKind::Number);
                    engine.links_mut()[sel_idx].add_binding(binding);
                    state.new_path_input.clear();
                    state.new_cell_target.clear();
                }
            });

            ui.separator();

            // --- Interactive Grid Cell Picker Preview ---
            ui.label(
                RichText::new("Grid Cell Picker Preview:")
                    .color(TEXT)
                    .strong(),
            );
            ui.horizontal(|ui| {
                ui.label(RichText::new("Click a cell below to select address:").color(TEXT_MUTED));
                if let Some(coord) = state.selected_grid_cell {
                    ui.label(RichText::new(coord.to_address()).color(ACCENT).strong());
                    if ui.button("Use Target").clicked() {
                        state.new_cell_target = coord.to_address();
                    }
                }
            });

            // Draw a 5-column by 8-row interactive spreadsheet preview
            Grid::new("spreadsheet_picker_grid")
                .striped(true)
                .show(ui, |ui| {
                    // Header row
                    ui.label("");
                    for c in 0..5 {
                        let col_letter = (b'A' + c as u8) as char;
                        ui.label(
                            RichText::new(col_letter.to_string())
                                .color(TEXT_MUTED)
                                .strong(),
                        );
                    }
                    ui.end_row();

                    // Data rows
                    for r in 0..8 {
                        ui.label(RichText::new(format!("{}", r + 1)).color(TEXT_MUTED));
                        for c in 0..5 {
                            let coord = SheetCoordinate { col: c, row: r };
                            let addr = coord.to_address();
                            let is_selected = state.selected_grid_cell == Some(coord);

                            let cell_button = if is_selected {
                                Button::new(RichText::new(&addr).color(SURFACE_0).strong())
                                    .fill(ACCENT)
                            } else {
                                Button::new(RichText::new(&addr).color(TEXT)).fill(SURFACE_2)
                            };

                            if ui.add(cell_button).clicked() {
                                state.selected_grid_cell = Some(coord);
                                state.new_cell_target = addr;
                            }
                        }
                        ui.end_row();
                    }
                });
        });
    });
}
