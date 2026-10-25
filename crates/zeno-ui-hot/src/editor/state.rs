//! Overlay editor state: composition model, playback clock, undo/redo, markers, and .arx I/O.

use std::collections::HashMap;
use std::hash::{DefaultHasher, Hasher};
use std::path::Path;

use serde::{Deserialize, Serialize};
use zeno_core::{AssetId, Color, Easing, Keyframe, LayerId, Vec2, Vec3};
use zeno_engine::{
    AssetKind, AssetRef, Composition, Fill, Layer, LayerContent, PlaybackClock, ShapeContent,
    ShapeGeometry, TextContent, Transform3D,
};
use zeno_format::{ArxPackage, FormatError, Manifest};

use crate::editor::command::{
    AddLayerCommand, AddOpacityKeyframeCommand, AddPositionKeyframeCommand,
    AddRotationKeyframeCommand, AddScaleKeyframeCommand, AnimProperty, CommandHistory,
    DeleteKeyframeCommand, DeleteLayerCommand, ModifyTextCommand, MoveKeyframeCommand,
    ReorderLayerCommand, SetKeyframeEasingCommand, SetTransformCommand,
};

/// Active tab in the overlay editor left sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeftTab {
    #[default]
    Layers,
    Assets,
}

/// Selection reference to an animated keyframe.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectedKeyframe {
    pub layer_id: LayerId,
    pub property: AnimProperty,
    pub time: f64,
}

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
    /// Active tab in the left sidebar
    pub active_left_tab: LeftTab,
    /// Raw embedded asset payload bytes keyed by relative container path (e.g. "assets/image.png")
    pub asset_bytes: HashMap<String, Vec<u8>>,
    /// Native asset pixel dimensions (width, height)
    pub asset_dimensions: HashMap<AssetId, (u32, u32)>,
    /// Currently selected keyframe in the dope sheet
    pub selected_keyframe: Option<SelectedKeyframe>,
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
            active_left_tab: LeftTab::Layers,
            asset_bytes: HashMap::new(),
            asset_dimensions: HashMap::new(),
            selected_keyframe: None,
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

    /// Imports an external media file, storing its bytes and metadata, and registers it in composition assets.
    pub fn import_asset(&mut self, path: &Path) -> Option<AssetId> {
        let bytes = std::fs::read(path).ok()?;
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        let kind = match ext.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" => AssetKind::Image,
            "mp4" | "mov" | "mkv" | "webm" | "avi" => AssetKind::Video,
            "ttf" | "otf" | "woff" | "woff2" => AssetKind::Font,
            "wav" | "mp3" | "ogg" | "flac" | "aac" => AssetKind::Audio,
            "gltf" | "glb" => AssetKind::Model3D,
            _ => AssetKind::Image,
        };

        let file_name = path
            .file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| "asset.bin".into());

        let asset_id = AssetId::new();
        let clean_path = format!("assets/{}", file_name);

        // Calculate deterministic SHA-256 / hash string
        let mut hasher = DefaultHasher::new();
        hasher.write(&bytes);
        let hash = format!("{:016x}", hasher.finish());

        // Probe image dimensions if applicable
        if kind == AssetKind::Image {
            if let Ok(img) = image::load_from_memory(&bytes) {
                self.asset_dimensions
                    .insert(asset_id, (img.width(), img.height()));
            }
        }

        self.asset_bytes.insert(clean_path.clone(), bytes.clone());
        self.composition.assets.push(AssetRef {
            id: asset_id,
            path: clean_path,
            kind,
            size_bytes: bytes.len() as u64,
            sha256: hash,
        });

        Some(asset_id)
    }

    /// Adds a bitmap image layer referencing an imported asset with undo support.
    pub fn add_image_layer_from_asset(
        &mut self,
        asset_id: AssetId,
        name: &str,
        width: f32,
        height: f32,
    ) -> LayerId {
        let id = LayerId::new();
        let mut layer = Layer::new(id, name, LayerContent::Image { asset: asset_id });
        layer.transform.position = zeno_core::Property::constant(Vec3::new(
            self.composition.width as f32 / 2.0,
            self.composition.height as f32 / 2.0,
            0.0,
        ));

        let (base_w, base_h) = self
            .asset_dimensions
            .get(&asset_id)
            .copied()
            .map(|(w, h)| (w as f32, h as f32))
            .unwrap_or((320.0, 180.0));

        let scale_x = (width / base_w.max(1.0)) * 100.0;
        let scale_y = (height / base_h.max(1.0)) * 100.0;
        layer.transform.scale_percent =
            zeno_core::Property::constant(Vec3::new(scale_x, scale_y, 100.0));

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

    /// Adds an animated scale keyframe to the selected layer at playhead time.
    pub fn add_scale_keyframe(&mut self, val: Vec3, easing: Easing) -> bool {
        if let Some(id) = self.selected_layer_id {
            let t = self.clock.time();
            let kf = Keyframe::new(t, val, easing);
            let cmd = AddScaleKeyframeCommand::new(id, kf);
            self.history.execute(Box::new(cmd), &mut self.composition);
            return true;
        }
        false
    }

    /// Adds an animated rotation keyframe to the selected layer at playhead time.
    pub fn add_rotation_keyframe(&mut self, val: Vec3, easing: Easing) -> bool {
        if let Some(id) = self.selected_layer_id {
            let t = self.clock.time();
            let kf = Keyframe::new(t, val, easing);
            let cmd = AddRotationKeyframeCommand::new(id, kf);
            self.history.execute(Box::new(cmd), &mut self.composition);
            return true;
        }
        false
    }

    /// Deletes a keyframe with undo support.
    pub fn delete_keyframe(&mut self, layer_id: LayerId, prop: AnimProperty, time: f64) -> bool {
        let cmd = DeleteKeyframeCommand::new(layer_id, prop, time);
        self.history.execute(Box::new(cmd), &mut self.composition);
        if let Some(ref sel) = self.selected_keyframe {
            if sel.layer_id == layer_id && sel.property == prop && (sel.time - time).abs() < 1e-3 {
                self.selected_keyframe = None;
            }
        }
        true
    }

    /// Retimes a keyframe with undo support.
    pub fn move_keyframe(
        &mut self,
        layer_id: LayerId,
        prop: AnimProperty,
        old_time: f64,
        new_time: f64,
    ) -> bool {
        let cmd = MoveKeyframeCommand::new(layer_id, prop, old_time, new_time);
        self.history.execute(Box::new(cmd), &mut self.composition);
        if let Some(ref mut sel) = self.selected_keyframe {
            if sel.layer_id == layer_id
                && sel.property == prop
                && (sel.time - old_time).abs() < 1e-3
            {
                sel.time = new_time;
            }
        }
        true
    }

    /// Sets keyframe easing with undo support.
    pub fn set_keyframe_easing(
        &mut self,
        layer_id: LayerId,
        prop: AnimProperty,
        time: f64,
        new_easing: Easing,
    ) -> bool {
        if let Some(layer) = self.composition.layers.iter().find(|l| l.id == layer_id) {
            let old_easing = match prop {
                AnimProperty::Position => layer
                    .transform
                    .position
                    .as_keyframes()
                    .iter()
                    .find(|k| (k.time_seconds - time).abs() < 1e-3)
                    .map(|k| k.easing_to_next),
                AnimProperty::Scale => layer
                    .transform
                    .scale_percent
                    .as_keyframes()
                    .iter()
                    .find(|k| (k.time_seconds - time).abs() < 1e-3)
                    .map(|k| k.easing_to_next),
                AnimProperty::Rotation => layer
                    .transform
                    .rotation_degrees
                    .as_keyframes()
                    .iter()
                    .find(|k| (k.time_seconds - time).abs() < 1e-3)
                    .map(|k| k.easing_to_next),
                AnimProperty::Opacity => layer
                    .transform
                    .opacity_percent
                    .as_keyframes()
                    .iter()
                    .find(|k| (k.time_seconds - time).abs() < 1e-3)
                    .map(|k| k.easing_to_next),
            };

            if let Some(old_easing) = old_easing {
                let cmd =
                    SetKeyframeEasingCommand::new(layer_id, prop, time, old_easing, new_easing);
                self.history.execute(Box::new(cmd), &mut self.composition);
                return true;
            }
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

    /// Saves the active composition as a standard `.arx` container package including all assets.
    pub fn save_to_arx(&self, path: &Path) -> Result<(), FormatError> {
        let manifest = Manifest::new(
            &self.name,
            "Zenocast Editor",
            self.composition.width,
            self.composition.height,
            self.composition.frames_per_second,
        );

        let mut pkg = ArxPackage::new(manifest, self.composition.clone());
        pkg.assets = self.asset_bytes.clone();
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

    /// Loads a composition from an existing `.arx` container package and extracts all assets.
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
        editor.asset_bytes = pkg.assets;

        // Reconstruct image dimensions from loaded asset bytes
        for asset_ref in &editor.composition.assets {
            let clean_path = if asset_ref.path.starts_with("assets/") {
                asset_ref.path.clone()
            } else {
                format!("assets/{}", asset_ref.path)
            };
            if let Some(data) = editor.asset_bytes.get(&clean_path) {
                if let Ok(img) = image::load_from_memory(data) {
                    editor
                        .asset_dimensions
                        .insert(asset_ref.id, (img.width(), img.height()));
                }
            }
        }

        editor.selected_layer_id = editor.composition.layers.first().map(|l| l.id);

        Ok(editor)
    }

    /// Alias for load_from_arx.
    pub fn load_arx(path: &Path) -> Result<Self, FormatError> {
        Self::load_from_arx(path)
    }
}
