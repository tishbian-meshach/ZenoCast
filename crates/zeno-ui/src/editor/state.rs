//! Overlay editor state: composition model, playback clock, undo/redo, markers, and .arx I/O.

use std::path::Path;

use serde::{Deserialize, Serialize};
use zeno_core::{Color, Easing, Keyframe, LayerId, Vec2, Vec3};
use zeno_engine::{
    Composition, Fill, Layer, LayerContent, PlaybackClock, ShapeContent, ShapeGeometry,
    TextContent, Transform3D,
};
use zeno_format::{ArxPackage, FormatError, Manifest};

use crate::editor::command::{
    AddLayerCommand, AddOpacityKeyframeCommand, AddPositionKeyframeCommand, CommandHistory,
    DeleteLayerCommand, ModifyTextCommand, ReorderLayerCommand, SetTransformCommand,
};

/// Timeline marker for synchronization and section tagging.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineMarker {
    pub time_seconds: f64,
    pub name: String,
    pub comment: Option<String>,
}

/// Active state and document for Edit Overlay mode.
pub struct OverlayEditor {
    /// Motion-graphics composition graph
    pub composition: Composition,
    /// Playback and scrubbing clock
    pub clock: PlaybackClock,
    /// Reversible edit command history
    pub history: CommandHistory,
    /// Currently selected layer ID in inspector/canvas
    pub selected_layer_id: Option<LayerId>,
    /// Internal layer clipboard for Copy/Paste
    pub clipboard_layer: Option<Layer>,
    /// Timeline cue and section markers
    pub markers: Vec<TimelineMarker>,
    /// Canvas viewport zoom factor
    pub zoom: f32,
    /// Canvas viewport pan offset in pixels
    pub pan: Vec2,
    /// Whether the Bezier Curve Editor is active
    pub curve_editor_open: bool,
    /// Active curve property identifier (e.g., "position.y", "opacity")
    pub active_curve_property: Option<String>,
    /// Bezier control handle 1 (normalized 0..1)
    pub curve_c1: Vec2,
    /// Bezier control handle 2 (normalized 0..1)
    pub curve_c2: Vec2,
    /// Name of active overlay
    pub name: String,
    /// Interactive on-canvas transform gizmo state
    pub transform_gizmo_state: crate::transform_gizmo::TransformGizmoState,
}

impl Default for OverlayEditor {
    fn default() -> Self {
        Self::new(1920, 1080, 60.0, 5.0)
    }
}

impl OverlayEditor {
    /// Creates a new overlay editor with an empty 1080p60 composition.
    pub fn new(width: u32, height: u32, fps: f32, duration_seconds: f64) -> Self {
        let composition = Composition::new(width, height, fps, duration_seconds);
        let clock = PlaybackClock::new(duration_seconds, fps);

        Self {
            composition,
            clock,
            history: CommandHistory::new(),
            selected_layer_id: None,
            clipboard_layer: None,
            markers: vec![
                TimelineMarker {
                    time_seconds: 0.0,
                    name: "Intro Start".into(),
                    comment: None,
                },
                TimelineMarker {
                    time_seconds: 1.0,
                    name: "Loop Region".into(),
                    comment: None,
                },
            ],
            zoom: 1.0,
            pan: Vec2::ZERO,
            curve_editor_open: false,
            active_curve_property: Some("opacity".into()),
            curve_c1: Vec2::new(0.42, 0.0),
            curve_c2: Vec2::new(0.58, 1.0),
            name: "Untitled Overlay".into(),
            transform_gizmo_state: crate::transform_gizmo::TransformGizmoState::default(),
        }
    }

    /// Adds a new editable text layer to the composition with undo support.
    pub fn add_text_layer(&mut self, name: &str, text: &str) -> LayerId {
        let id = LayerId::new();
        let mut layer = Layer::new(
            id,
            name,
            LayerContent::Text(TextContent {
                text: zeno_core::Property::constant(text.to_string()),
                font_family: "Inter".into(),
                font_size: zeno_core::Property::constant(48.0),
                color: zeno_core::Property::constant(Color::WHITE),
                ..Default::default()
            }),
        );
        layer.transform.position = zeno_core::Property::constant(Vec3::new(
            self.composition.width as f32 / 2.0,
            self.composition.height as f32 / 2.0,
            0.0,
        ));

        let index = self.composition.layers.len();
        self.history.execute(
            Box::new(AddLayerCommand::new(layer, index)),
            &mut self.composition,
        );

        self.selected_layer_id = Some(id);
        id
    }

