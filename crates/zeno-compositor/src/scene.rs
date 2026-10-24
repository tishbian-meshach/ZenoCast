//! Scene and SceneItem model with transform, crop, blend, and nesting capabilities.

use glam::{Mat4, Vec2, Vec3};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use zeno_core::{Color, SceneId, SourceId, TransitionId};
use zeno_engine::BlendMode;

/// Policy controlling how entering and exiting overlays are scheduled during scene transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum OverlayTiming {
    /// Out animation of leaving scene and In animation of entering scene run concurrently during transition.
    #[default]
    Overlap,
    /// Out animation completes first, then transition executes, then In animation starts.
    Sequential,
}

/// 2D rectangular pixel crop specification.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Crop {
    /// Inset from left edge in pixels.
    pub left: f32,
    /// Inset from top edge in pixels.
    pub top: f32,
    /// Inset from right edge in pixels.
    pub right: f32,
    /// Inset from bottom edge in pixels.
    pub bottom: f32,
}

impl Crop {
    /// Zero crop (no pixels clipped).
    pub const ZERO: Self = Self {
        left: 0.0,
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
    };

    /// Returns `true` if no crop insets are applied.
    #[inline]
    pub fn is_zero(&self) -> bool {
        self.left <= 0.0 && self.top <= 0.0 && self.right <= 0.0 && self.bottom <= 0.0
    }
}

impl Default for Crop {
    fn default() -> Self {
        Self::ZERO
    }
}

/// 2D affine transformation applied to a scene item.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ItemTransform {
    /// Translation coordinates in canvas pixels (x, y).
    pub position: Vec2,
    /// Scale multipliers (1.0 = 100% natural dimensions).
    pub scale: Vec2,
    /// Clockwise rotation in degrees.
    pub rotation_degrees: f32,
    /// Normalized anchor point inside the item [0.0..1.0] (default: 0.5, 0.5 = center).
    pub anchor_point: Vec2,
}

impl ItemTransform {
    /// Default identity transform at (0, 0) with center anchor.
    pub fn new(position: Vec2) -> Self {
        Self {
            position,
            scale: Vec2::ONE,
            rotation_degrees: 0.0,
            anchor_point: Vec2::new(0.5, 0.5),
        }
    }

    /// Computes the 4x4 model matrix mapping a unit quad [-0.5..0.5] to canvas space.
    pub fn to_model_matrix(&self, item_width: f32, item_height: f32) -> Mat4 {
        let w = (item_width * self.scale.x).max(1.0);
        let h = (item_height * self.scale.y).max(1.0);

        // Compute translation offset based on anchor point relative to center
        let anchor_offset_x = (0.5 - self.anchor_point.x) * w;
        let anchor_offset_y = (0.5 - self.anchor_point.y) * h;

        let rot_rad = self.rotation_degrees.to_radians();

        Mat4::from_translation(Vec3::new(
            self.position.x + anchor_offset_x,
            self.position.y + anchor_offset_y,
            0.0,
        )) * Mat4::from_rotation_z(rot_rad)
            * Mat4::from_scale(Vec3::new(w, h, 1.0))
    }
}

impl Default for ItemTransform {
    fn default() -> Self {
        Self {
            position: Vec2::ZERO,
            scale: Vec2::ONE,
            rotation_degrees: 0.0,
            anchor_point: Vec2::new(0.5, 0.5),
        }
    }
}

/// Target content of a scene item: physical source or nested scene.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SceneItemSource {
    /// Visual ingest source (capture card, window, webcam, image, etc.).
    Source(SourceId),
    /// Sub-scene nested inside the parent scene.
    NestedScene(SceneId),
}

/// GPU post-processing filter applied to an individual scene item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ItemFilter {
    /// Color adjustments: brightness [-1..1], contrast [0..2], saturation [0..2].
    ColorAdjustment {
        brightness: f32,
        contrast: f32,
        saturation: f32,
    },
    /// Chroma key green-screen removal.
    ChromaKey {
        key_color: Color,
        similarity: f32,
        smoothness: f32,
    },
}

