//! Application-wide registry owning all active capture sources independently of scenes.

use crate::source::{Source, SourceKind, VideoFrame};
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::{Device, Queue, TextureView};
use zeno_core::SourceId;

/// Central source registry owning live capture sources.
pub struct SourceRegistry {
    sources: HashMap<SourceId, Box<dyn Source>>,
}

impl Default for SourceRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceRegistry {
    /// Creates a new empty source registry.
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    /// Registers a new capture source and returns its assigned ID.
    pub fn register(&mut self, source: Box<dyn Source>) -> SourceId {
        let id = source.id();
        self.sources.insert(id, source);
        id
    }

    /// Unregisters and stops a source, releasing all native device and GPU handles.
    pub fn unregister(&mut self, id: SourceId) -> Option<Box<dyn Source>> {
        if let Some(mut src) = self.sources.remove(&id) {
            src.stop();
            Some(src)
        } else {
            None
        }
    }

    /// Retrieves an immutable reference to a live source.
    pub fn get(&self, id: SourceId) -> Option<&dyn Source> {
        self.sources.get(&id).map(|b| b.as_ref())
    }

    /// Retrieves a mutable reference to a live source.
    pub fn get_mut(&mut self, id: SourceId) -> Option<&mut (dyn Source + 'static)> {
        self.sources.get_mut(&id).map(|b| b.as_mut())
    }

    /// Returns `true` if the specified source exists in the registry.
    pub fn contains(&self, id: SourceId) -> bool {
        self.sources.contains_key(&id)
    }

    /// Number of registered sources.
    pub fn count(&self) -> usize {
        self.sources.len()
    }

    /// Returns `true` if no sources are currently registered.
    pub fn is_empty(&self) -> bool {
        self.sources.is_empty()
    }

    /// Lists summary metadata for all registered sources (id, name, kind, active).
    pub fn list(&self) -> Vec<(SourceId, String, SourceKind, bool)> {
        self.sources
            .iter()
            .map(|(id, src)| (*id, src.name().to_string(), src.kind(), src.is_active()))
            .collect()
    }

    /// Ticks all active sources to produce current frames.
    pub fn update_all(&mut self, device: &Device, queue: &Queue, current_time: f64) {
        for source in self.sources.values_mut() {
            if !source.is_active() {
                let _ = source.start(device, queue);
            }
            if source.is_active() {
                source.update(device, queue, current_time);
            }
        }
    }

    /// Collects available GPU texture views from all sources for the compositor render pass.
    pub fn get_all_frame_views(&mut self) -> HashMap<SourceId, Arc<TextureView>> {
        let mut views = HashMap::new();
        for (id, source) in self.sources.iter_mut() {
            if let Some(frame) = source.get_frame() {
                views.insert(*id, frame.texture_view.clone());
            }
        }
        views
    }

    /// Retrieves the most recent frame of a specific source.
    pub fn get_frame(&mut self, id: SourceId) -> Option<&VideoFrame> {
        self.sources.get_mut(&id).and_then(|s| s.get_frame())
    }

    /// Stops all registered sources and clears the registry.
    pub fn clear(&mut self) {
        for (_, mut src) in self.sources.drain() {
            src.stop();
        }
    }
}