    /// Adds a new rectangle shape layer with undo support.
    pub fn add_shape_layer(&mut self, name: &str, size: Vec2, fill_color: Color) -> LayerId {
        let id = LayerId::new();
        let mut layer = Layer::new(
            id,
            name,
            LayerContent::Shape(ShapeContent {
                geometry: ShapeGeometry::Rectangle {
                    size: zeno_core::Property::constant(size),
                },
                fill: Some(Fill::Solid(zeno_core::Property::constant(fill_color))),
                stroke: None,
            }),
        );
        layer.transform.position = zeno_core::Property::constant(Vec3::new(
            self.composition.width as f32 / 2.0,
            self.composition.height as f32 / 2.0,
            0.0,
        ));

        let index = self.composition.layers.len();
        self.history.execute(
            Box::new(AddLayerCommand::new(layer, index)),
            &mut self.composition,
        );

        self.selected_layer_id = Some(id);
        id
    }

    /// Adds a new perspective camera layer with undo support.
    pub fn add_camera_layer(&mut self, name: &str) -> LayerId {
        let id = LayerId::new();
        let cam_settings = zeno_engine::CameraSettings::broadcast_default(
            self.composition.width,
            self.composition.height,
        );
        let mut layer = Layer::new(id, name, LayerContent::Camera(cam_settings));
        layer.is_3d = true;

        let index = self.composition.layers.len();
        self.history.execute(
            Box::new(AddLayerCommand::new(layer, index)),
            &mut self.composition,
        );

        self.selected_layer_id = Some(id);
        id
    }

    /// Adds a new GPU particle emitter layer with undo support.
    pub fn add_particle_layer(
        &mut self,
        name: &str,
        emitter: zeno_engine::ParticleEmitter,
    ) -> LayerId {
        let id = LayerId::new();
        let mut layer = Layer::new(id, name, LayerContent::Particles(emitter));
        layer.transform.position = zeno_core::Property::constant(Vec3::new(
            self.composition.width as f32 / 2.0,
            self.composition.height as f32 / 2.0,
            0.0,
        ));

        let index = self.composition.layers.len();
        self.history.execute(
            Box::new(AddLayerCommand::new(layer, index)),
            &mut self.composition,
        );

        self.selected_layer_id = Some(id);
        id
    }

    /// Deletes the currently selected layer with undo support.
    pub fn delete_selected_layer(&mut self) -> bool {
        if let Some(id) = self.selected_layer_id {
            if let Some(cmd) = DeleteLayerCommand::new(id, &self.composition) {
                self.history.execute(Box::new(cmd), &mut self.composition);
                self.selected_layer_id = self.composition.layers.first().map(|l| l.id);
                return true;
            }
        }
        false
    }

    /// Reorders layers in the scene stack with undo support.
    pub fn reorder_layer(&mut self, from_idx: usize, to_idx: usize) {
        if from_idx != to_idx
            && from_idx < self.composition.layers.len()
            && to_idx < self.composition.layers.len()
        {
            let cmd = ReorderLayerCommand::new(from_idx, to_idx);
            self.history.execute(Box::new(cmd), &mut self.composition);
        }
    }

    /// Copies the selected layer into the editor clipboard.
    pub fn copy_selected(&mut self) -> bool {
        if let Some(id) = self.selected_layer_id {
            if let Some(layer) = self.composition.layers.iter().find(|l| l.id == id) {
                self.clipboard_layer = Some(layer.clone());
                return true;
            }
        }
        false
    }

