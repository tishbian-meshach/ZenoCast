//! Scene graph, layer contents, composition, and animation state model.

use serde::{Deserialize, Serialize};
use zeno_core::{
    AssetId, Color, LayerId, Lifecycle, Mat4, Property, Quat, VariableDeclaration, Vec2, Vec3,
};

/// 3D affine transformation parameters for a layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Transform3D {
    /// 3D spatial position (x, y, z).
    pub position: Property<Vec3>,
    /// Anchor / pivot point around which rotation and scaling occur.
    pub anchor_point: Property<Vec3>,
    /// Scale factor percentage (100.0 = 1.0x).
    pub scale_percent: Property<Vec3>,
    /// Euler rotation angles in degrees (x, y, z).
    pub rotation_degrees: Property<Vec3>,
    /// Layer opacity in percent (0.0 to 100.0).
    pub opacity_percent: Property<f32>,
}

impl Default for Transform3D {
    fn default() -> Self {
        Self {
            position: Property::constant(Vec3::ZERO),
            anchor_point: Property::constant(Vec3::ZERO),
            scale_percent: Property::constant(Vec3::new(100.0, 100.0, 100.0)),
            rotation_degrees: Property::constant(Vec3::ZERO),
            opacity_percent: Property::constant(100.0),
        }
    }
}

impl Transform3D {
    /// Constructs a 4x4 local transformation matrix from resolved transform values.
    pub fn compute_matrix(
        position: Vec3,
        anchor: Vec3,
        scale_percent: Vec3,
        rotation_degrees: Vec3,
    ) -> Mat4 {
        let scale = scale_percent / 100.0;
        let rot_rad = Vec3::new(
            rotation_degrees.x.to_radians(),
            rotation_degrees.y.to_radians(),
            rotation_degrees.z.to_radians(),
        );

        let rotation = Quat::from_euler(glam::EulerRot::YXZ, rot_rad.y, rot_rad.x, rot_rad.z);

        // Matrix order: T(position) * R(rotation) * S(scale) * T(-anchor)
        Mat4::from_translation(position)
            * Mat4::from_quat(rotation)
            * Mat4::from_scale(scale)
            * Mat4::from_translation(-anchor)
    }
}

/// GPU blend mode specifying how a layer composites onto underlying layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum BlendMode {
    /// Normal alpha compositing (standard over).
    #[default]
    Normal,
    /// Additive blending (linear dodge).
    Add,
    /// Multiply blending.
    Multiply,
    /// Screen blending.
    Screen,
    /// Overlay blending.
    Overlay,
    /// Darken mode.
    Darken,
    /// Lighten mode.
    Lighten,
    /// Color dodge.
    ColorDodge,
    /// Color burn.
    ColorBurn,
    /// Hard light.
    HardLight,
    /// Soft light.
    SoftLight,
    /// Difference mode.
    Difference,
    /// Exclusion mode.
    Exclusion,
}

/// Visual post-processing or rasterization effects.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Effect {
    /// Gaussian blur effect.
    Blur {
        /// Radius in pixels.
        radius_px: Property<f32>,
    },
    /// Outer glow effect.
    Glow {
        /// Glow radius in pixels.
        radius_px: Property<f32>,
        /// Glow intensity multiplier.
        intensity: Property<f32>,
        /// Glow color.
        color: Property<Color>,
    },
    /// Drop shadow effect.
    DropShadow {
        /// Pixel offset from source.
        offset: Property<Vec2>,
        /// Blur radius in pixels.
        blur_radius_px: Property<f32>,
        /// Shadow tint color.
        color: Property<Color>,
        /// Shadow opacity (0.0 to 1.0).
        opacity: Property<f32>,
    },
    /// Color correction and grading.
    ColorCorrection {
        /// Brightness offset (-1.0 to 1.0).
        brightness: Property<f32>,
        /// Contrast multiplier (0.0 to 2.0).
        contrast: Property<f32>,
        /// Saturation multiplier (0.0 to 2.0).
        saturation: Property<f32>,
        /// Hue rotation in degrees.
        hue_degrees: Property<f32>,
    },
    /// Green-screen / color-key extraction.
    ChromaKey {
        /// Target key color to extract.
        key_color: Property<Color>,
        /// Similarity tolerance threshold (0.0 to 1.0).
        similarity: Property<f32>,
        /// Edge smoothness softness (0.0 to 1.0).
        smoothness: Property<f32>,
    },
    /// Border stroke effect.
    Stroke {
        /// Stroke line width in pixels.
        width_px: Property<f32>,
        /// Stroke color.
        color: Property<Color>,
    },
}

