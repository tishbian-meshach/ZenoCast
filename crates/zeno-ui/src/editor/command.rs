//! Command pattern implementation for Edit Overlay mode undo/redo history.

use zeno_core::{Keyframe, LayerId, Vec3};
use zeno_engine::{Composition, Layer, LayerContent, Transform3D};

/// Trait implemented by all reversible editor modifications.
pub trait EditorCommand: Send + Sync {
    /// Applies the modification to the composition.
    fn execute(&mut self, comp: &mut Composition);

    /// Inverts the modification, restoring the previous state.
    fn undo(&mut self, comp: &mut Composition);

    /// Short human-readable description for UI undo/redo menus.
    fn description(&self) -> &str;
}

/// Command history manager supporting unlimited undo and redo.
#[derive(Default)]
pub struct CommandHistory {
    undo_stack: Vec<Box<dyn EditorCommand>>,
    redo_stack: Vec<Box<dyn EditorCommand>>,
}

impl CommandHistory {
    /// Creates a new empty command history.
    pub fn new() -> Self {
        Self::default()
    }

    /// Executes a command on the composition and pushes it onto the undo stack.
    pub fn execute(&mut self, mut cmd: Box<dyn EditorCommand>, comp: &mut Composition) {
        cmd.execute(comp);
        self.undo_stack.push(cmd);
        self.redo_stack.clear();
    }

    /// Reverts the most recent command.
    pub fn undo(&mut self, comp: &mut Composition) -> bool {
        if let Some(mut cmd) = self.undo_stack.pop() {
            cmd.undo(comp);
            self.redo_stack.push(cmd);
            true
        } else {
            false
        }
    }

    /// Re-applies the most recently reverted command.
    pub fn redo(&mut self, comp: &mut Composition) -> bool {
        if let Some(mut cmd) = self.redo_stack.pop() {
            cmd.execute(comp);
            self.undo_stack.push(cmd);
            true
        } else {
            false
        }
    }

    /// Checks if an undo operation is available.
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    /// Checks if a redo operation is available.
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    /// Returns the description of the action that would be undone.
    pub fn undo_description(&self) -> Option<&str> {
        self.undo_stack.last().map(|c| c.description())
    }

    /// Returns the description of the action that would be redone.
    pub fn redo_description(&self) -> Option<&str> {
        self.redo_stack.last().map(|c| c.description())
    }

    /// Number of commands available to undo.
    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    /// Number of commands available to redo.
    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    /// Clears all undo and redo history.
    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }
}

/// Command: Adds a new layer to the composition.
pub struct AddLayerCommand {
    layer: Option<Layer>,
    index: usize,
}

impl AddLayerCommand {
    pub fn new(layer: Layer, index: usize) -> Self {
        Self {
            layer: Some(layer),
            index,
        }
    }
}

impl EditorCommand for AddLayerCommand {
    fn execute(&mut self, comp: &mut Composition) {
        if let Some(layer) = self.layer.take() {
            let idx = self.index.min(comp.layers.len());
            comp.layers.insert(idx, layer);
        }
    }

    fn undo(&mut self, comp: &mut Composition) {
        if self.index < comp.layers.len() {
            self.layer = Some(comp.layers.remove(self.index));
        }
    }

    fn description(&self) -> &str {
        "Add Layer"
    }
}

/// Command: Deletes an existing layer from the composition.
pub struct DeleteLayerCommand {
    layer: Option<Layer>,
    index: usize,
    layer_id: LayerId,
}

impl DeleteLayerCommand {
    pub fn new(layer_id: LayerId, comp: &Composition) -> Option<Self> {
        let index = comp.layers.iter().position(|l| l.id == layer_id)?;
        let layer = comp.layers[index].clone();
        Some(Self {
            layer: Some(layer),
            index,
            layer_id,
        })
    }
}

impl EditorCommand for DeleteLayerCommand {
    fn execute(&mut self, comp: &mut Composition) {
        if let Some(pos) = comp.layers.iter().position(|l| l.id == self.layer_id) {
            self.index = pos;
            self.layer = Some(comp.layers.remove(pos));
        }
    }

    fn undo(&mut self, comp: &mut Composition) {
        if let Some(layer) = self.layer.take() {
            let idx = self.index.min(comp.layers.len());
            comp.layers.insert(idx, layer);
        }
    }

    fn description(&self) -> &str {
        "Delete Layer"
    }
}

/// Command: Reorders layers via drag-and-drop.
pub struct ReorderLayerCommand {
    from_index: usize,
    to_index: usize,
}

impl ReorderLayerCommand {
    pub fn new(from_index: usize, to_index: usize) -> Self {
        Self {
            from_index,
            to_index,
        }
    }
}

impl EditorCommand for ReorderLayerCommand {
    fn execute(&mut self, comp: &mut Composition) {
        if self.from_index < comp.layers.len() && self.to_index < comp.layers.len() {
            let layer = comp.layers.remove(self.from_index);
            comp.layers.insert(self.to_index, layer);
        }
    }

    fn undo(&mut self, comp: &mut Composition) {
        if self.to_index < comp.layers.len() && self.from_index < comp.layers.len() {
            let layer = comp.layers.remove(self.to_index);
            comp.layers.insert(self.from_index, layer);
        }
    }

