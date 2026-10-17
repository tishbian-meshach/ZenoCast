//! Scene graph parent-child hierarchy resolution and world matrix calculation.

use crate::eval::evaluate_at;
use crate::model::Composition;

pub use crate::eval::ResolvedLayer;

/// Resolves hierarchical parenting relationships into absolute world transform matrices and evaluated layers.
#[inline]
pub fn resolve_scene_hierarchy(composition: &Composition, time_seconds: f64) -> Vec<ResolvedLayer> {
    evaluate_at(composition, time_seconds).layers
}
