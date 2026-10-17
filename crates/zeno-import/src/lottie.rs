//! Lottie / Bodymovin JSON animation importer.
//!
//! Converts Lottie compositions, shape layers, transforms, and Bezier keyframes
//! into native `zeno-engine` layers, properties, and easing animations.

use serde_json::Value;
use std::collections::HashMap;
use zeno_core::{Color, Easing, Keyframe, LayerId, Property, Vec2, Vec3};
use zeno_engine::{
    BlendMode, Composition, Fill, Layer, LayerContent, ShapeContent, ShapeGeometry, Stroke,
    TextContent,
};

use crate::error::ImportError;
use crate::report::ImportReport;

/// Standalone Lottie animation importer.
pub struct LottieImporter;

impl LottieImporter {
    /// Ingests a Lottie JSON string and converts it into a native `Composition` and `ImportReport`.
    pub fn import_str(json_str: &str) -> Result<(Composition, ImportReport), ImportError> {
        let root: Value = serde_json::from_str(json_str)?;
        let mut report = ImportReport::new("animation.json", "Lottie");

        let width = root.get("w").and_then(|v| v.as_u64()).unwrap_or(1920) as u32;
        let height = root.get("h").and_then(|v| v.as_u64()).unwrap_or(1080) as u32;
        let fps = root.get("fr").and_then(|v| v.as_f64()).unwrap_or(60.0) as f32;
        let in_point_frames = root.get("ip").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let out_point_frames = root.get("op").and_then(|v| v.as_f64()).unwrap_or(180.0);

        let duration_seconds = (out_point_frames - in_point_frames).max(1.0) / (fps as f64);
        let mut comp = Composition::new(width, height, fps, duration_seconds);

        let empty_vec = Vec::new();
        let layers_array = root
            .get("layers")
            .and_then(|l| l.as_array())
            .unwrap_or(&empty_vec);

        // Map Lottie 1-based or 0-based layer index `ind` to generated LayerId
        let mut index_to_layer_id: HashMap<u32, LayerId> = HashMap::new();
        let mut layer_parents: HashMap<LayerId, u32> = HashMap::new();

        for (l_idx, l_val) in layers_array.iter().enumerate() {
            let ind = l_val
                .get("ind")
                .and_then(|i| i.as_u64())
                .unwrap_or(l_idx as u64) as u32;

            let name = l_val
                .get("nm")
                .and_then(|n| n.as_str())
                .unwrap_or("LottieLayer")
                .to_string();

            let layer_type = l_val.get("ty").and_then(|t| t.as_u64()).unwrap_or(4);
            let layer_id = LayerId::new();
            index_to_layer_id.insert(ind, layer_id);

            if let Some(parent_idx) = l_val.get("parent").and_then(|p| p.as_u64()) {
                layer_parents.insert(layer_id, parent_idx as u32);
            }

            // Layer In / Out time
            let ip = l_val
                .get("ip")
                .and_then(|v| v.as_f64())
                .unwrap_or(in_point_frames)
                / (fps as f64);
            let op = l_val
                .get("op")
                .and_then(|v| v.as_f64())
                .unwrap_or(out_point_frames)
                / (fps as f64);

            // Layer Transforms (ks)
            let ks = l_val.get("ks");
            let pos_prop = parse_vec3_property(ks.and_then(|k| k.get("p")), fps, Vec3::ZERO);
            let scale_prop = parse_vec3_property(
                ks.and_then(|k| k.get("s")),
                fps,
                Vec3::new(100.0, 100.0, 100.0),
            );
            let rot_prop = parse_rotation_property(ks.and_then(|k| k.get("r")), fps);
            let opacity_prop = parse_scalar_property(ks.and_then(|k| k.get("o")), fps, 100.0);
            let anchor_prop = parse_vec3_property(ks.and_then(|k| k.get("a")), fps, Vec3::ZERO);

            // Check `bind:` prefix
            let mut bound_path = None;
            if let Some(path) = name.strip_prefix("bind:") {
                let p = path.trim().to_string();
                report.add_binding(&name, &p);
                bound_path = Some(p);
            }

            // Determine content
            let content = match layer_type {
                1 => {
                    // Solid Color Layer
                    let sw = l_val.get("sw").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
                    let sh = l_val.get("sh").and_then(|v| v.as_f64()).unwrap_or(100.0) as f32;
                    let hex_color = l_val
                        .get("sc")
                        .and_then(|v| v.as_str())
                        .unwrap_or("#FFFFFF");
                    let color = parse_hex_color(hex_color);

                    LayerContent::Shape(ShapeContent {
                        geometry: ShapeGeometry::Rectangle {
                            size: Property::constant(Vec2::new(sw, sh)),
                        },
                        fill: Some(Fill::Solid(Property::constant(color))),
                        stroke: None,
                    })
                }
                4 => {
                    // Shape Layer
                    parse_shape_content(l_val.get("shapes"))
                }
                5 => {
                    // Text Layer
                    let text_str = if let Some(ref bp) = bound_path {
                        format!("{{{}}}", bp)
                    } else {
                        extract_lottie_text(l_val)
                    };

                    LayerContent::Text(TextContent {
                        text: Property::constant(text_str),
                        font_family: "Inter".to_string(),
                        font_size: Property::constant(48.0),
                        color: Property::constant(Color::WHITE),
                        ..Default::default()
                    })
                }
                _ => {
                    // Precomp / Null / Generic
                    LayerContent::Null
                }
            };

            let mut layer = Layer::new(layer_id, name, content);
            layer.in_point_seconds = ip;
            layer.out_point_seconds = op;
            layer.transform.position = pos_prop;
            layer.transform.scale_percent = scale_prop;
            layer.transform.rotation_degrees = rot_prop;
            layer.transform.opacity_percent = opacity_prop;
            layer.transform.anchor_point = anchor_prop;
            layer.blend_mode = BlendMode::Normal;

            comp.layers.push(layer);
            report.imported_layers += 1;
        }

        // Resolve parent references
        for layer in &mut comp.layers {
            if let Some(&p_idx) = layer_parents.get(&layer.id) {
                if let Some(&p_id) = index_to_layer_id.get(&p_idx) {
                    layer.parent = Some(p_id);
                }
            }
        }

        report.success = true;
        Ok((comp, report))
    }
}