/// Masking mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum MaskMode {
    /// Alpha mask (source alpha cuts target).
    #[default]
    Alpha,
    /// Inverted alpha mask.
    InvertedAlpha,
    /// Luminance mask (source brightness modulates alpha).
    Luma,
    /// Inverted luminance mask.
    InvertedLuma,
}

/// Layer masking configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Mask {
    /// Mask blending mode.
    pub mode: MaskMode,
    /// Layer whose alpha/luminance acts as the mask.
    pub mask_layer: LayerId,
}

/// Horizontal text alignment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TextAlignment {
    /// Left aligned.
    #[default]
    Left,
    /// Centered.
    Center,
    /// Right aligned.
    Right,
}

/// Formatted text content properties.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextContent {
    /// Dynamic text string.
    pub text: Property<String>,
    /// Font family name (e.g. "Inter", "JetBrains Mono").
    pub font_family: String,
    /// Font size in pixels.
    pub font_size: Property<f32>,
    /// Font weight (100 to 900, 400 = Regular, 700 = Bold).
    pub font_weight: u16,
    /// Fill text color.
    pub color: Property<Color>,
    /// Alignment.
    pub alignment: TextAlignment,
    /// Tracking / letter spacing in pixels.
    pub letter_spacing: Property<f32>,
    /// Line height multiplier (1.0 = normal).
    pub line_height_multiplier: Property<f32>,
    /// Whether to wrap long text at max_width.
    pub word_wrap: bool,
    /// Maximum layout width in pixels.
    pub max_width: Option<f32>,
}

impl Default for TextContent {
    fn default() -> Self {
        Self {
            text: Property::constant(String::new()),
            font_family: "Inter".to_string(),
            font_size: Property::constant(24.0),
            font_weight: 400,
            color: Property::constant(Color::WHITE),
            alignment: TextAlignment::Left,
            letter_spacing: Property::constant(0.0),
            line_height_multiplier: Property::constant(1.2),
            word_wrap: false,
            max_width: None,
        }
    }
}

/// Color gradient stop.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GradientStop {
    /// Position along the gradient ramp [0.0, 1.0].
    pub offset: f32,
    /// Color at this stop.
    pub color: Color,
}

/// Brush used to fill vector shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Fill {
    /// Flat solid color.
    Solid(Property<Color>),
    /// Two-point linear gradient.
    LinearGradient {
        /// Normalized start position.
        start: Vec2,
        /// Normalized end position.
        end: Vec2,
        /// Color stops.
        stops: Vec<GradientStop>,
    },
    /// Radial gradient.
    RadialGradient {
        /// Normalized center position.
        center: Vec2,
        /// Normalized radius.
        radius: f32,
        /// Color stops.
        stops: Vec<GradientStop>,
    },
}

/// Outline stroke for vector shapes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Stroke {
    /// Stroke line width in pixels.
    pub width: Property<f32>,
    /// Stroke line color.
    pub color: Property<Color>,
}

/// Vector shape geometry definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ShapeGeometry {
    /// Simple rectangle.
    Rectangle {
        /// Width and height in pixels.
        size: Property<Vec2>,
    },
    /// Rounded corner rectangle.
    RoundedRectangle {
        /// Width and height in pixels.
        size: Property<Vec2>,
        /// Corner rounding radius in pixels.
        corner_radius: Property<f32>,
    },
    /// Ellipse / circle.
    Ellipse {
        /// Radius in x and y.
        radius: Property<Vec2>,
    },
    /// Arbitrary SVG path data.
    Path {
        /// SVG path commands string (e.g. "M 0 0 L 100 100 Z").
        svg_data: String,
    },
}

