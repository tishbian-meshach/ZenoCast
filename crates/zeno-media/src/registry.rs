//! Application-wide registry owning all active capture sources independently of scenes.

use crate::audio_source::AudioCaptureSource;
use crate::browser_source::BrowserSource;
use crate::camera_source::CameraCaptureSource;
use crate::color_source::ColorSource;
use crate::display_source::DisplayCaptureSource;
use crate::image_source::ImageSource;
use crate::source::{Source, SourceKind, SourceProperties, VideoFrame};
use crate::text_source::TextSource;
use crate::video_source::VideoSource;
use crate::window_source::WindowCaptureSource;
use std::collections::HashMap;
use std::sync::Arc;
use wgpu::{Device, Queue, TextureView};
use zeno_core::SourceId;

/// An internal entry tracking a concrete capture source and its active reference count across scenes.
struct SourceEntry {
    source: Box<dyn Source>,
    ref_count: usize,
}

/// Creates a concrete Source trait object from typed SourceProperties.
pub fn create_source_from_properties(name: &str, props: &SourceProperties) -> Box<dyn Source> {
    match props {
        SourceProperties::Display {
            monitor_index,
            cursor_visible,
        } => {
            let mut disp = DisplayCaptureSource::from_index(*monitor_index).unwrap_or_else(|_| {
                DisplayCaptureSource::primary()
                    .unwrap_or_else(|_| DisplayCaptureSource::default_or_fallback())
            });
            disp.set_name(name.to_string());
            disp.set_capture_cursor(*cursor_visible);
            Box::new(disp)
        }
        SourceProperties::Window {
            window_title,
            cursor_visible,
        } => {
            let mut win = if window_title.is_empty() {
                WindowCaptureSource::default_or_fallback()
            } else {
                WindowCaptureSource::from_title(window_title)
            };
            win.set_name(name.to_string());
            win.set_capture_cursor(*cursor_visible);
            Box::new(win)
        }
        SourceProperties::Color {
            color,
            width,
            height,
        } => Box::new(ColorSource::new(name, *color, *width, *height)),
        SourceProperties::Text {
            text,
            font_family: _,
            font_size,
            color,
        } => Box::new(TextSource::new(name, text, *font_size, *color, 1920, 1080)),
        SourceProperties::Image {
            path,
            scale_mode: _,
        } => Box::new(ImageSource::new(name, path)),
        SourceProperties::Video { path, looping } => {
            let mut vid = VideoSource::new(name, path);
            vid.set_looping(*looping);
            Box::new(vid)
        }
        SourceProperties::Camera {
            device_name,
            format_index: _,
            resolution: _,
        } => {
            let cam = CameraCaptureSource::from_name_or_id(device_name);
            Box::new(cam)
        }
        SourceProperties::AudioInput {
            device_name,
            volume,
            muted,
        } => {
            let mut aud = AudioCaptureSource::new_input(name, device_name);
            aud.set_volume(*volume);
            aud.set_muted(*muted);
            Box::new(aud)
        }
        SourceProperties::AudioOutput {
            device_name,
            volume,
            muted,
        } => {
            let mut aud = AudioCaptureSource::new_output(name, device_name);
            aud.set_volume(*volume);
            aud.set_muted(*muted);
            Box::new(aud)
        }
        SourceProperties::Browser {
            url,
            width,
            height,
            custom_css: _,
        } => Box::new(BrowserSource::new(url, *width, *height)),
        _ => Box::new(ColorSource::new(name, [0.1, 0.2, 0.3, 1.0], 1920, 1080)),
    }
}

/// Central source registry owning live capture sources with reference counting.
pub struct SourceRegistry {
    sources: HashMap<SourceId, SourceEntry>,
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

    /// Registers a new capture source with an initial reference count of 1.
    /// If the source is already registered, increments its reference count.
    pub fn register(&mut self, source: Box<dyn Source>) -> SourceId {
        let id = source.id();
        if let Some(entry) = self.sources.get_mut(&id) {
            entry.ref_count += 1;
        } else {
            self.sources.insert(
                id,
                SourceEntry {
                    source,
                    ref_count: 1,
                },
            );
        }
        id
    }

