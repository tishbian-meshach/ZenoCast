//! Error types for the scene compositor and transition engine.

use thiserror::Error;
use zeno_core::{SceneId, SourceId, TransitionId};

/// Compositor errors covering invalid scenes, transitions, and GPU binding failures.
#[derive(Debug, Error)]
pub enum CompositorError {
    /// Referenced scene was not found in the scene collection.
    #[error("Scene not found: {0}")]
    SceneNotFound(SceneId),

    /// Referenced media or capture source was not found.
    #[error("Source not found: {0}")]
    SourceNotFound(SourceId),

    /// Transition not found in transition catalog.
    #[error("Transition not found: {0}")]
    TransitionNotFound(TransitionId),

    /// Recursive nested scene cycle detected.
    #[error("Nested scene cycle detected at scene {0}")]
    SceneRecursionCycle(SceneId),

    /// GPU pipeline or buffer creation error.
    #[error("GPU pipeline error: {0}")]
    GpuPipeline(String),

    /// Render target or texture operation failure.
    #[error("Texture error: {0}")]
    TextureError(String),
}
