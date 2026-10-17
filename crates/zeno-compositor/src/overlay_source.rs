//! Overlay live source rendering .arx compositions through zeno-engine into the compositor pipeline.

use std::sync::Arc;
use zeno_core::SourceId;
use zeno_data::{DataStore, DataValue};
use zeno_engine::eval::{evaluate, ResolvedFrame};
use zeno_engine::Composition;

/// An overlay source hosted inside the compositor that shares the same evaluation engine
/// and live reactive DataStore.
#[derive(Debug, Clone)]
pub struct ArxOverlaySource {
    pub id: SourceId,
    pub name: String,
    pub composition: Composition,
    pub store: Arc<DataStore>,
    pub current_time: f64,
}

impl ArxOverlaySource {
    /// Creates a new overlay source backed by the shared DataStore.
    pub fn new(name: impl Into<String>, composition: Composition, store: Arc<DataStore>) -> Self {
        Self {
            id: SourceId::new(),
            name: name.into(),
            composition,
            store,
            current_time: 0.0,
        }
    }

    /// Advances playback time.
    pub fn tick(&mut self, delta_seconds: f64) {
        self.current_time += delta_seconds;
    }

    /// Sets playback time.
    pub fn set_time(&mut self, time_seconds: f64) {
        self.current_time = time_seconds;
    }

    /// Evaluates the overlay frame using the shared engine renderer and live DataStore snapshot.
    pub fn evaluate_frame(&self) -> ResolvedFrame {
        let mut comp = self.composition.clone();
        zeno_data::apply_bindings_to_composition(&mut comp, &self.store);
        let snapshot = self.store.to_snapshot();
        evaluate(&comp, Some(&snapshot), self.current_time)
    }

    /// Updates a variable path in the shared store.
    pub fn set_data(&self, path: impl Into<String>, value: impl Into<DataValue>) {
        self.store.set(path, value);
    }
}
