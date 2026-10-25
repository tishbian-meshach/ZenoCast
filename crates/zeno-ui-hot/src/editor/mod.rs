//! Motion Graphics Overlay Editor (Milestone M9).
//!
//! Provides the complete standalone motion graphics editor:
//! - Viewport canvas with interactive transform gizmos (Move, Scale, Anchor)
//! - Layer hierarchy tree with reordering, visibility, locking, add/delete
//! - Properties inspector with keyframe stopwatch toggles
//! - Bottom timeline with playhead scrubbing, keyframe diamonds, timeline markers
//! - Interactive Bezier Curve Editor for keyframe easing adjustment
//! - Asset manager for importing and embedding media assets
//! - Command pattern with full Undo / Redo history
//! - Direct `.arx` container package load and save

pub mod assets;
pub mod canvas;
pub mod command;
pub mod layers_panel;
pub mod properties_panel;
pub mod state;
pub mod timeline;

pub use assets::ui_assets_panel;
pub use canvas::ui_canvas;
pub use command::{
    AddLayerCommand, AddOpacityKeyframeCommand, AddPositionKeyframeCommand,
    AddRotationKeyframeCommand, AddScaleKeyframeCommand, AnimProperty, CommandHistory,
    DeleteKeyframeCommand, DeleteLayerCommand, EditorCommand, ModifyTextCommand,
    MoveKeyframeCommand, ReorderLayerCommand, SetKeyframeEasingCommand, SetTransformCommand,
};
pub use layers_panel::ui_layers_panel;
pub use properties_panel::ui_properties_panel;
pub use state::{LeftTab, OverlayEditor, TimelineMarker};
pub use timeline::{ui_timeline, TimelineMode};