/// A single visual element placed inside a production scene.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SceneItem {
    /// Unique item instance ID.
    pub id: Uuid,
    /// Friendly user-visible item name.
    pub name: String,
    /// Attached media source or nested sub-scene.
    pub source: SceneItemSource,
    /// 2D position, scale, and rotation.
    pub transform: ItemTransform,
    /// Pixel clipping boundaries.
    pub crop: Crop,
    /// Visibility toggle.
    pub visible: bool,
    /// Layer opacity [0.0..1.0].
    pub opacity: f32,
    /// Blending mode against underlying items.
    pub blend_mode: BlendMode,
    /// Chain of GPU shaders/filters applied to this item.
    pub filters: Vec<ItemFilter>,
}

impl SceneItem {
    /// Creates a new visible scene item with default transform and normal blend mode.
    pub fn new(name: impl Into<String>, source: SceneItemSource) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            source,
            transform: ItemTransform::default(),
            crop: Crop::default(),
            visible: true,
            opacity: 1.0,
            blend_mode: BlendMode::Normal,
            filters: Vec::new(),
        }
    }

    /// Sets custom position on canvas.
    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.transform.position = Vec2::new(x, y);
        self
    }

    /// Sets item scale multiplier.
    pub fn with_scale(mut self, sx: f32, sy: f32) -> Self {
        self.transform.scale = Vec2::new(sx, sy);
        self
    }

    /// Sets crop insets.
    pub fn with_crop(mut self, left: f32, top: f32, right: f32, bottom: f32) -> Self {
        self.crop = Crop {
            left,
            top,
            right,
            bottom,
        };
        self
    }

    /// Sets blend mode.
    pub fn with_blend_mode(mut self, blend_mode: BlendMode) -> Self {
        self.blend_mode = blend_mode;
        self
    }
}

/// A complete broadcast scene containing an ordered stack of visual items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    /// Unique scene identifier.
    pub id: SceneId,
    /// User-visible scene name (e.g. "Starting Soon", "Caster Desk", "Gameplay").
    pub name: String,
    /// Ordered list of visual items rendered bottom-to-top.
    pub items: Vec<SceneItem>,
    /// Background clear color for this scene.
    pub background: Color,
    /// Overlay timing policy for transition handoffs.
    pub overlay_timing: OverlayTiming,
    /// Optional default transition when entering this scene.
    pub default_transition: Option<TransitionId>,
}

impl Scene {
    /// Creates a new scene with transparent broadcast background.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: SceneId::new(),
            name: name.into(),
            items: Vec::new(),
            background: Color::TRANSPARENT,
            overlay_timing: OverlayTiming::Overlap,
            default_transition: None,
        }
    }

    /// Adds a scene item to the top of the stack.
    pub fn add_item(&mut self, item: SceneItem) {
        self.items.push(item);
    }

    /// Removes an item by UUID.
    pub fn remove_item(&mut self, item_id: Uuid) -> Option<SceneItem> {
        if let Some(pos) = self.items.iter().position(|i| i.id == item_id) {
            Some(self.items.remove(pos))
        } else {
            None
        }
    }

    /// Moves an item up or down in the stack.
    pub fn move_item(&mut self, item_id: Uuid, new_index: usize) -> bool {
        if let Some(current_pos) = self.items.iter().position(|i| i.id == item_id) {
            let item = self.items.remove(current_pos);
            let target = new_index.min(self.items.len());
            self.items.insert(target, item);
            true
        } else {
            false
        }
    }

    /// Returns a reference to an item by ID.
    pub fn get_item(&self, item_id: Uuid) -> Option<&SceneItem> {
        self.items.iter().find(|i| i.id == item_id)
    }

    /// Returns a mutable reference to an item by ID.
    pub fn get_item_mut(&mut self, item_id: Uuid) -> Option<&mut SceneItem> {
        self.items.iter_mut().find(|i| i.id == item_id)
    }
}
