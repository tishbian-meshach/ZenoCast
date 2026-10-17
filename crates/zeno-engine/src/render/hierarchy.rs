//! Scene graph parent-child hierarchy resolution and world matrix calculation.

use crate::model::{Composition, Layer, Transform3D};
use std::collections::{HashMap, HashSet};
use zeno_core::{LayerId, Mat4};

/// A flattened, evaluated layer ready for rasterization or GPU rendering.
#[derive(Debug, Clone)]
pub struct ResolvedLayer {
    /// The underlying layer definition.
    pub layer: Layer,
    /// Computed 4x4 world transformation matrix.
    pub world_matrix: Mat4,
    /// Net resolved opacity (0.0 to 1.0) incorporating parent opacities.
    pub net_opacity: f32,
}

/// Resolves hierarchical parenting relationships into absolute world transform matrices.
pub fn resolve_scene_hierarchy(composition: &Composition, time_seconds: f64) -> Vec<ResolvedLayer> {
    // 1. Filter active visible layers
    let active_layers: Vec<&Layer> = composition
        .layers
        .iter()
        .filter(|l| l.is_active_at(time_seconds))
        .collect();

    // Map layer ID to layer
    let mut layer_map: HashMap<LayerId, &Layer> = HashMap::new();
    for layer in &active_layers {
        layer_map.insert(layer.id, layer);
    }

    // 2. Compute local transform matrices for each layer
    let mut local_matrices: HashMap<LayerId, Mat4> = HashMap::new();
    let mut local_opacities: HashMap<LayerId, f32> = HashMap::new();

    for layer in &active_layers {
        let pos = *layer.transform.position.fallback_value();
        let anchor = *layer.transform.anchor_point.fallback_value();
        let scale = *layer.transform.scale_percent.fallback_value();
        let rot = *layer.transform.rotation_degrees.fallback_value();
        let opacity = (*layer.transform.opacity_percent.fallback_value() / 100.0).clamp(0.0, 1.0);

        let local_mat = Transform3D::compute_matrix(pos, anchor, scale, rot);
        local_matrices.insert(layer.id, local_mat);
        local_opacities.insert(layer.id, opacity);
    }

    // 3. Resolve world matrices recursively with cycle detection
    let mut world_matrices: HashMap<LayerId, Mat4> = HashMap::new();
    let mut net_opacities: HashMap<LayerId, f32> = HashMap::new();

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
    let mut resolved_list = Vec::with_capacity(active_layers.len());

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

        resolved_list.push(ResolvedLayer {
            layer: layer.clone(),
            world_matrix,
            net_opacity,
        });
    }

    resolved_list
}