/// Parses a 3D vector property from Lottie JSON, supporting static values and keyframe easing.
fn parse_vec3_property(val: Option<&Value>, fps: f32, default: Vec3) -> Property<Vec3> {
    let prop_obj = match val {
        Some(v) => v,
        None => return Property::constant(default),
    };

    let is_animated = prop_obj.get("a").and_then(|a| a.as_u64()).unwrap_or(0) == 1;
    let k_val = match prop_obj.get("k") {
        Some(k) => k,
        None => return Property::constant(default),
    };

    if !is_animated {
        if let Some(arr) = k_val.as_array() {
            let x = arr
                .first()
                .and_then(|v| v.as_f64())
                .unwrap_or(default.x as f64) as f32;
            let y = arr
                .get(1)
                .and_then(|v| v.as_f64())
                .unwrap_or(default.y as f64) as f32;
            let z = arr
                .get(2)
                .and_then(|v| v.as_f64())
                .unwrap_or(default.z as f64) as f32;
            return Property::constant(Vec3::new(x, y, z));
        }
        return Property::constant(default);
    }

    // Parse Keyframe Array
    if let Some(kf_array) = k_val.as_array() {
        let mut keyframes = Vec::new();
        for kf_obj in kf_array {
            let time_frames = kf_obj.get("t").and_then(|t| t.as_f64()).unwrap_or(0.0);
            let time_secs = (time_frames / (fps as f64)).max(0.0);

            let s_val = kf_obj.get("s").and_then(|s| s.as_array());
            let vec_val = if let Some(arr) = s_val {
                let x = arr
                    .first()
                    .and_then(|v| v.as_f64())
                    .unwrap_or(default.x as f64) as f32;
                let y = arr
                    .get(1)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(default.y as f64) as f32;
                let z = arr
                    .get(2)
                    .and_then(|v| v.as_f64())
                    .unwrap_or(default.z as f64) as f32;
                Vec3::new(x, y, z)
            } else {
                default
            };

            // Interpolation / Easing
            let interp = if kf_obj.get("h").and_then(|h| h.as_u64()).unwrap_or(0) == 1 {
                Easing::Hold
            } else if let (Some(i_obj), Some(o_obj)) = (kf_obj.get("i"), kf_obj.get("o")) {
                let ix = i_obj
                    .get("x")
                    .and_then(|v| v.as_f64())
                    .or_else(|| {
                        i_obj
                            .as_array()
                            .and_then(|a| a.first())
                            .and_then(|v| v.as_f64())
                    })
                    .unwrap_or(0.33) as f32;
                let iy = i_obj
                    .get("y")
                    .and_then(|v| v.as_f64())
                    .or_else(|| {
                        i_obj
                            .as_array()
                            .and_then(|a| a.first())
                            .and_then(|v| v.as_f64())
                    })
                    .unwrap_or(0.33) as f32;
                let ox = o_obj
                    .get("x")
                    .and_then(|v| v.as_f64())
                    .or_else(|| {
                        o_obj
                            .as_array()
                            .and_then(|a| a.first())
                            .and_then(|v| v.as_f64())
                    })
                    .unwrap_or(0.67) as f32;
                let oy = o_obj
                    .get("y")
                    .and_then(|v| v.as_f64())
                    .or_else(|| {
                        o_obj
                            .as_array()
                            .and_then(|a| a.first())
                            .and_then(|v| v.as_f64())
                    })
                    .unwrap_or(0.67) as f32;

                Easing::CubicBezier {
                    control_1_x: ix.clamp(0.0, 1.0),
                    control_1_y: iy,
                    control_2_x: ox.clamp(0.0, 1.0),
                    control_2_y: oy,
                }
            } else {
                Easing::Linear
            };

            keyframes.push(Keyframe::new(time_secs, vec_val, interp));
        }

        if !keyframes.is_empty() {
            return Property::keyframes(keyframes);
        }
    }

    Property::constant(default)
}

