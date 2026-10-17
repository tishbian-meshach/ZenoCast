//! Fully evaluated frame and layer structures consumed by renderers and export pipelines.

use crate::model::{BlendMode, GradientStop, Layer, TextAlignment};
use zeno_core::{AssetId, Color, LayerId, Mat4, Vec2, Vec3};

/// Evaluated shape fill with concrete color or gradient stops.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedFill {
    /// Solid single color.
    Solid(Color),
    /// Linear two-point color gradient.
    LinearGradient {
        /// Normalized gradient start coordinates [0.0..1.0].
        start: Vec2,
        /// Normalized gradient end coordinates [0.0..1.0].
        end: Vec2,
        /// Ordered gradient color stops.
        stops: Vec<GradientStop>,
    },
    /// Radial circular gradient.
    RadialGradient {
        /// Normalized center coordinates [0.0..1.0].
        center: Vec2,
        /// Normalized radius.
        radius: f32,
        /// Ordered gradient color stops.
        stops: Vec<GradientStop>,
    },
}

/// Evaluated stroke with concrete width and color.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedStroke {
    /// Stroke color.
    pub color: Color,
    /// Stroke line width in canvas pixels.
    pub width: f32,
}

/// Evaluated vector shape geometry with concrete pixel dimensions.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedShapeGeometry {
    /// Axis-aligned rectangle with concrete width and height.
    Rectangle {
        /// Pixel dimensions (width, height).
        size: Vec2,
    },
    /// Rounded rectangle with corner radius.
    RoundedRectangle {
        /// Pixel dimensions (width, height).
        size: Vec2,
        /// Corner radius in pixels.
        corner_radius: f32,
    },
    /// Ellipse with horizontal and vertical radii.
    Ellipse {
        /// Radii (rx, ry).
        radius: Vec2,
    },
    /// SVG path string.
    Path {
        /// Raw SVG path command data.
        svg_data: String,
    },
}

/// Fully evaluated vector or shape content.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedShape {
    /// Evaluated geometric primitive.
    pub geometry: ResolvedShapeGeometry,
    /// Evaluated interior fill.
    pub fill: Option<ResolvedFill>,
    /// Evaluated boundary stroke.
    pub stroke: Option<ResolvedStroke>,
}

/// Fully evaluated text element with concrete characters, layout, and appearance.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedText {
    /// Text string content.
    pub text: String,
    /// Font family name (e.g. "Inter", "JetBrains Mono").
    pub font_family: String,
    /// Font size in pixels.
    pub font_size: f32,
    /// Numerical font weight (e.g. 400 for Normal, 700 for Bold).
    pub font_weight: u16,
    /// Text color.
    pub color: Color,
    /// Line height multiplier (1.0 = normal).
    pub line_height_multiplier: f32,
    /// Letter spacing adjustment in pixels.
    pub letter_spacing: f32,
    /// Horizontal alignment.
    pub alignment: TextAlignment,
}

/// Simulated particle instance for GPU rendering and CPU verification.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ParticleInstance {
    /// 2D position in emitter coordinate space.
    pub position: [f32; 2],
    /// Current particle size (diameter in pixels).
    pub size: f32,
    /// Normalized age in [0.0..1.0].
    pub age_normalized: f32,
    /// RGBA color channels [r, g, b, a].
    pub color: [f32; 4],
}

/// Evaluated content payload for a layer.
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedContent {
    /// Empty container / null object.
    Null,
    /// Text element.
    Text(ResolvedText),
    /// Bitmap image texture.
    Image {
        /// Asset reference ID.
        asset: AssetId,
    },
    /// Video playback layer.
    Video {
        /// Asset reference ID.
        asset: AssetId,
        /// Whether playback loops seamlessly.
        loop_playback: bool,
        /// Playback speed multiplier.
        speed: f32,
    },
    /// Geometric shape.
    Shape(ResolvedShape),
    /// Group of child layers.
    Group {
        /// Identifiers of child layers.
        children: Vec<LayerId>,
    },
    /// GPU particle emitter.
    Particles {
        /// Active simulated particle instances.
        particles: Vec<ParticleInstance>,
        /// Whether additive blending is enabled.
        additive_blend: bool,
        /// Optional sprite texture asset.
        sprite_asset: Option<AssetId>,
    },
    /// 3D model asset with resolved PBR parameters.
    Model3D {
        /// Asset reference ID.
        asset: AssetId,
        /// Cumulative rotation in degrees [x, y, z].
        rotation_degrees: Vec3,
        /// Metallic factor.
        metallic: f32,
        /// Roughness factor.
        roughness: f32,
        /// Base color.
        base_color: [f32; 4],
        /// Directional light vector.
        light_direction: [f32; 4],
        /// Directional light color.
        light_color: [f32; 4],
        /// Ambient light color.
        ambient_color: [f32; 4],
    },
    /// Scene perspective or orthographic camera.
    Camera,
    /// Data repeater instancing template group once per table/spreadsheet row.
    Repeater {
        /// Template group layer to clone.
        template_group: LayerId,
        /// Variable data path referencing the array/table.
        data_path: String,
        /// Maximum row instances.
        max_rows: u32,
        /// Per-row pixel offset.
        row_offset: Vec2,
    },
}

/// Fully evaluated layer ready for rendering.
#[derive(Debug, Clone)]
pub struct ResolvedLayer {
    /// Unique layer identifier.
    pub id: LayerId,
    /// User-assigned layer name.
    pub name: String,
    /// Identifier of parent layer, if parented.
    pub parent: Option<LayerId>,
    /// Evaluated 4x4 world transformation matrix.
    pub world_matrix: Mat4,
    /// Net resolved opacity (0.0 to 1.0) incorporating parent opacities.
    pub net_opacity: f32,
    /// Blending mode for compositor passes.
    pub blend_mode: BlendMode,
    /// 3D layer flag.
    pub is_3d: bool,
    /// Visibility flag.
    pub visible: bool,
    /// Strongly-typed evaluated content.
    pub content: ResolvedContent,
    /// Underlying layer model definition.
    pub layer: Layer,
}

/// A pure evaluated frame representing an entire composition at a discrete timestamp.
#[derive(Debug, Clone)]
pub struct ResolvedFrame {
    /// Timestamp of this frame in seconds.
    pub time_seconds: f64,
    /// Composition canvas width in pixels.
    pub width: u32,
    /// Composition canvas height in pixels.
    pub height: u32,
    /// Ordered list of active, visible layers ready for drawing.
    pub layers: Vec<ResolvedLayer>,
    /// Evaluated view-projection matrix (perspective if Camera layer exists, else orthographic).
    pub view_proj_matrix: Mat4,
    /// Whether this frame was evaluated with a 3D perspective camera.
    pub is_perspective: bool,
    /// Evaluated camera view matrix if a camera is active.
    pub camera_view: Option<Mat4>,
}