/// Vector shape content (geometry + optional fill + optional stroke).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShapeContent {
    /// Shape geometry.
    pub geometry: ShapeGeometry,
    /// Interior fill brush.
    pub fill: Option<Fill>,
    /// Border outline stroke.
    pub stroke: Option<Stroke>,
}

/// Particle emitter configuration for GPU particle simulation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParticleEmitter {
    /// Particles emitted per second.
    pub rate_per_second: Property<f32>,
    /// Lifetime of each particle in seconds.
    pub lifetime_seconds: Property<f32>,
    /// Initial launch velocity in pixels/second.
    pub initial_speed: Property<f32>,
    /// Angular dispersion spread in degrees.
    pub spread_degrees: Property<f32>,
    /// Particle diameter at birth.
    pub start_size_px: Property<f32>,
    /// Particle diameter at death.
    pub end_size_px: Property<f32>,
    /// Color at birth.
    pub start_color: Property<Color>,
    /// Color at death.
    pub end_color: Property<Color>,
    /// Gravitational acceleration vector (pixels/s^2).
    pub gravity: Property<Vec2>,
    /// Additive blending flag.
    pub additive_blend: bool,
    /// Optional particle sprite texture asset.
    pub sprite_asset: Option<AssetId>,
    /// Deterministic RNG seed for scrubbing consistency.
    pub seed: u64,
}

impl Default for ParticleEmitter {
    fn default() -> Self {
        Self {
            rate_per_second: Property::constant(50.0),
            lifetime_seconds: Property::constant(2.0),
            initial_speed: Property::constant(100.0),
            spread_degrees: Property::constant(45.0),
            start_size_px: Property::constant(8.0),
            end_size_px: Property::constant(0.0),
            start_color: Property::constant(Color::WHITE),
            end_color: Property::constant(Color::TRANSPARENT),
            gravity: Property::constant(Vec2::new(0.0, 98.0)),
            additive_blend: true,
            sprite_asset: None,
            seed: 1337,
        }
    }
}

/// 3D Perspective camera parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CameraSettings {
    /// Vertical field of view in degrees.
    pub field_of_view_degrees: Property<f32>,
    /// 3D camera position.
    pub position: Property<Vec3>,
    /// 3D look-at target.
    pub target: Property<Vec3>,
    /// Near clipping plane distance.
    pub near_plane: f32,
    /// Far clipping plane distance.
    pub far_plane: f32,
}

impl Default for CameraSettings {
    fn default() -> Self {
        Self {
            field_of_view_degrees: Property::constant(60.0),
            position: Property::constant(Vec3::new(960.0, 540.0, -935.3074)),
            target: Property::constant(Vec3::new(960.0, 540.0, 0.0)),
            near_plane: 0.1,
            far_plane: 10000.0,
        }
    }
}

impl CameraSettings {
    /// Constructs a standard broadcast perspective camera matching composition dimensions.
    ///
    /// At default distance `(height / 2.0) / tan(fov / 2)`, an object at z = 0.0 with size
    /// equal to the composition resolution maps 1:1 with canvas pixels.
    pub fn broadcast_default(width: u32, height: u32) -> Self {
        let fov: f32 = 60.0;
        let half_fov_rad = (fov / 2.0).to_radians();
        let dist = (height as f32 / 2.0) / half_fov_rad.tan();
        let center_x = width as f32 / 2.0;
        let center_y = height as f32 / 2.0;

        Self {
            field_of_view_degrees: Property::constant(fov),
            position: Property::constant(Vec3::new(center_x, center_y, -dist)),
            target: Property::constant(Vec3::new(center_x, center_y, 0.0)),
            near_plane: 0.1,
            far_plane: 10000.0,
        }
    }