/// Parses Lottie rotation (degrees around Z axis) into a 3D Euler angles property `Property<Vec3>`.
fn parse_rotation_property(val: Option<&Value>, fps: f32) -> Property<Vec3> {
    let prop_obj = match val {
        Some(v) => v,
        None => return Property::constant(Vec3::ZERO),
    };

    let is_animated = prop_obj.get("a").and_then(|a| a.as_u64()).unwrap_or(0) == 1;
    let k_val = match prop_obj.get("k") {
        Some(k) => k,
        None => return Property::constant(Vec3::ZERO),
    };

    if !is_animated {
        let deg = k_val.as_f64().unwrap_or(0.0) as f32;
        return Property::constant(Vec3::new(0.0, 0.0, deg));
    }

    if let Some(kf_array) = k_val.as_array() {
        let mut keyframes = Vec::new();
        for kf_obj in kf_array {
            let time_frames = kf_obj.get("t").and_then(|t| t.as_f64()).unwrap_or(0.0);
            let time_secs = (time_frames / (fps as f64)).max(0.0);

            let s_deg = kf_obj
                .get("s")
                .and_then(|s| {
                    if let Some(arr) = s.as_array() {
                        arr.first().and_then(|v| v.as_f64())
                    } else {
                        s.as_f64()
                    }
                })
                .unwrap_or(0.0) as f32;

            keyframes.push(Keyframe::new(
                time_secs,
                Vec3::new(0.0, 0.0, s_deg),
                Easing::Linear,
            ));
        }

        if !keyframes.is_empty() {
            return Property::keyframes(keyframes);
        }
    }

    Property::constant(Vec3::ZERO)
}

/// Parses a 2D vector property from Lottie JSON.
fn parse_vec2_property(val: Option<&Value>, _fps: f32, default: Vec2) -> Property<Vec2> {
    if let Some(prop_obj) = val {
        if let Some(arr) = prop_obj.get("k").and_then(|k| k.as_array()) {
            let x = arr
                .first()
                .and_then(|v| v.as_f64())
                .unwrap_or(default.x as f64) as f32;
            let y = arr
                .get(1)
                .and_then(|v| v.as_f64())
                .unwrap_or(default.y as f64) as f32;
            return Property::constant(Vec2::new(x, y));
        }
    }
    Property::constant(default)
}