    /// Registers a source with an explicit initial reference count.
    pub fn register_with_refs(&mut self, source: Box<dyn Source>, refs: usize) -> SourceId {
        let id = source.id();
        let ref_count = refs.max(1);
        if let Some(entry) = self.sources.get_mut(&id) {
            entry.ref_count += ref_count;
        } else {
            self.sources.insert(id, SourceEntry { source, ref_count });
        }
        id
    }

    /// Increments the reference count of an existing registered source.
    /// Returns the new reference count, or 0 if the source was not found.
    pub fn add_ref(&mut self, id: SourceId) -> usize {
        if let Some(entry) = self.sources.get_mut(&id) {
            entry.ref_count += 1;
            entry.ref_count
        } else {
            0
        }
    }

    /// Decrements the reference count of a registered source.
    ///
    /// If the reference count drops to 0, the source is automatically stopped
    /// (releasing all native device and GPU handles) and removed from the registry.
    /// Returns the remaining reference count (0 if torn down).
    pub fn release_ref(&mut self, id: SourceId) -> usize {
        if let Some(entry) = self.sources.get_mut(&id) {
            if entry.ref_count > 1 {
                entry.ref_count -= 1;
                entry.ref_count
            } else {
                if let Some(mut removed) = self.sources.remove(&id) {
                    removed.source.stop();
                }
                0
            }
        } else {
            0
        }
    }

    /// Returns the current reference count of a registered source (0 if absent).
    pub fn ref_count(&self, id: SourceId) -> usize {
        self.sources.get(&id).map(|e| e.ref_count).unwrap_or(0)
    }

    /// Releases a reference to the source. If the reference count drops to 0, stops and returns the source.
    pub fn unregister(&mut self, id: SourceId) -> Option<Box<dyn Source>> {
        if let Some(entry) = self.sources.get_mut(&id) {
            if entry.ref_count > 1 {
                entry.ref_count -= 1;
                None
            } else {
                let mut removed = self.sources.remove(&id)?;
                removed.source.stop();
                Some(removed.source)
            }
        } else {
            None
        }
    }

    /// Forcefully stops and removes a source from the registry regardless of its reference count.
    pub fn force_unregister(&mut self, id: SourceId) -> Option<Box<dyn Source>> {
        if let Some(mut entry) = self.sources.remove(&id) {
            entry.source.stop();
            Some(entry.source)
        } else {
            None
        }
    }

    /// Clones an existing source into a new independent source with a fresh UUID and cloned settings.
    ///
    /// The duplicated source is initialized and started if device and queue are provided,
    /// and registered with an initial reference count of 1.
    pub fn duplicate_source(
        &mut self,
        id: SourceId,
        new_name: Option<String>,
        device: Option<&Device>,
        queue: Option<&Queue>,
    ) -> Option<SourceId> {
        let (name, props) = {
            let src = self.sources.get(&id)?.source.as_ref();
            let name = new_name.unwrap_or_else(|| format!("{} (Copy)", src.name()));
            let props = src.properties();
            (name, props)
        };

        let new_id = SourceId::new();
        let mut new_source = create_source_from_properties(&name, &props);
        new_source.set_name(name);

        if let (Some(dev), Some(q)) = (device, queue) {
            let _ = new_source.start(dev, q);
        }

        self.sources.insert(
            new_id,
            SourceEntry {
                source: new_source,
                ref_count: 1,
            },
        );

        Some(new_id)
    }

    /// Retrieves an immutable reference to a live source.
    pub fn get(&self, id: SourceId) -> Option<&dyn Source> {
        self.sources.get(&id).map(|e| e.source.as_ref())
    }

    /// Retrieves a mutable reference to a live source.
    pub fn get_mut(&mut self, id: SourceId) -> Option<&mut (dyn Source + 'static)> {
        self.sources.get_mut(&id).map(|e| e.source.as_mut())
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
            .map(|(id, e)| {
                (
                    *id,
                    e.source.name().to_string(),
                    e.source.kind(),
                    e.source.is_active(),
                )
            })
            .collect()
    }

