//! Esports fast data control panel (Data mode):
//! Big, rapid controls for score increments (+/-), team/player names,
//! series status, quick scene switcher buttons, and overlay triggers.

use egui::{Button, RichText, ScrollArea, TextEdit, Ui};
use zeno_control::ControlState;
use zeno_data::DataStore;

use crate::theme::{
    ACCENT, AMBER_WARN, BUTTON_HEIGHT, GRID_UNIT, RED_LIVE, SURFACE_0, SURFACE_2, SURFACE_3, TEXT,
    TEXT_MUTED,
};

/// UI state for the Esports Data mode control panel.
#[derive(Debug, Default)]
pub struct DataPanelState {
    pub team1_name: String,
    pub team2_name: String,
    pub caster1_name: String,
    pub caster2_name: String,
    pub map_name: String,
    pub best_of: String,
}

/// Renders the Esports Data Mode control panel.
pub fn ui_esports_data_panel(
    store: &DataStore,
    control_state: &ControlState,
    state: &mut DataPanelState,
    ui: &mut Ui,
) {
    ScrollArea::vertical().show(ui, |ui| {
        ui.add_space(GRID_UNIT);

        // --- Master Production Toolbar ---
        ui.horizontal(|ui| {
            ui.heading(RichText::new("ESPORTS CONTROL PANEL").color(TEXT).strong());
            ui.add_space(GRID_UNIT * 2.0);

            let is_streaming = control_state
                .is_streaming
                .load(std::sync::atomic::Ordering::Relaxed);
            let stream_btn = if is_streaming {
                Button::new(RichText::new("[● STREAMING]").color(SURFACE_0).strong()).fill(RED_LIVE)
            } else {
                Button::new(RichText::new("START STREAM").color(TEXT)).fill(SURFACE_2)
            };
            if ui.add_sized([130.0, BUTTON_HEIGHT], stream_btn).clicked() {
                control_state
                    .is_streaming
                    .store(!is_streaming, std::sync::atomic::Ordering::Relaxed);
            }

            let is_recording = control_state
                .is_recording
                .load(std::sync::atomic::Ordering::Relaxed);
            let record_btn = if is_recording {
                Button::new(RichText::new("[● REC]").color(SURFACE_0).strong()).fill(AMBER_WARN)
            } else {
                Button::new(RichText::new("RECORD").color(TEXT)).fill(SURFACE_2)
            };
            if ui.add_sized([110.0, BUTTON_HEIGHT], record_btn).clicked() {
                control_state
                    .is_recording
                    .store(!is_recording, std::sync::atomic::Ordering::Relaxed);
            }
        });

        ui.add_space(GRID_UNIT);
        ui.separator();
        ui.add_space(GRID_UNIT);

        // --- Fast Scene Switcher Buttons ---
        ui.label(
            RichText::new("ONE-CLICK SCENE SWITCHER")
                .color(TEXT_MUTED)
                .strong(),
        );
        ui.horizontal(|ui| {
            let active_scene = control_state.current_scene.read().clone();
            let scenes = [
                "Starting Soon",
                "Live Match",
                "Caster Desk",
                "Break",
                "Ending",
            ];

            for scn in scenes {
                let is_active = active_scene == scn;
                let btn = if is_active {
                    Button::new(
                        RichText::new(format!("[▶ {}]", scn))
                            .color(SURFACE_0)
                            .strong(),
                    )
                    .fill(ACCENT)
                } else {
                    Button::new(RichText::new(scn).color(TEXT)).fill(SURFACE_2)
                };

                if ui.add_sized([130.0, BUTTON_HEIGHT], btn).clicked() {
                    *control_state.current_scene.write() = scn.to_string();
                    control_state.broadcast_ws("scene_change", serde_json::json!({ "scene": scn }));
                }
            }
        });

        ui.add_space(GRID_UNIT * 1.5);
        ui.separator();
        ui.add_space(GRID_UNIT);

        // --- Live Scoreboard Fast Controls ---
        ui.label(
            RichText::new("LIVE MATCH SCOREBOARD")
                .color(TEXT_MUTED)
                .strong(),
        );
        ui.horizontal(|ui| {
            // Team 1 Card
            ui.group(|ui| {
                ui.set_width(280.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("TEAM 1").color(TEXT_MUTED));
                    let cur_t1 = store
                        .get("scoreboard.team1.name")
                        .map(|v| v.to_display_string())
                        .unwrap_or_else(|| "Sentinels".to_string());
                    if state.team1_name.is_empty() {
                        state.team1_name = cur_t1;
                    }
                    if ui
                        .add(TextEdit::singleline(&mut state.team1_name))
                        .changed()
                    {
                        store.set("scoreboard.team1.name", state.team1_name.clone());
                    }

                    let cur_score_1 = store
                        .get("scoreboard.team1.score")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    ui.add_space(GRID_UNIT * 0.5);
                    ui.horizontal(|ui| {
                        ui.heading(
                            RichText::new(format!("{}", cur_score_1))
                                .size(36.0)
                                .color(TEXT)
                                .strong(),
                        );
                        ui.add_space(GRID_UNIT);

                        if ui
                            .add_sized(
                                [40.0, 36.0],
                                Button::new(RichText::new("-").size(20.0).color(TEXT))
                                    .fill(SURFACE_3),
                            )
                            .clicked()
                        {
                            let new_score = (cur_score_1 - 1).max(0);
                            store.set("scoreboard.team1.score", new_score);
                        }

                        if ui
                            .add_sized(
                                [50.0, 36.0],
                                Button::new(
                                    RichText::new("+").size(24.0).color(SURFACE_0).strong(),
                                )
                                .fill(ACCENT),
                            )
                            .clicked()
                        {
                            let new_score = cur_score_1 + 1;
                            store.set("scoreboard.team1.score", new_score);
                        }
                    });
                });
            });

            ui.add_space(GRID_UNIT);

            // Team 2 Card
            ui.group(|ui| {
                ui.set_width(280.0);
                ui.vertical(|ui| {
                    ui.label(RichText::new("TEAM 2").color(TEXT_MUTED));
                    let cur_t2 = store
                        .get("scoreboard.team2.name")
                        .map(|v| v.to_display_string())
                        .unwrap_or_else(|| "Fnatic".to_string());
                    if state.team2_name.is_empty() {
                        state.team2_name = cur_t2;
                    }
                    if ui
                        .add(TextEdit::singleline(&mut state.team2_name))
                        .changed()
                    {
                        store.set("scoreboard.team2.name", state.team2_name.clone());
                    }

                    let cur_score_2 = store
                        .get("scoreboard.team2.score")
                        .and_then(|v| v.as_i64())
                        .unwrap_or(0);

                    ui.add_space(GRID_UNIT * 0.5);
                    ui.horizontal(|ui| {
                        ui.heading(
                            RichText::new(format!("{}", cur_score_2))
                                .size(36.0)
                                .color(TEXT)
                                .strong(),
                        );
                        ui.add_space(GRID_UNIT);

                        if ui
                            .add_sized(
                                [40.0, 36.0],
                                Button::new(RichText::new("-").size(20.0).color(TEXT))
                                    .fill(SURFACE_3),
                            )
                            .clicked()
                        {
                            let new_score = (cur_score_2 - 1).max(0);
                            store.set("scoreboard.team2.score", new_score);
                        }

                        if ui
                            .add_sized(
                                [50.0, 36.0],
                                Button::new(
                                    RichText::new("+").size(24.0).color(SURFACE_0).strong(),
                                )
                                .fill(ACCENT),
                            )
                            .clicked()
                        {
                            let new_score = cur_score_2 + 1;
                            store.set("scoreboard.team2.score", new_score);
                        }
                    });
                });
            });
        });

        ui.add_space(GRID_UNIT * 1.5);
        ui.separator();
        ui.add_space(GRID_UNIT);

        // --- Rapid Overlay Triggers ---
        ui.label(
            RichText::new("RAPID OVERLAY TRIGGERS")
                .color(TEXT_MUTED)
                .strong(),
        );
        ui.horizontal(|ui| {
            let triggers = [
                ("Scoreboard", "scoreboard_overlay"),
                ("Lower Third", "lower_third_overlay"),
                ("Starting Soon", "starting_soon_overlay"),
                ("Kill Feed", "kill_feed_overlay"),
            ];

            for (label, id) in triggers {
                if ui
                    .add_sized(
                        [140.0, BUTTON_HEIGHT],
                        Button::new(RichText::new(format!("Toggle {}", label)).color(TEXT))
                            .fill(SURFACE_2),
                    )
                    .clicked()
                {
                    control_state
                        .overlay_triggers
                        .write()
                        .push((id.to_string(), "toggle".to_string()));
                    control_state.broadcast_ws(
                        "overlay_trigger",
                        serde_json::json!({ "overlay_id": id, "action": "toggle" }),
                    );
                }
            }
        });
    });
}
