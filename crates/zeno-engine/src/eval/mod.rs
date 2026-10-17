//! Pure evaluation engine transforming Composition, DataSnapshot, and PlaybackClock into ResolvedFrame.

pub mod clock;
pub mod frame;
pub mod lifecycle;
pub mod snapshot;

pub use clock::{PlaybackClock, PlaybackState};
pub use frame::{
    ResolvedContent, ResolvedFill, ResolvedFrame, ResolvedLayer, ResolvedShape,
    ResolvedShapeGeometry, ResolvedStroke, ResolvedText,
};
pub use lifecycle::{LifecycleController, LifecycleManager};
pub use snapshot::DataSnapshot;

use crate::model::{
    Composition, Fill, Layer, LayerContent, ShapeContent, ShapeGeometry, Stroke, TextContent,
    Transform3D,
};
use std::collections::{HashMap, HashSet};
use zeno_core::{evaluate_keyframes, Color, LayerId, Mat4, Property, Vec2, Vec3};

/// Evaluates a property of type `f32` following Section 6 resolution order.
fn resolve_f32(prop: &Property<f32>, data: Option<&DataSnapshot>, t: f64) -> f32 {
    match prop {
        Property::Constant(v) => *v,
        Property::Keyframes(keys) => evaluate_keyframes(keys, t),
        Property::Binding {
            data_path,
            fallback,
        } => {
            if let Some(snapshot) = data {
                if let Some(val) = snapshot.get(data_path) {
                    if let Some(n) = val.as_f64() {
                        return n as f32;
                    }
                }
            }
            *fallback
        }
        Property::Expression { fallback, .. } => *fallback,
    }
}

/// Evaluates a property of type `String` following Section 6 resolution order.
fn resolve_string(prop: &Property<String>, data: Option<&DataSnapshot>, t: f64) -> String {
    match prop {
        Property::Constant(v) => v.clone(),
        Property::Keyframes(keys) => evaluate_keyframes(keys, t),
        Property::Binding {
            data_path,
            fallback,
        } => {
            if let Some(snapshot) = data {
                if let Some(val) = snapshot.get(data_path) {
                    if let Some(s) = val.as_str() {
                        return s.to_string();
                    }
                    return val.to_string();
                }
            }
            fallback.clone()
        }
        Property::Expression { fallback, .. } => fallback.clone(),
    }
}

/// Evaluates a property of type `Vec2` following Section 6 resolution order.
fn resolve_vec2(prop: &Property<Vec2>, _data: Option<&DataSnapshot>, t: f64) -> Vec2 {
    match prop {
        Property::Constant(v) => *v,
        Property::Keyframes(keys) => evaluate_keyframes(keys, t),
        Property::Binding { fallback, .. } => *fallback,
        Property::Expression { fallback, .. } => *fallback,
    }
}

/// Evaluates a property of type `Vec3` following Section 6 resolution order.
fn resolve_vec3(prop: &Property<Vec3>, _data: Option<&DataSnapshot>, t: f64) -> Vec3 {
    match prop {
        Property::Constant(v) => *v,
        Property::Keyframes(keys) => evaluate_keyframes(keys, t),
        Property::Binding { fallback, .. } => *fallback,
        Property::Expression { fallback, .. } => *fallback,
    }
}

/// Evaluates a property of type `Color` following Section 6 resolution order.
fn resolve_color(prop: &Property<Color>, data: Option<&DataSnapshot>, t: f64) -> Color {
    match prop {
        Property::Constant(v) => *v,
        Property::Keyframes(keys) => evaluate_keyframes(keys, t),
        Property::Binding {
            data_path,
            fallback,
        } => {
            if let Some(snapshot) = data {
                if let Some(val) = snapshot.get(data_path) {
                    if let Some(s) = val.as_str() {
                        if let Ok(c) = Color::from_hex(s) {
                            return c;
                        }
                    }
                }
            }
            *fallback
        }
        Property::Expression { fallback, .. } => *fallback,
    }
}

