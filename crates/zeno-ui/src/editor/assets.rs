//! Asset browser: imported images, fonts, and media assets in .arx container.

use egui::Ui;
use zeno_core::AssetId;
use zeno_engine::{AssetKind, AssetRef};

use crate::editor::state::OverlayEditor;
use crate::theme::{ACCENT, GRID_UNIT, SURFACE_1, TEXT_MUTED};

/// Renders the asset browser panel.
pub fn ui_assets_panel(editor: &mut OverlayEditor, ui: &mut Ui) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            if ui.button("+ Import Asset").clicked() {
                let id = AssetId::new();
                editor.composition.assets.push(AssetRef {
                    id,
                    path: format!("assets/asset_{}.png", editor.composition.assets.len() + 1),
                    kind: AssetKind::Image,
                    size_bytes: 1024,
                    sha256: "dummy_hash".into(),
                });
            }
        });

        ui.separator();

        if editor.composition.assets.is_empty() {
            ui.colored_label(TEXT_MUTED, "No assets imported in .arx container");
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for asset in &editor.composition.assets {
                    egui::Frame::NONE
                        .fill(SURFACE_1)
                        .inner_margin(egui::Margin::same(GRID_UNIT as i8))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.colored_label(ACCENT, "[IMG]");
                                ui.label(&asset.path);
                            });
                        });
                }
            });
        }
    });
}
