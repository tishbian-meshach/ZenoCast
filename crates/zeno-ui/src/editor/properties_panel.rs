//! Properties inspector: Transform (Pos/Scale/Rot/Opacity) with keyframe stopwatch toggles,
//! content-specific parameters (Text, Shape), and timing controls.

use egui::{DragValue, Slider, Ui};
use zeno_core::{Easing, Property, Vec3};
use zeno_engine::{LayerContent, ShapeGeometry};

use crate::editor::state::OverlayEditor;
use crate::theme::TEXT_MUTED;

/// Renders the properties inspector for the currently selected layer.
pub fn ui_properties_panel(editor: &mut OverlayEditor, ui: &mut Ui) {
    let selected_id = match editor.selected_layer_id {
        Some(id) => id,
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.colored_label(TEXT_MUTED, "No layer selected");
            });
            return;
        }
    };

    let mut add_pos_keyframe: Option<Vec3> = None;
    let mut add_op_keyframe: Option<f32> = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        let layer_opt = editor
            .composition
            .layers
            .iter_mut()
            .find(|l| l.id == selected_id);
        let layer = match layer_opt {
            Some(l) => l,
            None => return,
        };

        ui.heading(&layer.name);
        ui.separator();

        // 1. Transform Section
        ui.collapsing("Transform", |ui| {
            // Position
            ui.horizontal(|ui| {
                let is_anim = layer.transform.position.is_animated();
                let stop_label = if is_anim {
                    "Stopwatch (*)"
                } else {
                    "Stopwatch (o)"
                };
                if ui.button(stop_label).clicked() {
                    add_pos_keyframe = Some(*layer.transform.position.fallback_value());
                }

                ui.label("Position X:");
                let mut pos = *layer.transform.position.fallback_value();
                if ui.add(DragValue::new(&mut pos.x).speed(1.0)).changed() {
                    layer.transform.position = Property::constant(pos);
                }
                ui.label("Y:");
                if ui.add(DragValue::new(&mut pos.y).speed(1.0)).changed() {
                    layer.transform.position = Property::constant(pos);
                }
            });

            // Scale
            ui.horizontal(|ui| {
                ui.label("Scale X (%):");
                let mut scale = *layer.transform.scale_percent.fallback_value();
                if ui
                    .add(DragValue::new(&mut scale.x).speed(1.0).range(0.0..=1000.0))
                    .changed()
                {
                    layer.transform.scale_percent = Property::constant(scale);
                }
                ui.label("Y (%):");
                if ui
                    .add(DragValue::new(&mut scale.y).speed(1.0).range(0.0..=1000.0))
                    .changed()
                {
                    layer.transform.scale_percent = Property::constant(scale);
                }
            });

            // Rotation
            ui.horizontal(|ui| {
                ui.label("Rotation Z (deg):");
                let mut rot = *layer.transform.rotation_degrees.fallback_value();
                if ui.add(DragValue::new(&mut rot.z).speed(0.5)).changed() {
                    layer.transform.rotation_degrees = Property::constant(rot);
                }
            });

            // Opacity
            ui.horizontal(|ui| {
                let is_anim = layer.transform.opacity_percent.is_animated();
                let stop_label = if is_anim {
                    "Stopwatch (*)"
                } else {
                    "Stopwatch (o)"
                };
                if ui.button(stop_label).clicked() {
                    add_op_keyframe = Some(*layer.transform.opacity_percent.fallback_value());
                }

                ui.label("Opacity (%):");
                let mut op = *layer.transform.opacity_percent.fallback_value();
                if ui.add(Slider::new(&mut op, 0.0..=100.0)).changed() {
                    layer.transform.opacity_percent = Property::constant(op);
                }
            });
        });

        ui.separator();

        // 2. Content Specific Section
        match &mut layer.content {
            LayerContent::Text(text_content) => {
                ui.collapsing("Text Properties", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Text:");
                        let mut txt = text_content.text.fallback_value().clone();
                        if ui.text_edit_singleline(&mut txt).changed() {
                            text_content.text = Property::constant(txt);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Font Size:");
                        let mut sz = *text_content.font_size.fallback_value();
                        if ui
                            .add(DragValue::new(&mut sz).speed(1.0).range(8.0..=200.0))
                            .changed()
                        {
                            text_content.font_size = Property::constant(sz);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Font Family:");
                        ui.text_edit_singleline(&mut text_content.font_family);
                    });
                });
            }
            LayerContent::Shape(shape_content) => {
                ui.collapsing("Shape Properties", |ui| match &mut shape_content.geometry {
                    ShapeGeometry::Rectangle { size } => {
                        ui.horizontal(|ui| {
                            let mut s = *size.fallback_value();
                            ui.label("Width:");
                            let c1 = ui
                                .add(DragValue::new(&mut s.x).speed(1.0).range(1.0..=3840.0))
                                .changed();
                            ui.label("Height:");
                            let c2 = ui
                                .add(DragValue::new(&mut s.y).speed(1.0).range(1.0..=2160.0))
                                .changed();
                            if c1 || c2 {
                                *size = Property::constant(s);
                            }
                        });
                    }
                    ShapeGeometry::RoundedRectangle {
                        size,
                        corner_radius,
                    } => {
                        ui.horizontal(|ui| {
                            let mut s = *size.fallback_value();
                            ui.label("Width:");
                            let c1 = ui
                                .add(DragValue::new(&mut s.x).speed(1.0).range(1.0..=3840.0))
                                .changed();
                            ui.label("Height:");
                            let c2 = ui
                                .add(DragValue::new(&mut s.y).speed(1.0).range(1.0..=2160.0))
                                .changed();
                            if c1 || c2 {
                                *size = Property::constant(s);
                            }
                        });
                        ui.horizontal(|ui| {
                            let mut cr = *corner_radius.fallback_value();
                            ui.label("Corner Radius:");
                            if ui
                                .add(DragValue::new(&mut cr).speed(0.5).range(0.0..=200.0))
                                .changed()
                            {
                                *corner_radius = Property::constant(cr);
                            }
                        });
                    }
                    ShapeGeometry::Ellipse { radius } => {
                        ui.horizontal(|ui| {
                            let mut r = *radius.fallback_value();
                            ui.label("Radius X:");
                            let c1 = ui
                                .add(DragValue::new(&mut r.x).speed(1.0).range(1.0..=2000.0))
                                .changed();
                            ui.label("Radius Y:");
                            let c2 = ui
                                .add(DragValue::new(&mut r.y).speed(1.0).range(1.0..=2000.0))
                                .changed();
                            if c1 || c2 {
                                *radius = Property::constant(r);
                            }
                        });
                    }
                    _ => {}
                });
            }
            _ => {}
        }

        ui.separator();

        // 3. Timing Section (In/Out Point)
        ui.collapsing("Timing", |ui| {
            ui.horizontal(|ui| {
                ui.label("In Point (s):");
                ui.add(
                    DragValue::new(&mut layer.in_point_seconds)
                        .speed(0.1)
                        .range(0.0..=layer.out_point_seconds),
                );
            });
            ui.horizontal(|ui| {
                ui.label("Out Point (s):");
                ui.add(
                    DragValue::new(&mut layer.out_point_seconds)
                        .speed(0.1)
                        .range(layer.in_point_seconds..=1000.0),
                );
            });
        });
    });

    if let Some(pos) = add_pos_keyframe {
        editor.add_position_keyframe(pos, Easing::ease());
    }
    if let Some(op) = add_op_keyframe {
        editor.add_opacity_keyframe(op, Easing::ease());
    }
}