    /// Pastes the clipboard layer as a new layer with unique ID and offset.
    pub fn paste(&mut self) -> Option<LayerId> {
        let mut layer = self.clipboard_layer.clone()?;
        let new_id = LayerId::new();
        layer.id = new_id;
        layer.name = format!("{} (Copy)", layer.name);

        // Offset position slightly
        let cur_pos = *layer.transform.position.fallback_value();
        layer.transform.position =
            zeno_core::Property::constant(cur_pos + Vec3::new(20.0, 20.0, 0.0));

        let index = self.composition.layers.len();
        self.history.execute(
            Box::new(AddLayerCommand::new(layer, index)),
            &mut self.composition,
        );

        self.selected_layer_id = Some(new_id);
        Some(new_id)
    }

    /// Adds an animated position keyframe to the selected layer at playhead time.
    pub fn add_position_keyframe(&mut self, val: Vec3, easing: Easing) -> bool {
        if let Some(id) = self.selected_layer_id {
            let t = self.clock.time();
            let kf = Keyframe::new(t, val, easing);
            let cmd = AddPositionKeyframeCommand::new(id, kf);
            self.history.execute(Box::new(cmd), &mut self.composition);
            return true;
        }
        false
    }

    /// Adds an animated opacity keyframe to the selected layer at playhead time.
    pub fn add_opacity_keyframe(&mut self, val: f32, easing: Easing) -> bool {
        if let Some(id) = self.selected_layer_id {
            let t = self.clock.time();
            let kf = Keyframe::new(t, val, easing);
            let cmd = AddOpacityKeyframeCommand::new(id, kf);
            self.history.execute(Box::new(cmd), &mut self.composition);
            return true;
        }
        false
    }

    /// Modifies text on the selected layer with undo support.
    pub fn modify_selected_text(&mut self, new_text: &str) -> bool {
        if let Some(id) = self.selected_layer_id {
            if let Some(layer) = self.composition.layers.iter().find(|l| l.id == id) {
                if let LayerContent::Text(ref t) = layer.content {
                    let old_text = t.text.fallback_value().clone();
                    let cmd = ModifyTextCommand::new(id, old_text, new_text.to_string());
                    self.history.execute(Box::new(cmd), &mut self.composition);
                    return true;
                }
            }
        }
        false
    }

    /// Updates transform on the selected layer with undo support.
    pub fn set_selected_transform(&mut self, new_transform: Transform3D) -> bool {
        if let Some(id) = self.selected_layer_id {
            if let Some(layer) = self.composition.layers.iter().find(|l| l.id == id) {
                let old_transform = layer.transform.clone();
                let cmd = SetTransformCommand::new(id, old_transform, new_transform);
                self.history.execute(Box::new(cmd), &mut self.composition);
                return true;
            }
        }
        false
    }

    /// Reverts the last command.
    pub fn undo(&mut self) -> bool {
        self.history.undo(&mut self.composition)
    }

    /// Redoes the last reverted command.
    pub fn redo(&mut self) -> bool {
        self.history.redo(&mut self.composition)
    }

    /// Saves the active composition as a standard `.arx` container package.
    pub fn save_to_arx(&self, path: &Path) -> Result<(), FormatError> {
        let manifest = Manifest::new(
            &self.name,
            "Zenocast Editor",
            self.composition.width,
            self.composition.height,
            self.composition.frames_per_second,
        );

        let pkg = ArxPackage::new(manifest, self.composition.clone());
        let bytes = pkg.to_bytes()?;
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(path, bytes).map_err(FormatError::Io)?;
        Ok(())
    }

    /// Alias for save_to_arx.
    pub fn save_arx(&self, path: &Path) -> Result<(), FormatError> {
        self.save_to_arx(path)
    }

    /// Loads a composition from an existing `.arx` container package.
    pub fn load_from_arx(path: &Path) -> Result<Self, FormatError> {
        let bytes = std::fs::read(path).map_err(FormatError::Io)?;
        let pkg = ArxPackage::from_bytes(&bytes)?;

        let mut editor = Self::new(
            pkg.manifest.width,
            pkg.manifest.height,
            pkg.manifest.fps,
            pkg.scene.duration_seconds,
        );
        editor.name = pkg.manifest.name;
        editor.composition = pkg.scene;
        editor.selected_layer_id = editor.composition.layers.first().map(|l| l.id);

        Ok(editor)
    }

    /// Alias for load_from_arx.
    pub fn load_arx(path: &Path) -> Result<Self, FormatError> {
        Self::load_from_arx(path)
    }
}
