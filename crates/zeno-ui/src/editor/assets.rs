//! Asset browser: native media import, asset listings, drag-and-drop to canvas, and container packaging.

use std::path::Path;

use egui::{
    vec2, Align2, Button, CornerRadius, CursorIcon, Id, LayerId as EguiLayerId, Order, Rect, Sense,
    Stroke, StrokeKind, Ui,
};
use zeno_engine::AssetKind;

use crate::editor::state::OverlayEditor;
use crate::theme::{
    ACCENT, AMBER_WARN, BORDER_WIDTH, CORNER_RADIUS, GREEN_OK, GRID_UNIT, SURFACE_0, SURFACE_1,
    SURFACE_2, TEXT, TEXT_MUTED,
};

/// Renders the asset browser panel.
pub fn ui_assets_panel(editor: &mut OverlayEditor, ui: &mut Ui) {
    let panel_rect = ui.available_rect_before_wrap();

    // Check for drag-and-drop from Windows Explorer onto Assets panel
    let dropped_from_os = ui.input(|i| {
        if !i.raw.dropped_files.is_empty() {
            if let Some(pos) = i.pointer.hover_pos() {
                if panel_rect.contains(pos) {
                    return i.raw.dropped_files.clone();
                }
            }
        }
        Vec::new()
    });

    for file in dropped_from_os {
        if let Some(ref path) = file.path {
            editor.import_asset(path);
        }
    }

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            if ui.button("+ Import Media…").clicked() {
                let picked = rfd::FileDialog::new()
                    .add_filter(
                        "Supported Media (*.png, *.jpg, *.jpeg, *.gif, *.webp, *.mp4, *.mov, *.webm, *.wav, *.mp3, *.ttf, *.otf)",
                        &[
                            "png", "jpg", "jpeg", "gif", "webp", "mp4", "mov", "webm", "wav",
                            "mp3", "ttf", "otf",
                        ],
                    )
                    .add_filter("Images (*.png, *.jpg, *.jpeg, *.gif, *.webp)", &[
                        "png", "jpg", "jpeg", "gif", "webp",
                    ])
                    .add_filter("Video (*.mp4, *.mov, *.webm)", &["mp4", "mov", "webm"])
                    .add_filter("Audio (*.wav, *.mp3, *.ogg, *.flac)", &[
                        "wav", "mp3", "ogg", "flac",
                    ])
                    .add_filter("Fonts (*.ttf, *.otf)", &["ttf", "otf"])
                    .add_filter("All Files (*.*)", &["*"])
                    .pick_files();

                if let Some(paths) = picked {
                    for path in paths {
                        editor.import_asset(&path);
                    }
                }
            }

            ui.colored_label(
                TEXT_MUTED,
                format!("{} assets", editor.composition.assets.len()),
            );
        });

        ui.separator();

        if editor.composition.assets.is_empty() {
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.colored_label(
                    TEXT_MUTED,
                    "No assets imported yet — click + Import Media or drag files here",
                );
            });
            return;
        }

        let mut to_add_layer: Option<(zeno_core::AssetId, String)> = None;

        egui::ScrollArea::vertical().show(ui, |ui| {
            let assets = editor.composition.assets.clone();
            for asset in &assets {
                let clean_name = Path::new(&asset.path)
                    .file_name()
                    .map(|f| f.to_string_lossy().to_string())
                    .unwrap_or_else(|| asset.path.clone());

                let size_text = if asset.size_bytes >= 1024 * 1024 {
                    format!("{:.1} MB", asset.size_bytes as f64 / (1024.0 * 1024.0))
                } else {
                    format!("{:.1} KB", (asset.size_bytes as f64 / 1024.0).max(0.1))
                };

                let (badge_text, badge_color) = match asset.kind {
                    AssetKind::Image => ("[IMG]", ACCENT),
                    AssetKind::Video => ("[VID]", AMBER_WARN),
                    AssetKind::Audio => ("[AUD]", GREEN_OK),
                    AssetKind::Font => ("[FONT]", TEXT),
                    AssetKind::Model3D => ("[3D]", ACCENT),
                };

                let item_resp = egui::Frame::NONE
                    .fill(SURFACE_1)
                    .inner_margin(egui::Margin::same(GRID_UNIT as i8))
                    .corner_radius(CornerRadius::same(CORNER_RADIUS as u8))
                    .stroke(Stroke::new(BORDER_WIDTH, SURFACE_2))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            // Drag grip
                            let grip_resp = ui
                                .colored_label(TEXT_MUTED, "⠿")
                                .interact(Sense::drag());

                            if grip_resp.hovered() {
                                ui.ctx().set_cursor_icon(CursorIcon::Grab);
                            }

                            if grip_resp.drag_started() {
                                ui.memory_mut(|mem| {
                                    mem.data.insert_temp(
                                        Id::new("dnd_dragging_asset_id"),
                                        asset.id.0.to_string(),
                                    );
                                });
                            }

                            if grip_resp.dragged() {
                                ui.ctx().set_cursor_icon(CursorIcon::Grabbing);
                                if let Some(pointer_pos) = ui.ctx().pointer_latest_pos() {
                                    let painter = ui.ctx().layer_painter(EguiLayerId::new(
                                        Order::Tooltip,
                                        Id::new("asset_drag_tooltip"),
                                    ));
                                    let tip_text = format!("Adding {} to canvas", clean_name);
                                    let tip_pos = pointer_pos + vec2(16.0, 16.0);
                                    let font = egui::FontId::proportional(12.0);
                                    painter.rect_filled(
                                        Rect::from_min_size(tip_pos, vec2(180.0, 24.0)),
                                        CornerRadius::same(CORNER_RADIUS as u8),
                                        SURFACE_0,
                                    );
                                    painter.rect_stroke(
                                        Rect::from_min_size(tip_pos, vec2(180.0, 24.0)),
                                        CornerRadius::same(CORNER_RADIUS as u8),
                                        Stroke::new(BORDER_WIDTH, ACCENT),
                                        StrokeKind::Inside,
                                    );
                                    painter.text(
                                        tip_pos + vec2(8.0, 4.0),
                                        Align2::LEFT_TOP,
                                        tip_text,
                                        font,
                                        TEXT,
                                    );
                                }
                            }

                            ui.colored_label(badge_color, badge_text);

                            ui.vertical(|ui| {
                                ui.label(&clean_name);
                                ui.colored_label(TEXT_MUTED, size_text);
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add(Button::new("+ Add").small())
                                        .on_hover_text("Add as new layer centered on canvas")
                                        .clicked()
                                    {
                                        to_add_layer = Some((asset.id, clean_name.clone()));
                                    }
                                },
                            );
                        });
                    })
                    .response;

                if item_resp.drag_started() {
                    ui.memory_mut(|mem| {
                        mem.data.insert_temp(
                            Id::new("dnd_dragging_asset_id"),
                            asset.id.0.to_string(),
                        );
                    });
                }
            }
        });

        if let Some((asset_id, name)) = to_add_layer {
            let (nw, nh) = editor
                .asset_dimensions
                .get(&asset_id)
                .copied()
                .unwrap_or((320, 180));
            let max_w = editor.composition.width as f32;
            let max_h = editor.composition.height as f32;
            let mut w = nw as f32;
            let mut h = nh as f32;
            if w > max_w || h > max_h {
                let s = (max_w / w).min(max_h / h);
                w *= s;
                h *= s;
            }
            editor.add_image_layer_from_asset(asset_id, &name, w, h);
        }
    });
}
