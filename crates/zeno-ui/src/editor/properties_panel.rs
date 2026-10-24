//! Properties inspector: Transform (Pos/Scale/Rot/Opacity) with keyframe stopwatch toggles,
//! content-specific parameters (Text, Shape, Camera, Particles, Image), and timing controls.

use egui::{DragValue, Slider, Ui};
use zeno_core::{Color, Easing, Property, Vec3};
use zeno_engine::{Fill, LayerContent, ShapeGeometry, Stroke, TextAlignment};

use crate::editor::state::OverlayEditor;
use crate::theme::{color_to_egui, egui_to_color, RED_LIVE, TEXT, TEXT_MUTED};

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

    let selected_asset_info = if let Some(l) = editor
        .composition
        .layers
        .iter()
        .find(|l| l.id == selected_id)
    {
        if let LayerContent::Image { asset } = l.content {
            let asset_ref = editor
                .composition
                .assets
                .iter()
                .find(|a| a.id == asset)
                .cloned();
            let dims = editor.asset_dimensions.get(&asset).copied();
            Some((asset, asset_ref, dims))
        } else {
            None
        }
    } else {
        None
    };

    let mut add_pos_keyframe: Option<Vec3> = None;
    let mut add_scale_keyframe: Option<Vec3> = None;
    let mut add_rot_keyframe: Option<Vec3> = None;
    let mut add_op_keyframe: Option<f32> = None;
    let mut trigger_reset_native_scale: Option<(f32, f32)> = None;

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

        ui.horizontal(|ui| {
            ui.heading(&layer.name);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.checkbox(&mut layer.visible, "Visible");
            });
        });
        ui.separator();

        // 1. Transform Section
        ui.collapsing("Transform", |ui| {
            ui.checkbox(&mut layer.is_3d, "3D Layer (2.5D)");

            // Position
            ui.horizontal(|ui| {
                let is_anim = layer.transform.position.is_animated();
                let stop_label = if is_anim {
                    "Stopwatch (*)"
                } else {
                    "Stopwatch (o)"
                };
                if ui
                    .button(stop_label)
                    .on_hover_text("Add position keyframe at playhead")
                    .clicked()
                {
                    add_pos_keyframe = Some(*layer.transform.position.fallback_value());
                }

                ui.label("Position X:");
                let mut pos = *layer.transform.position.fallback_value();
                let mut changed = ui.add(DragValue::new(&mut pos.x).speed(1.0)).changed();
                ui.label("Y:");
                changed |= ui.add(DragValue::new(&mut pos.y).speed(1.0)).changed();
                if layer.is_3d {
                    ui.label("Z:");
                    changed |= ui.add(DragValue::new(&mut pos.z).speed(1.0)).changed();
                }
                if changed {
                    layer.transform.position = Property::constant(pos);
                }
            });

            // Scale
            ui.horizontal(|ui| {
                let is_anim = layer.transform.scale_percent.is_animated();
                let stop_label = if is_anim {
                    "Stopwatch (*)"
                } else {
                    "Stopwatch (o)"
                };
                if ui
                    .button(stop_label)
                    .on_hover_text("Add scale keyframe at playhead")
                    .clicked()
                {
                    add_scale_keyframe = Some(*layer.transform.scale_percent.fallback_value());
                }

                ui.label("Scale X (%):");
                let mut scale = *layer.transform.scale_percent.fallback_value();
                let mut changed = ui
                    .add(DragValue::new(&mut scale.x).speed(1.0).range(0.0..=1000.0))
                    .changed();
                ui.label("Y (%):");
                changed |= ui
                    .add(DragValue::new(&mut scale.y).speed(1.0).range(0.0..=1000.0))
                    .changed();
                if layer.is_3d {
                    ui.label("Z (%):");
                    changed |= ui
                        .add(DragValue::new(&mut scale.z).speed(1.0).range(0.0..=1000.0))
                        .changed();
                }
                if changed {
                    layer.transform.scale_percent = Property::constant(scale);
                }
            });

            // Rotation
            ui.horizontal(|ui| {
                let is_anim = layer.transform.rotation_degrees.is_animated();
                let stop_label = if is_anim {
                    "Stopwatch (*)"
                } else {
                    "Stopwatch (o)"
                };
                if ui
                    .button(stop_label)
                    .on_hover_text("Add rotation keyframe at playhead")
                    .clicked()
                {
                    add_rot_keyframe = Some(*layer.transform.rotation_degrees.fallback_value());
                }

                let mut rot = *layer.transform.rotation_degrees.fallback_value();
                let mut changed = false;
                if layer.is_3d {
                    ui.label("Rot X:");
                    changed |= ui.add(DragValue::new(&mut rot.x).speed(0.5)).changed();
                    ui.label("Y (Flip):");
                    changed |= ui.add(DragValue::new(&mut rot.y).speed(0.5)).changed();
                    ui.label("Z:");
                    changed |= ui.add(DragValue::new(&mut rot.z).speed(0.5)).changed();
                } else {
                    ui.label("Rotation Z (deg):");
                    changed |= ui.add(DragValue::new(&mut rot.z).speed(0.5)).changed();
                }
                if changed {
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
                if ui
                    .button(stop_label)
                    .on_hover_text("Add opacity keyframe at playhead")
                    .clicked()
                {
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
                        ui.label("Font Family:");
                        ui.text_edit_singleline(&mut text_content.font_family);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Font Size:");
                        let mut sz = *text_content.font_size.fallback_value();
                        if ui
                            .add(DragValue::new(&mut sz).speed(1.0).range(8.0..=300.0))
                            .changed()
                        {
                            text_content.font_size = Property::constant(sz);
                        }
                    });

                    // Color picker
                    ui.horizontal(|ui| {
                        ui.label("Color:");
                        let mut c32 = color_to_egui(*text_content.color.fallback_value());
                        if ui.color_edit_button_srgba(&mut c32).changed() {
                            text_content.color = Property::constant(egui_to_color(c32));
                        }
                    });

                    // Alignment
                    ui.horizontal(|ui| {
                        ui.label("Alignment:");
                        if ui
                            .selectable_label(text_content.alignment == TextAlignment::Left, "Left")
                            .clicked()
                        {
                            text_content.alignment = TextAlignment::Left;
                        }
                        if ui
                            .selectable_label(
                                text_content.alignment == TextAlignment::Center,
                                "Center",
                            )
                            .clicked()
                        {
                            text_content.alignment = TextAlignment::Center;
                        }
                        if ui
                            .selectable_label(
                                text_content.alignment == TextAlignment::Right,
                                "Right",
                            )
                            .clicked()
                        {
                            text_content.alignment = TextAlignment::Right;
                        }
                    });

                    // Letter and line spacing
                    ui.horizontal(|ui| {
                        ui.label("Letter Spacing (px):");
                        let mut ls = *text_content.letter_spacing.fallback_value();
                        if ui
                            .add(DragValue::new(&mut ls).speed(0.5).range(-10.0..=100.0))
                            .changed()
                        {
                            text_content.letter_spacing = Property::constant(ls);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Line Height (mult):");
                        let mut lh = *text_content.line_height_multiplier.fallback_value();
                        if ui
                            .add(DragValue::new(&mut lh).speed(0.05).range(0.5..=4.0))
                            .changed()
                        {
                            text_content.line_height_multiplier = Property::constant(lh);
                        }
                    });

                    ui.checkbox(&mut text_content.word_wrap, "Word Wrap");
                });
            }
            LayerContent::Shape(shape_content) => {
                ui.collapsing("Shape Properties", |ui| {
                    let mut switch_to_rounded: Option<(Property<zeno_core::Vec2>, f32)> = None;

                    // Geometry dimensions and corner radius
                    match &mut shape_content.geometry {
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

                            let mut cr = 0.0_f32;
                            ui.horizontal(|ui| {
                                ui.label("Corner Radius:");
                                if ui
                                    .add(DragValue::new(&mut cr).speed(0.5).range(0.0..=200.0))
                                    .changed()
                                    && cr > 0.0
                                {
                                    switch_to_rounded = Some((size.clone(), cr));
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
                    }

                    if let Some((size, cr)) = switch_to_rounded {
                        shape_content.geometry = ShapeGeometry::RoundedRectangle {
                            size,
                            corner_radius: Property::constant(cr),
                        };
                    }

                    ui.separator();

                    // Fill Color
                    let has_fill = shape_content.fill.is_some();
                    let mut enable_fill = has_fill;
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut enable_fill, "Fill").changed() {
                            if enable_fill {
                                shape_content.fill = Some(Fill::Solid(Property::constant(
                                    Color::rgba(0.2, 0.4, 0.8, 1.0),
                                )));
                            } else {
                                shape_content.fill = None;
                            }
                        }

                        if let Some(Fill::Solid(ref mut color_prop)) = shape_content.fill {
                            let mut c32 = color_to_egui(*color_prop.fallback_value());
                            if ui.color_edit_button_srgba(&mut c32).changed() {
                                *color_prop = Property::constant(egui_to_color(c32));
                            }
                        }
                    });

                    // Stroke
                    let has_stroke = shape_content.stroke.is_some();
                    let mut enable_stroke = has_stroke;
                    ui.horizontal(|ui| {
                        if ui.checkbox(&mut enable_stroke, "Stroke").changed() {
                            if enable_stroke {
                                shape_content.stroke = Some(Stroke {
                                    width: Property::constant(2.0),
                                    color: Property::constant(Color::WHITE),
                                });
                            } else {
                                shape_content.stroke = None;
                            }
                        }

                        if let Some(ref mut stroke) = shape_content.stroke {
                            let mut c32 = color_to_egui(*stroke.color.fallback_value());
                            if ui.color_edit_button_srgba(&mut c32).changed() {
                                stroke.color = Property::constant(egui_to_color(c32));
                            }

                            ui.label("Width:");
                            let mut w = *stroke.width.fallback_value();
                            if ui
                                .add(DragValue::new(&mut w).speed(0.5).range(0.5..=100.0))
                                .changed()
                            {
                                stroke.width = Property::constant(w);
                            }
                        }
                    });
                });
            }
            LayerContent::Camera(cam) => {
                ui.collapsing("Camera Settings", |ui| {
                    ui.horizontal(|ui| {
                        let mut fov = *cam.field_of_view_degrees.fallback_value();
                        ui.label("Field of View (deg):");
                        if ui.add(Slider::new(&mut fov, 10.0..=120.0)).changed() {
                            cam.field_of_view_degrees = Property::constant(fov);
                        }
                    });

                    ui.horizontal(|ui| {
                        let mut pos = *cam.position.fallback_value();
                        ui.label("Position X:");
                        let mut c = ui.add(DragValue::new(&mut pos.x).speed(1.0)).changed();
                        ui.label("Y:");
                        c |= ui.add(DragValue::new(&mut pos.y).speed(1.0)).changed();
                        ui.label("Z:");
                        c |= ui.add(DragValue::new(&mut pos.z).speed(1.0)).changed();
                        if c {
                            cam.position = Property::constant(pos);
                        }
                    });

                    ui.horizontal(|ui| {
                        let mut tgt = *cam.target.fallback_value();
                        ui.label("Target X:");
                        let mut c = ui.add(DragValue::new(&mut tgt.x).speed(1.0)).changed();
                        ui.label("Y:");
                        c |= ui.add(DragValue::new(&mut tgt.y).speed(1.0)).changed();
                        ui.label("Z:");
                        c |= ui.add(DragValue::new(&mut tgt.z).speed(1.0)).changed();
                        if c {
                            cam.target = Property::constant(tgt);
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Near Plane:");
                        ui.add(
                            DragValue::new(&mut cam.near_plane)
                                .speed(0.1)
                                .range(0.01..=100.0),
                        );
                        ui.label("Far Plane:");
                        ui.add(
                            DragValue::new(&mut cam.far_plane)
                                .speed(10.0)
                                .range(100.0..=100000.0),
                        );
                    });
                });
            }
            LayerContent::Particles(emitter) => {
                ui.collapsing("GPU Particle System", |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Presets:");
                        if ui.button("Confetti").clicked() {
                            *emitter = zeno_engine::ParticleEmitter::confetti();
                        }
                        if ui.button("Sparks").clicked() {
                            *emitter = zeno_engine::ParticleEmitter::sparks();
                        }
                        if ui.button("Smoke").clicked() {
                            *emitter = zeno_engine::ParticleEmitter::smoke();
                        }
                    });

                    ui.horizontal(|ui| {
                        let mut rate = *emitter.rate_per_second.fallback_value();
                        ui.label("Rate (parts/s):");
                        if ui
                            .add(DragValue::new(&mut rate).speed(5.0).range(1.0..=50000.0))
                            .changed()
                        {
                            emitter.rate_per_second = Property::constant(rate);
                        }
                    });

                    ui.horizontal(|ui| {
                        let mut lt = *emitter.lifetime_seconds.fallback_value();
                        ui.label("Lifetime (s):");
                        if ui
                            .add(DragValue::new(&mut lt).speed(0.1).range(0.05..=30.0))
                            .changed()
                        {
                            emitter.lifetime_seconds = Property::constant(lt);
                        }
                    });

                    ui.horizontal(|ui| {
                        let mut spd = *emitter.initial_speed.fallback_value();
                        ui.label("Speed (px/s):");
                        if ui
                            .add(DragValue::new(&mut spd).speed(5.0).range(0.0..=5000.0))
                            .changed()
                        {
                            emitter.initial_speed = Property::constant(spd);
                        }
                    });

                    ui.horizontal(|ui| {
                        let mut spread = *emitter.spread_degrees.fallback_value();
                        ui.label("Spread (deg):");
                        if ui.add(Slider::new(&mut spread, 0.0..=360.0)).changed() {
                            emitter.spread_degrees = Property::constant(spread);
                        }
                    });

                    ui.horizontal(|ui| {
                        let mut s_sz = *emitter.start_size_px.fallback_value();
                        let mut e_sz = *emitter.end_size_px.fallback_value();
                        ui.label("Start Size:");
                        let mut c = ui
                            .add(DragValue::new(&mut s_sz).speed(0.5).range(0.0..=300.0))
                            .changed();
                        ui.label("End Size:");
                        c |= ui
                            .add(DragValue::new(&mut e_sz).speed(0.5).range(0.0..=300.0))
                            .changed();
                        if c {
                            emitter.start_size_px = Property::constant(s_sz);
                            emitter.end_size_px = Property::constant(e_sz);
                        }
                    });

                    // Start Color and End Color pickers
                    ui.horizontal(|ui| {
                        ui.label("Start Color:");
                        let mut sc32 = color_to_egui(*emitter.start_color.fallback_value());
                        if ui.color_edit_button_srgba(&mut sc32).changed() {
                            emitter.start_color = Property::constant(egui_to_color(sc32));
                        }

                        ui.label("End Color:");
                        let mut ec32 = color_to_egui(*emitter.end_color.fallback_value());
                        if ui.color_edit_button_srgba(&mut ec32).changed() {
                            emitter.end_color = Property::constant(egui_to_color(ec32));
                        }
                    });

                    ui.horizontal(|ui| {
                        let mut grav = *emitter.gravity.fallback_value();
                        ui.label("Gravity X:");
                        let mut c = ui.add(DragValue::new(&mut grav.x).speed(5.0)).changed();
                        ui.label("Y:");
                        c |= ui.add(DragValue::new(&mut grav.y).speed(5.0)).changed();
                        if c {
                            emitter.gravity = Property::constant(grav);
                        }
                    });

                    ui.checkbox(&mut emitter.additive_blend, "Additive Blend");

                    ui.horizontal(|ui| {
                        ui.label("Seed:");
                        ui.add(DragValue::new(&mut emitter.seed));
                    });
                });
            }
            LayerContent::Image { asset } => {
                ui.collapsing("Image Asset", |ui| {
                    if let Some((_, Some(ref a_ref), dims)) = selected_asset_info {
                        ui.label(format!("Path: {}", a_ref.path));
                        if let Some((w, h)) = dims {
                            ui.label(format!("Native Dimensions: {}x{}", w, h));
                            if ui.button("Reset Native Size").clicked() {
                                trigger_reset_native_scale = Some((w as f32, h as f32));
                            }
                        }
                        let sz = if a_ref.size_bytes >= 1024 * 1024 {
                            format!("{:.2} MB", a_ref.size_bytes as f64 / (1024.0 * 1024.0))
                        } else {
                            format!("{:.1} KB", a_ref.size_bytes as f64 / 1024.0)
                        };
                        ui.label(format!("File Size: {}", sz));
                    } else {
                        ui.label(format!("Asset ID: {}", asset.0));
                    }
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

        ui.separator();

        // 4. Expressions & Scripting Section
        ui.collapsing("Expressions & Scripting", |ui| {
            ui.colored_label(
                TEXT_MUTED,
                "Sandboxed Rhai expressions evaluated dynamically per frame (e.g. wiggle(5.0, 20.0), data(\"team.score\")).",
            );

            // Opacity Expression toggle
            let is_expr_opacity = matches!(layer.transform.opacity_percent, Property::Expression { .. });
            let mut toggle_op = is_expr_opacity;
            if ui.checkbox(&mut toggle_op, "Expression on Opacity").changed() {
                if toggle_op {
                    let cur = *layer.transform.opacity_percent.fallback_value();
                    layer.transform.opacity_percent = Property::expression("wiggle(2.0, 30.0)", cur);
                } else {
                    let cur = *layer.transform.opacity_percent.fallback_value();
                    layer.transform.opacity_percent = Property::constant(cur);
                }
            }
            if let Property::Expression { source, .. } = &mut layer.transform.opacity_percent {
                ui.horizontal(|ui| {
                    ui.label("Expr:");
                    ui.text_edit_singleline(source);
                });
                let runtime = zeno_engine::get_script_runtime();
                match runtime.compile(source) {
                    Ok(_) => {
                        ui.colored_label(TEXT, "Expression syntax valid");
                    }
                    Err(e) => {
                        ui.colored_label(RED_LIVE, format!("Error: {e}"));
                    }
                }
            }

            // Position Expression toggle
            let is_expr_pos = matches!(layer.transform.position, Property::Expression { .. });
            let mut toggle_pos = is_expr_pos;
            if ui.checkbox(&mut toggle_pos, "Expression on Position").changed() {
                if toggle_pos {
                    let cur = *layer.transform.position.fallback_value();
                    layer.transform.position = Property::expression("[value[0] + wiggle(3.0, 15.0), value[1], value[2]]", cur);
                } else {
                    let cur = *layer.transform.position.fallback_value();
                    layer.transform.position = Property::constant(cur);
                }
            }
            if let Property::Expression { source, .. } = &mut layer.transform.position {
                ui.horizontal(|ui| {
                    ui.label("Expr:");
                    ui.text_edit_singleline(source);
                });
                let runtime = zeno_engine::get_script_runtime();
                match runtime.compile(source) {
                    Ok(_) => {
                        ui.colored_label(TEXT, "Expression syntax valid");
                    }
                    Err(e) => {
                        ui.colored_label(RED_LIVE, format!("Error: {e}"));
                    }
                }
            }
        });
    });

    if let Some(pos) = add_pos_keyframe {
        editor.add_position_keyframe(pos, Easing::ease());
    }
    if let Some(scale) = add_scale_keyframe {
        editor.add_scale_keyframe(scale, Easing::ease());
    }
    if let Some(rot) = add_rot_keyframe {
        editor.add_rotation_keyframe(rot, Easing::ease());
    }
    if let Some(op) = add_op_keyframe {
        editor.add_opacity_keyframe(op, Easing::ease());
    }
    if let Some((_w, _h)) = trigger_reset_native_scale {
        if let Some(layer) = editor
            .composition
            .layers
            .iter_mut()
            .find(|l| l.id == selected_id)
        {
            layer.transform.scale_percent = Property::constant(Vec3::splat(100.0));
        }
    }
}