    /// Evaluates camera properties at timestamp `t` and computes view, projection, and combined view-proj matrices.
    pub fn compute_view_projection(&self, width: u32, height: u32, t: f64) -> (Mat4, Mat4, Mat4) {
        let fov = self.field_of_view_degrees.evaluate_at(t);
        let pos = self.position.evaluate_at(t);
        let tgt = self.target.evaluate_at(t);
        let aspect = (width as f32) / (height as f32).max(1.0);

        let proj = Mat4::perspective_lh(
            fov.to_radians(),
            aspect,
            self.near_plane.max(0.01),
            self.far_plane.max(10.0),
        );
        let view = Mat4::look_at_lh(pos, tgt, Vec3::new(0.0, -1.0, 0.0));
        let view_proj = proj * view;

        (view, proj, view_proj)
    }
}

/// Specific content payload of a layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum LayerContent {
    /// Null object layer used as a parent transform pivot or invisible anchor.
    Null,
    /// Styled vector text layout.
    Text(TextContent),
    /// Bitmap image texture.
    Image {
        /// Asset identifier.
        asset: AssetId,
    },
    /// Video clip playback layer.
    Video {
        /// Asset identifier.
        asset: AssetId,
        /// Loop video playback.
        loop_playback: bool,
        /// Playback speed multiplier (1.0 = normal speed).
        speed: Property<f32>,
    },
    /// Vector shape geometry (rectangle, ellipse, path).
    Shape(ShapeContent),
    /// Group containing nested child layers.
    Group {
        /// Ordered child layer identifiers.
        children: Vec<LayerId>,
    },
    /// GPU accelerated particle emitter.
    Particles(ParticleEmitter),
    /// 3D glTF mesh model.
    Model3D {
        /// Asset identifier.
        asset: AssetId,
    },
    /// Virtual 3D camera layer.
    Camera(CameraSettings),
    /// Repeater layer instancing a template group once per data row.
    Repeater {
        /// Template layer or group to clone for each row.
        template_group: LayerId,
        /// Data array path in data store.
        data_path: String,
        /// Maximum instances to spawn.
        max_rows: u32,
        /// Positional offset between subsequent rows.
        row_offset: Property<Vec2>,
    },
}

/// An individual scene layer in a composition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    /// Unique layer identifier.
    pub id: LayerId,
    /// Human-readable layer name.
    pub name: String,
    /// Parent layer identifier for hierarchical transforms.
    pub parent: Option<LayerId>,
    /// Entrance in-point in seconds.
    pub in_point_seconds: f64,
    /// Exit out-point in seconds.
    pub out_point_seconds: f64,
    /// 3D / 2.5D transform properties.
    pub transform: Transform3D,
    /// Whether layer participates in 3D perspective and z-depth ordering.
    pub is_3d: bool,
    /// Visibility toggle.
    pub visible: bool,
    /// GPU blending mode.
    pub blend_mode: BlendMode,
    /// Applied visual effects.
    pub effects: Vec<Effect>,
    /// Optional clipping or luminance mask.
    pub mask: Option<Mask>,
    /// Specific content payload.
    pub content: LayerContent,
}

impl Layer {
    /// Create a new layer with default settings.
    pub fn new(id: LayerId, name: impl Into<String>, content: LayerContent) -> Self {
        Self {
            id,
            name: name.into(),
            parent: None,
            in_point_seconds: 0.0,
            out_point_seconds: 10.0,
            transform: Transform3D::default(),
            is_3d: false,
            visible: true,
            blend_mode: BlendMode::Normal,
            effects: Vec::new(),
            mask: None,
            content,
        }
    }

    /// Check if layer is active at given timestamp.
    pub fn is_active_at(&self, time_seconds: f64) -> bool {
        self.visible
            && time_seconds >= self.in_point_seconds
            && time_seconds <= self.out_point_seconds
    }
}

/// Playback repetition mode for an animation state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum StatePlayback {
    /// Play once to the end, then stop or transition.
    #[default]
    Once,
    /// Repeat continuously while state is active.
    Loop,
    /// Play once and hold the final keyframe value.
    HoldAtEnd,
}