/// Parses a scalar float property from Lottie JSON, supporting static values and keyframe easing.
fn parse_scalar_property(val: Option<&Value>, fps: f32, default: f32) -> Property<f32> {
    let prop_obj = match val {
        Some(v) => v,
        None => return Property::constant(default),
    };

    let is_animated = prop_obj.get("a").and_then(|a| a.as_u64()).unwrap_or(0) == 1;
    let k_val = match prop_obj.get("k") {
        Some(k) => k,
        None => return Property::constant(default),
    };

    if !is_animated {
        if let Some(num) = k_val.as_f64() {
            return Property::constant(num as f32);
        } else if let Some(arr) = k_val.as_array() {
            if let Some(num) = arr.first().and_then(|v| v.as_f64()) {
                return Property::constant(num as f32);
            }
        }
        return Property::constant(default);
    }

    if let Some(kf_array) = k_val.as_array() {
        let mut keyframes = Vec::new();
        for kf_obj in kf_array {
            let time_frames = kf_obj.get("t").and_then(|t| t.as_f64()).unwrap_or(0.0);
            let time_secs = (time_frames / (fps as f64)).max(0.0);

            let s_val = kf_obj
                .get("s")
                .and_then(|s| {
                    if let Some(arr) = s.as_array() {
                        arr.first().and_then(|v| v.as_f64())
                    } else {
                        s.as_f64()
                    }
                })
                .unwrap_or(default as f64) as f32;

            keyframes.push(Keyframe::new(time_secs, s_val, Easing::Linear));
        }

        if !keyframes.is_empty() {
            return Property::keyframes(keyframes);
        }
    }

    Property::constant(default)
}

/// Extracts text string from Lottie text document descriptor.
fn extract_lottie_text(layer_val: &Value) -> String {
    if let Some(t_obj) = layer_val.get("t") {
        if let Some(d_obj) = t_obj.get("d") {
            if let Some(k_arr) = d_obj.get("k").and_then(|k| k.as_array()) {
                if let Some(first) = k_arr.first() {
                    if let Some(txt) = first
                        .get("s")
                        .and_then(|s| s.get("t"))
                        .and_then(|t| t.as_str())
                    {
                        return txt.to_string();
                    }
                }
            }
        }
    }
    "Lottie Text".to_string()
}

/// Converts Lottie shape elements (rect, ellipse, fill, stroke) into native `ShapeContent`.
fn parse_shape_content(shapes_val: Option<&Value>) -> LayerContent {
    let empty_arr = Vec::new();
    let shapes = shapes_val.and_then(|s| s.as_array()).unwrap_or(&empty_arr);

    let mut geometry = ShapeGeometry::Rectangle {
        size: Property::constant(Vec2::new(100.0, 100.0)),
    };
    let mut fill = Some(Fill::Solid(Property::constant(Color::WHITE)));
    let mut stroke = None;

    for s in shapes {
        let ty = s.get("ty").and_then(|t| t.as_str()).unwrap_or("");
        match ty {
            "rc" => {
                // Rectangle shape
                let size_prop = parse_vec2_property(s.get("s"), 60.0, Vec2::new(100.0, 100.0));
                geometry = ShapeGeometry::Rectangle { size: size_prop };
            }
            "el" => {
                // Ellipse shape
                let radius_prop = parse_vec2_property(s.get("s"), 60.0, Vec2::new(50.0, 50.0));
                geometry = ShapeGeometry::Ellipse {
                    radius: radius_prop,
                };
            }
            "fl" => {
                // Fill
                if let Some(c_arr) = s
                    .get("c")
                    .and_then(|c| c.get("k"))
                    .and_then(|k| k.as_array())
                {
                    let r = c_arr.first().and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                    let g = c_arr.get(1).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                    let b = c_arr.get(2).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                    let a = c_arr.get(3).and_then(|v| v.as_f64()).unwrap_or(1.0) as f32;
                    fill = Some(Fill::Solid(Property::constant(Color::rgba(r, g, b, a))));
                }
            }
            "st" => {
                // Stroke
                let width = s
                    .get("w")
                    .and_then(|w| w.get("k"))
                    .and_then(|k| k.as_f64())
                    .unwrap_or(2.0) as f32;
                stroke = Some(Stroke {
                    width: Property::constant(width),
                    color: Property::constant(Color::WHITE),
                });
            }
            "gr" => {
                // Nested shape group: recurse
                if let Some(sub_shapes) = s.get("it") {
                    return parse_shape_content(Some(sub_shapes));
                }
            }
            _ => {}
        }
    }

    LayerContent::Shape(ShapeContent {
        geometry,
        fill,
        stroke,
    })
}

/// Parses hex color strings like "#RRGGBB".
fn parse_hex_color(hex: &str) -> Color {
    let clean = hex.trim_start_matches('#');
    if clean.len() == 6 {
        let r = u8::from_str_radix(&clean[0..2], 16).unwrap_or(255);
        let g = u8::from_str_radix(&clean[2..4], 16).unwrap_or(255);
        let b = u8::from_str_radix(&clean[4..6], 16).unwrap_or(255);
        Color::rgba(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
    } else {
        Color::WHITE
    }
}