    fn description(&self) -> &str {
        "Reorder Layers"
    }
}

/// Command: Modifies a layer's 3D transform.
pub struct SetTransformCommand {
    layer_id: LayerId,
    old_transform: Transform3D,
    new_transform: Transform3D,
}

impl SetTransformCommand {
    pub fn new(layer_id: LayerId, old_transform: Transform3D, new_transform: Transform3D) -> Self {
        Self {
            layer_id,
            old_transform,
            new_transform,
        }
    }
}

impl EditorCommand for SetTransformCommand {
    fn execute(&mut self, comp: &mut Composition) {
        if let Some(layer) = comp.layers.iter_mut().find(|l| l.id == self.layer_id) {
            layer.transform = self.new_transform.clone();
        }
    }

    fn undo(&mut self, comp: &mut Composition) {
        if let Some(layer) = comp.layers.iter_mut().find(|l| l.id == self.layer_id) {
            layer.transform = self.old_transform.clone();
        }
    }

    fn description(&self) -> &str {
        "Modify Transform"
    }
}

/// Command: Adds a position keyframe to a layer.
pub struct AddPositionKeyframeCommand {
    layer_id: LayerId,
    keyframe: Keyframe<Vec3>,
}

impl AddPositionKeyframeCommand {
    pub fn new(layer_id: LayerId, keyframe: Keyframe<Vec3>) -> Self {
        Self { layer_id, keyframe }
    }
}

impl EditorCommand for AddPositionKeyframeCommand {
    fn execute(&mut self, comp: &mut Composition) {
        if let Some(layer) = comp.layers.iter_mut().find(|l| l.id == self.layer_id) {
            let mut kfs = layer.transform.position.as_keyframes().to_vec();
            kfs.retain(|k| (k.time_seconds - self.keyframe.time_seconds).abs() > 1e-4);
            kfs.push(self.keyframe.clone());
            kfs.sort_by(|a, b| a.time_seconds.partial_cmp(&b.time_seconds).unwrap());
            layer.transform.position = zeno_core::Property::keyframes(kfs);
        }
    }

    fn undo(&mut self, comp: &mut Composition) {
        if let Some(layer) = comp.layers.iter_mut().find(|l| l.id == self.layer_id) {
            let mut kfs = layer.transform.position.as_keyframes().to_vec();
            kfs.retain(|k| (k.time_seconds - self.keyframe.time_seconds).abs() > 1e-4);
            layer.transform.position = zeno_core::Property::keyframes(kfs);
        }
    }

    fn description(&self) -> &str {
        "Add Position Keyframe"
    }
}

/// Command: Adds an opacity keyframe to a layer.
pub struct AddOpacityKeyframeCommand {
    layer_id: LayerId,
    keyframe: Keyframe<f32>,
}

impl AddOpacityKeyframeCommand {
    pub fn new(layer_id: LayerId, keyframe: Keyframe<f32>) -> Self {
        Self { layer_id, keyframe }
    }
}

impl EditorCommand for AddOpacityKeyframeCommand {
    fn execute(&mut self, comp: &mut Composition) {
        if let Some(layer) = comp.layers.iter_mut().find(|l| l.id == self.layer_id) {
            let mut kfs = layer.transform.opacity_percent.as_keyframes().to_vec();
            kfs.retain(|k| (k.time_seconds - self.keyframe.time_seconds).abs() > 1e-4);
            kfs.push(self.keyframe.clone());
            kfs.sort_by(|a, b| a.time_seconds.partial_cmp(&b.time_seconds).unwrap());
            layer.transform.opacity_percent = zeno_core::Property::keyframes(kfs);
        }
    }

    fn undo(&mut self, comp: &mut Composition) {
        if let Some(layer) = comp.layers.iter_mut().find(|l| l.id == self.layer_id) {
            let mut kfs = layer.transform.opacity_percent.as_keyframes().to_vec();
            kfs.retain(|k| (k.time_seconds - self.keyframe.time_seconds).abs() > 1e-4);
            layer.transform.opacity_percent = zeno_core::Property::keyframes(kfs);
        }
    }

    fn description(&self) -> &str {
        "Add Opacity Keyframe"
    }
}

/// Command: Modifies text content.
pub struct ModifyTextCommand {
    layer_id: LayerId,
    old_text: String,
    new_text: String,
}

impl ModifyTextCommand {
    pub fn new(layer_id: LayerId, old_text: String, new_text: String) -> Self {
        Self {
            layer_id,
            old_text,
            new_text,
        }
    }
}

impl EditorCommand for ModifyTextCommand {
    fn execute(&mut self, comp: &mut Composition) {
        if let Some(layer) = comp.layers.iter_mut().find(|l| l.id == self.layer_id) {
            if let LayerContent::Text(ref mut t) = layer.content {
                t.text = zeno_core::Property::constant(self.new_text.clone());
            }
        }
    }

    fn undo(&mut self, comp: &mut Composition) {
        if let Some(layer) = comp.layers.iter_mut().find(|l| l.id == self.layer_id) {
            if let LayerContent::Text(ref mut t) = layer.content {
                t.text = zeno_core::Property::constant(self.old_text.clone());
            }
        }
    }

    fn description(&self) -> &str {
        "Modify Text"
    }
}