/// Pure evaluation function transforming a Composition at a specific timestamp into a `ResolvedFrame`.
///
/// Complies with Section 6 resolution order:
/// 1. `Binding`
/// 2. `Expression`
/// 3. `Keyframes`
/// 4. `Constant`
pub fn evaluate(
    composition: &Composition,
    data_snapshot: Option<&DataSnapshot>,
    time_seconds: f64,
) -> ResolvedFrame {
    // 1. Filter active visible layers at this timestamp
    let active_layers: Vec<&Layer> = composition
        .layers
        .iter()
        .filter(|l| l.is_active_at(time_seconds))
        .collect();

    let mut layer_map: HashMap<LayerId, &Layer> = HashMap::with_capacity(active_layers.len());
    for layer in &active_layers {
        layer_map.insert(layer.id, layer);
    }

    // 2. Evaluate local transforms and content for each active layer
    let mut local_matrices: HashMap<LayerId, Mat4> = HashMap::with_capacity(active_layers.len());
    let mut local_opacities: HashMap<LayerId, f32> = HashMap::with_capacity(active_layers.len());
    let mut resolved_contents: HashMap<LayerId, ResolvedContent> =
        HashMap::with_capacity(active_layers.len());
    let mut evaluated_layers: HashMap<LayerId, Layer> = HashMap::with_capacity(active_layers.len());

    for layer in &active_layers {
        let pos = resolve_vec3(&layer.transform.position, data_snapshot, time_seconds);
        let anchor = resolve_vec3(&layer.transform.anchor_point, data_snapshot, time_seconds);
        let scale = resolve_vec3(&layer.transform.scale_percent, data_snapshot, time_seconds);
        let rot = resolve_vec3(
            &layer.transform.rotation_degrees,
            data_snapshot,
            time_seconds,
        );
        let opacity = (resolve_f32(
            &layer.transform.opacity_percent,
            data_snapshot,
            time_seconds,
        ) / 100.0)
            .clamp(0.0, 1.0);

        let local_mat = Transform3D::compute_matrix(pos, anchor, scale, rot);
        local_matrices.insert(layer.id, local_mat);
        local_opacities.insert(layer.id, opacity);

        // Resolve layer content
        let (resolved_content, evaluated_content) = match &layer.content {
            LayerContent::Null => (ResolvedContent::Null, LayerContent::Null),
            LayerContent::Shape(shape) => {
                let (resolved_geom, eval_geom) = match &shape.geometry {
                    ShapeGeometry::Rectangle { size } => {
                        let s = resolve_vec2(size, data_snapshot, time_seconds);
                        (
                            ResolvedShapeGeometry::Rectangle { size: s },
                            ShapeGeometry::Rectangle {
                                size: Property::constant(s),
                            },
                        )
                    }
                    ShapeGeometry::RoundedRectangle {
                        size,
                        corner_radius,
                    } => {
                        let s = resolve_vec2(size, data_snapshot, time_seconds);
                        let r = resolve_f32(corner_radius, data_snapshot, time_seconds);
                        (
                            ResolvedShapeGeometry::RoundedRectangle {
                                size: s,
                                corner_radius: r,
                            },
                            ShapeGeometry::RoundedRectangle {
                                size: Property::constant(s),
                                corner_radius: Property::constant(r),
                            },
                        )
                    }
                    ShapeGeometry::Ellipse { radius } => {
                        let r = resolve_vec2(radius, data_snapshot, time_seconds);
                        (
                            ResolvedShapeGeometry::Ellipse { radius: r },
                            ShapeGeometry::Ellipse {
                                radius: Property::constant(r),
                            },
                        )
                    }
                    ShapeGeometry::Path { svg_data } => (
                        ResolvedShapeGeometry::Path {
                            svg_data: svg_data.clone(),
                        },
                        ShapeGeometry::Path {
                            svg_data: svg_data.clone(),
                        },
                    ),
                };

                let (resolved_fill, eval_fill) = match &shape.fill {
                    Some(Fill::Solid(c)) => {
                        let color = resolve_color(c, data_snapshot, time_seconds);
                        (
                            Some(ResolvedFill::Solid(color)),
                            Some(Fill::Solid(Property::constant(color))),
                        )
                    }
                    Some(Fill::LinearGradient { start, end, stops }) => (
                        Some(ResolvedFill::LinearGradient {
                            start: *start,
                            end: *end,
                            stops: stops.clone(),
                        }),
                        Some(Fill::LinearGradient {
                            start: *start,
                            end: *end,
                            stops: stops.clone(),
                        }),
                    ),
                    Some(Fill::RadialGradient {
                        center,
                        radius,
                        stops,
                    }) => (
                        Some(ResolvedFill::RadialGradient {
                            center: *center,
                            radius: *radius,
                            stops: stops.clone(),
                        }),
                        Some(Fill::RadialGradient {
                            center: *center,
                            radius: *radius,
                            stops: stops.clone(),
                        }),
                    ),
                    None => (None, None),
                };

                let (resolved_stroke, eval_stroke) = match &shape.stroke {
                    Some(Stroke { color, width }) => {
                        let c = resolve_color(color, data_snapshot, time_seconds);
                        let w = resolve_f32(width, data_snapshot, time_seconds);
                        (
                            Some(ResolvedStroke { color: c, width: w }),
                            Some(Stroke {
                                color: Property::constant(c),
                                width: Property::constant(w),
                            }),
                        )
                    }
                    None => (None, None),
                };

                (
                    ResolvedContent::Shape(ResolvedShape {
                        geometry: resolved_geom,
                        fill: resolved_fill,
                        stroke: resolved_stroke,
                    }),
                    LayerContent::Shape(ShapeContent {
                        geometry: eval_geom,
                        fill: eval_fill,
                        stroke: eval_stroke,
                    }),
                )
            }
            LayerContent::Text(text) => {
                let resolved_str = resolve_string(&text.text, data_snapshot, time_seconds);
                let font_size = resolve_f32(&text.font_size, data_snapshot, time_seconds);
                let color = resolve_color(&text.color, data_snapshot, time_seconds);
                let line_height_multiplier =
                    resolve_f32(&text.line_height_multiplier, data_snapshot, time_seconds);
                let letter_spacing = resolve_f32(&text.letter_spacing, data_snapshot, time_seconds);

                (
                    ResolvedContent::Text(ResolvedText {
                        text: resolved_str.clone(),
                        font_family: text.font_family.clone(),
                        font_size,
                        font_weight: text.font_weight,
                        color,
                        line_height_multiplier,
                        letter_spacing,
                        alignment: text.alignment,
                    }),
                    LayerContent::Text(TextContent {
                        text: Property::constant(resolved_str),
                        font_family: text.font_family.clone(),
                        font_size: Property::constant(font_size),
                        font_weight: text.font_weight,
                        color: Property::constant(color),
                        line_height_multiplier: Property::constant(line_height_multiplier),
                        letter_spacing: Property::constant(letter_spacing),
                        alignment: text.alignment,
                        word_wrap: text.word_wrap,
                        max_width: text.max_width,
                    }),
                )
            }
            LayerContent::Image { asset } => (
                ResolvedContent::Image { asset: *asset },
                LayerContent::Image { asset: *asset },
            ),
            LayerContent::Video {
                asset,
                loop_playback,
                speed,
            } => {
                let s = resolve_f32(speed, data_snapshot, time_seconds);
                (
                    ResolvedContent::Video {
                        asset: *asset,
                        loop_playback: *loop_playback,
                        speed: s,
                    },
                    LayerContent::Video {
                        asset: *asset,
                        loop_playback: *loop_playback,
                        speed: Property::constant(s),
                    },
                )
            }
            LayerContent::Group { children } => (
                ResolvedContent::Group {
                    children: children.clone(),
                },
                LayerContent::Group {
                    children: children.clone(),
                },
            ),
            LayerContent::Particles(_) => (ResolvedContent::Particles, layer.content.clone()),
            LayerContent::Model3D { asset } => (
                ResolvedContent::Model3D { asset: *asset },
                LayerContent::Model3D { asset: *asset },
            ),
            LayerContent::Camera(_) => (ResolvedContent::Camera, layer.content.clone()),
            LayerContent::Repeater {
                template_group,
                data_path,
                max_rows,
                row_offset,
            } => {
                let offset = resolve_vec2(row_offset, data_snapshot, time_seconds);
                (
                    ResolvedContent::Repeater {
                        template_group: *template_group,
                        data_path: data_path.clone(),
                        max_rows: *max_rows,
                        row_offset: offset,
                    },
                    LayerContent::Repeater {
                        template_group: *template_group,
                        data_path: data_path.clone(),
                        max_rows: *max_rows,
                        row_offset: Property::constant(offset),
                    },
                )
            }
        };

        resolved_contents.insert(layer.id, resolved_content);

        let mut eval_layer = (*layer).clone();
        eval_layer.transform.position = Property::constant(pos);
        eval_layer.transform.anchor_point = Property::constant(anchor);
        eval_layer.transform.scale_percent = Property::constant(scale);
        eval_layer.transform.rotation_degrees = Property::constant(rot);
        eval_layer.transform.opacity_percent = Property::constant(opacity * 100.0);
        eval_layer.content = evaluated_content;
        evaluated_layers.insert(layer.id, eval_layer);
    }

    // 3. Resolve parent-child hierarchy matrices and net opacities
    let mut world_matrices: HashMap<LayerId, Mat4> = HashMap::with_capacity(active_layers.len());
    let mut net_opacities: HashMap<LayerId, f32> = HashMap::with_capacity(active_layers.len());

    fn resolve_node(
        id: LayerId,
        layer_map: &HashMap<LayerId, &Layer>,
        local_matrices: &HashMap<LayerId, Mat4>,
        local_opacities: &HashMap<LayerId, f32>,
        world_matrices: &mut HashMap<LayerId, Mat4>,
        net_opacities: &mut HashMap<LayerId, f32>,
        visited: &mut HashSet<LayerId>,
    ) -> (Mat4, f32) {
        if let (Some(&cached_mat), Some(&cached_op)) =
            (world_matrices.get(&id), net_opacities.get(&id))
        {
            return (cached_mat, cached_op);
        }

        if !visited.insert(id) {
            // Cycle detected: fallback to identity
            return (Mat4::IDENTITY, 1.0);
        }

        let local_mat = local_matrices.get(&id).copied().unwrap_or(Mat4::IDENTITY);
        let local_op = local_opacities.get(&id).copied().unwrap_or(1.0);

        let (parent_mat, parent_op) = if let Some(layer) = layer_map.get(&id) {
            if let Some(parent_id) = layer.parent {
                if layer_map.contains_key(&parent_id) {
                    resolve_node(
                        parent_id,
                        layer_map,
                        local_matrices,
                        local_opacities,
                        world_matrices,
                        net_opacities,
                        visited,
                    )
                } else {
                    (Mat4::IDENTITY, 1.0)
                }
            } else {
                (Mat4::IDENTITY, 1.0)
            }
        } else {
            (Mat4::IDENTITY, 1.0)
        };

        let world_mat = parent_mat * local_mat;
        let net_op = parent_op * local_op;

        world_matrices.insert(id, world_mat);
        net_opacities.insert(id, net_op);
        visited.remove(&id);

        (world_mat, net_op)
    }

    let mut visited = HashSet::new();
    let mut resolved_layers = Vec::with_capacity(active_layers.len());

    for layer in active_layers {
        let (world_matrix, net_opacity) = resolve_node(
            layer.id,
            &layer_map,
            &local_matrices,
            &local_opacities,
            &mut world_matrices,
            &mut net_opacities,
            &mut visited,
        );

        let content = resolved_contents
            .remove(&layer.id)
            .unwrap_or(ResolvedContent::Null);
        let eval_layer = evaluated_layers
            .remove(&layer.id)
            .unwrap_or_else(|| layer.clone());

        resolved_layers.push(ResolvedLayer {
            id: layer.id,
            name: layer.name.clone(),
            parent: layer.parent,
            world_matrix,
            net_opacity,
            blend_mode: layer.blend_mode,
            is_3d: layer.is_3d,
            visible: layer.visible,
            content,
            layer: eval_layer,
        });
    }

    ResolvedFrame {
        time_seconds,
        width: composition.width,
        height: composition.height,
        layers: resolved_layers,
    }
}

/// Convenience evaluation without data binding overrides.
#[inline]
pub fn evaluate_at(composition: &Composition, time_seconds: f64) -> ResolvedFrame {
    evaluate(composition, None, time_seconds)
}

/// Convenience evaluation driven by a `PlaybackClock`.
#[inline]
pub fn evaluate_with_clock(
    composition: &Composition,
    data_snapshot: Option<&DataSnapshot>,
    clock: &PlaybackClock,
) -> ResolvedFrame {
    evaluate(composition, data_snapshot, clock.time())
}