    /// Ticks all active sources to produce current frames.
    pub fn update_all(&mut self, device: &Device, queue: &Queue, current_time: f64) {
        for entry in self.sources.values_mut() {
            if !entry.source.is_active() {
                let _ = entry.source.start(device, queue);
            }
            if entry.source.is_active() {
                entry.source.update(device, queue, current_time);
            }
        }
    }

    /// Collects available GPU texture views from all sources for the compositor render pass.
    pub fn get_all_frame_views(&mut self) -> HashMap<SourceId, Arc<TextureView>> {
        let mut views = HashMap::new();
        for (id, entry) in self.sources.iter_mut() {
            if let Some(frame) = entry.source.get_frame() {
                views.insert(*id, frame.texture_view.clone());
            }
        }
        views
    }

    /// Retrieves the most recent frame of a specific source.
    pub fn get_frame(&mut self, id: SourceId) -> Option<&VideoFrame> {
        self.sources.get_mut(&id).and_then(|e| e.source.get_frame())
    }

    /// Stops all registered sources and clears the registry.
    pub fn clear(&mut self) {
        for (_, mut entry) in self.sources.drain() {
            entry.source.stop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color_source::ColorSource;

    #[test]
    fn test_registry_reference_counting_lifecycle() {
        let mut registry = SourceRegistry::new();
        let color_src = Box::new(ColorSource::new(
            "Test Background",
            [1.0, 0.0, 0.0, 1.0],
            1920,
            1080,
        ));
        let src_id = registry.register(color_src);

        // Initial register should give ref_count 1
        assert_eq!(registry.ref_count(src_id), 1);
        assert!(registry.contains(src_id));

        // Add second reference (e.g. Paste Reference into Scene 2)
        assert_eq!(registry.add_ref(src_id), 2);
        assert_eq!(registry.ref_count(src_id), 2);

        // Release first reference (removing item from Scene 1)
        assert_eq!(registry.release_ref(src_id), 1);
        assert!(
            registry.contains(src_id),
            "Source must remain active while ref_count > 0"
        );

        // Release second reference (removing item from Scene 2)
        assert_eq!(registry.release_ref(src_id), 0);
        assert!(
            !registry.contains(src_id),
            "Source must be torn down when ref_count hits 0"
        );
    }

    #[test]
    fn test_registry_duplicate_source_independence() {
        let mut registry = SourceRegistry::new();
        let orig_src = Box::new(ColorSource::new(
            "Camera Cam",
            [0.0, 1.0, 0.0, 1.0],
            1280,
            720,
        ));
        let orig_id = registry.register(orig_src);

        let dup_id = registry
            .duplicate_source(orig_id, Some("Camera Cam (Copy)".into()), None, None)
            .expect("Duplicate should succeed");

        assert_ne!(
            orig_id, dup_id,
            "Duplicated source must have a distinct unique ID"
        );
        assert_eq!(registry.ref_count(orig_id), 1);
        assert_eq!(registry.ref_count(dup_id), 1);

        // Verify properties were cloned
        let orig = registry.get(orig_id).unwrap();
        let dup = registry.get(dup_id).unwrap();
        assert_eq!(orig.name(), "Camera Cam");
        assert_eq!(dup.name(), "Camera Cam (Copy)");

        // Releasing original must not affect duplicate
        assert_eq!(registry.release_ref(orig_id), 0);
        assert!(!registry.contains(orig_id));
        assert!(registry.contains(dup_id));
        assert_eq!(registry.ref_count(dup_id), 1);
    }

    #[test]
    fn test_registry_force_unregister() {
        let mut registry = SourceRegistry::new();
        let src = Box::new(ColorSource::new(
            "Sticky Source",
            [0.5, 0.5, 0.5, 1.0],
            640,
            480,
        ));
        let src_id = registry.register(src);
        registry.add_ref(src_id);
        registry.add_ref(src_id);
        assert_eq!(registry.ref_count(src_id), 3);

        let removed = registry.force_unregister(src_id);
        assert!(removed.is_some());
        assert!(!registry.contains(src_id));
        assert_eq!(registry.ref_count(src_id), 0);
    }
}