/// Named animation state or custom action trigger range.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnimationState {
    /// State name. Reserved names: "in", "loop", "out". Custom names: "score_change", "highlight", etc.
    pub name: String,
    /// Start timestamp in seconds.
    pub start_seconds: f64,
    /// End timestamp in seconds.
    pub end_seconds: f64,
    /// Playback mode.
    pub playback: StatePlayback,
}

impl AnimationState {
    /// Construct a new animation state.
    pub fn new(
        name: impl Into<String>,
        start_seconds: f64,
        end_seconds: f64,
        playback: StatePlayback,
    ) -> Self {
        Self {
            name: name.into(),
            start_seconds,
            end_seconds,
            playback,
        }
    }

    /// State duration in seconds.
    pub fn duration(&self) -> f64 {
        (self.end_seconds - self.start_seconds).max(0.0)
    }
}

/// Media asset type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AssetKind {
    /// Raster image (PNG, JPEG, WebP).
    Image,
    /// Video clip (MP4, WebM).
    Video,
    /// TrueType / OpenType font file.
    Font,
    /// Audio sample or loop.
    Audio,
    /// 3D glTF / GLB model.
    Model3D,
}

/// Reference to an external or package-internal asset.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssetRef {
    /// Unique asset identifier.
    pub id: AssetId,
    /// Relative path within `.arx` container `assets/` directory.
    pub path: String,
    /// Asset kind.
    pub kind: AssetKind,
    /// Asset size in bytes.
    pub size_bytes: u64,
    /// SHA-256 integrity hash hex string.
    pub sha256: String,
}

/// A complete motion-graphics composition containing layers, variables, states, and assets.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Composition {
    /// Canvas width in pixels (e.g. 1920).
    pub width: u32,
    /// Canvas height in pixels (e.g. 1080).
    pub height: u32,
    /// Frame rate in frames per second (e.g. 60.0).
    pub frames_per_second: f32,
    /// Total duration of composition timeline in seconds.
    pub duration_seconds: f64,
    /// Layer stack in bottom-to-top rendering order.
    pub layers: Vec<Layer>,
    /// Declared variable schema for live data bindings.
    pub variables: Vec<VariableDeclaration>,
    /// Named animation state ranges (in, loop, out, custom triggers).
    pub states: Vec<AnimationState>,
    /// Referenced media assets.
    pub assets: Vec<AssetRef>,
    /// Optional three-phase In/Loop/Out lifecycle configuration.
    pub lifecycle: Option<Lifecycle>,
}

impl Default for Composition {
    fn default() -> Self {
        Self {
            width: 1920,
            height: 1080,
            frames_per_second: 60.0,
            duration_seconds: 10.0,
            layers: Vec::new(),
            variables: Vec::new(),
            states: vec![
                AnimationState::new("in", 0.0, 1.0, StatePlayback::Once),
                AnimationState::new("loop", 1.0, 3.0, StatePlayback::Loop),
                AnimationState::new("out", 3.0, 4.0, StatePlayback::Once),
            ],
            assets: Vec::new(),
            lifecycle: Some(Lifecycle::default()),
        }
    }
}

impl Composition {
    /// Create a new composition with specified canvas size and framerate.
    pub fn new(width: u32, height: u32, fps: f32, duration_seconds: f64) -> Self {
        Self {
            width,
            height,
            frames_per_second: fps,
            duration_seconds,
            layers: Vec::new(),
            variables: Vec::new(),
            states: Vec::new(),
            assets: Vec::new(),
            lifecycle: None,
        }
    }

    /// Find layer by its LayerId.
    pub fn find_layer(&self, id: LayerId) -> Option<&Layer> {
        self.layers.iter().find(|l| l.id == id)
    }

    /// Find mutable layer by its LayerId.
    pub fn find_layer_mut(&mut self, id: LayerId) -> Option<&mut Layer> {
        self.layers.iter_mut().find(|l| l.id == id)
    }

    /// Find animation state by name.
    pub fn find_state(&self, name: &str) -> Option<&AnimationState> {
        self.states.iter().find(|s| s.name == name)
    }
}
