//! Clean-room filter system data model for video effect and audio DSP processing.

use crate::id::FilterId;
use serde::{Deserialize, Serialize};

/// High-level categorization of a processing filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilterCategory {
    /// 2D/3D visual post-processing shaders executed on GPU.
    Video,
    /// Real-time 48 kHz stereo DSP audio processors.
    Audio,
}

/// Concrete kind of processing filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FilterKind {
    // Video effect filters
    ColorCorrection,
    ChromaKey,
    Blur,
    Sharpen,
    CropPad,
    Mask,

    // Audio DSP filters
    Gain,
    NoiseGate,
    Compressor,
    Limiter,
    Eq,
    NoiseSuppression,
}

impl FilterKind {
    /// Returns the category (Video or Audio) for this filter kind.
    pub fn category(&self) -> FilterCategory {
        match self {
            Self::ColorCorrection
            | Self::ChromaKey
            | Self::Blur
            | Self::Sharpen
            | Self::CropPad
            | Self::Mask => FilterCategory::Video,

            Self::Gain
            | Self::NoiseGate
            | Self::Compressor
            | Self::Limiter
            | Self::Eq
            | Self::NoiseSuppression => FilterCategory::Audio,
        }
    }

    /// Returns the standard default user-visible name.
    pub fn default_name(&self) -> &'static str {
        match self {
            Self::ColorCorrection => "Color Correction",
            Self::ChromaKey => "Chroma Key",
            Self::Blur => "Blur",
            Self::Sharpen => "Sharpen",
            Self::CropPad => "Crop/Pad",
            Self::Mask => "Image Mask/Blend",
            Self::Gain => "Gain",
            Self::NoiseGate => "Noise Gate",
            Self::Compressor => "Compressor",
            Self::Limiter => "Limiter",
            Self::Eq => "3-Band Equalizer",
            Self::NoiseSuppression => "Noise Suppression",
        }
    }

    /// Generates default settings for this filter kind.
    pub fn default_settings(&self) -> FilterSettings {
        match self {
            Self::ColorCorrection => FilterSettings::ColorCorrection {
                brightness: 0.0,
                contrast: 1.0,
                saturation: 1.0,
                hue_shift: 0.0,
                gamma: 1.0,
                opacity: 1.0,
            },
            Self::ChromaKey => FilterSettings::ChromaKey {
                key_color: [0.0, 1.0, 0.0, 1.0], // green
                similarity: 0.4,
                smoothness: 0.08,
                spill_reduction: 0.1,
            },
            Self::Blur => FilterSettings::Blur { radius: 4.0 },
            Self::Sharpen => FilterSettings::Sharpen { sharpness: 0.2 },
            Self::CropPad => FilterSettings::CropPad {
                left: 0.0,
                top: 0.0,
                right: 0.0,
                bottom: 0.0,
            },
            Self::Mask => FilterSettings::Mask {
                path: String::new(),
                invert: false,
            },
            Self::Gain => FilterSettings::Gain { gain_db: 0.0 },
            Self::NoiseGate => FilterSettings::NoiseGate {
                threshold_db: -40.0,
                attack_ms: 2.0,
                hold_ms: 50.0,
                release_ms: 150.0,
                reduction_db: -60.0,
            },
            Self::Compressor => FilterSettings::Compressor {
                threshold_db: -18.0,
                ratio: 4.0,
                attack_ms: 6.0,
                release_ms: 60.0,
                makeup_gain_db: 0.0,
            },
            Self::Limiter => FilterSettings::Limiter {
                threshold_db: -1.0,
                release_ms: 60.0,
            },
            Self::Eq => FilterSettings::Eq {
                low_gain_db: 0.0,
                mid_gain_db: 0.0,
                high_gain_db: 0.0,
            },
            Self::NoiseSuppression => FilterSettings::NoiseSuppression {
                suppress_level_db: -30.0,
            },
        }
    }
}

/// Typed, serde-tagged filter configuration settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum FilterSettings {
    ColorCorrection {
        #[serde(default)]
        brightness: f32, // [-1.0..1.0]
        #[serde(default = "default_one")]
        contrast: f32, // [0.0..2.0]
        #[serde(default = "default_one")]
        saturation: f32, // [0.0..2.0]
        #[serde(default)]
        hue_shift: f32, // [-180.0..180.0]
        #[serde(default = "default_one")]
        gamma: f32, // [0.1..3.0]
        #[serde(default = "default_one")]
        opacity: f32, // [0.0..1.0]
    },
    ChromaKey {
        #[serde(default = "default_green")]
        key_color: [f32; 4],
        #[serde(default = "default_chroma_similarity")]
        similarity: f32, // [0.0..1.0]
        #[serde(default = "default_chroma_smoothness")]
        smoothness: f32, // [0.0..1.0]
        #[serde(default = "default_chroma_spill")]
        spill_reduction: f32, // [0.0..1.0]
    },
    Blur {
        #[serde(default = "default_blur_radius")]
        radius: f32, // [1.0..64.0]
    },
    Sharpen {
        #[serde(default = "default_sharpness")]
        sharpness: f32, // [0.0..1.0]
    },
    CropPad {
        #[serde(default)]
        left: f32,
        #[serde(default)]
        top: f32,
        #[serde(default)]
        right: f32,
        #[serde(default)]
        bottom: f32,
    },
    Mask {
        #[serde(default)]
        path: String,
        #[serde(default)]
        invert: bool,
    },
    Gain {
        #[serde(default)]
        gain_db: f32,
    },
    NoiseGate {
        #[serde(default = "default_gate_threshold")]
        threshold_db: f32,
        #[serde(default = "default_gate_attack")]
        attack_ms: f32,
        #[serde(default = "default_gate_hold")]
        hold_ms: f32,
        #[serde(default = "default_gate_release")]
        release_ms: f32,
        #[serde(default = "default_gate_reduction")]
        reduction_db: f32,
    },
    Compressor {
        #[serde(default = "default_comp_threshold")]
        threshold_db: f32,
        #[serde(default = "default_comp_ratio")]
        ratio: f32,
        #[serde(default = "default_comp_attack")]
        attack_ms: f32,
        #[serde(default = "default_comp_release")]
        release_ms: f32,
        #[serde(default)]
        makeup_gain_db: f32,
    },
    Limiter {
        #[serde(default = "default_limiter_threshold")]
        threshold_db: f32,
        #[serde(default = "default_limiter_release")]
        release_ms: f32,
    },
    Eq {
        #[serde(default)]
        low_gain_db: f32,
        #[serde(default)]
        mid_gain_db: f32,
        #[serde(default)]
        high_gain_db: f32,
    },
    NoiseSuppression {
        #[serde(default = "default_suppress_db")]
        suppress_level_db: f32,
    },
}

fn default_one() -> f32 {
    1.0
}
fn default_green() -> [f32; 4] {
    [0.0, 1.0, 0.0, 1.0]
}
fn default_chroma_similarity() -> f32 {
    0.4
}
fn default_chroma_smoothness() -> f32 {
    0.08
}
fn default_chroma_spill() -> f32 {
    0.1
}
fn default_blur_radius() -> f32 {
    4.0
}
fn default_sharpness() -> f32 {
    0.2
}
fn default_gate_threshold() -> f32 {
    -40.0
}
fn default_gate_attack() -> f32 {
    2.0
}
fn default_gate_hold() -> f32 {
    50.0
}
fn default_gate_release() -> f32 {
    150.0
}
fn default_gate_reduction() -> f32 {
    -60.0
}
fn default_comp_threshold() -> f32 {
    -18.0
}
fn default_comp_ratio() -> f32 {
    4.0
}
fn default_comp_attack() -> f32 {
    6.0
}
fn default_comp_release() -> f32 {
    60.0
}
fn default_limiter_threshold() -> f32 {
    -1.0
}
fn default_limiter_release() -> f32 {
    60.0
}
fn default_suppress_db() -> f32 {
    -30.0
}

/// A single discrete processing filter in a filter chain.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FilterDefinition {
    /// Unique filter instance ID.
    pub id: FilterId,
    /// Concrete filter kind.
    pub kind: FilterKind,
    /// User-editable display name.
    pub name: String,
    /// Filter active / bypass toggle.
    pub enabled: bool,
    /// Typed per-kind settings payload.
    pub settings: FilterSettings,
}

impl FilterDefinition {
    /// Constructs a new enabled filter with default settings and standard name.
    pub fn new(kind: FilterKind) -> Self {
        Self {
            id: FilterId::new(),
            kind,
            name: kind.default_name().to_string(),
            enabled: true,
            settings: kind.default_settings(),
        }
    }
}

/// An ordered sequential chain of filters attached to a source or scene item.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FilterChain {
    pub filters: Vec<FilterDefinition>,
}

impl FilterChain {
    /// Creates an empty filter chain.
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
        }
    }

    /// Appends a new filter to the end of the chain.
    pub fn add(&mut self, filter: FilterDefinition) {
        self.filters.push(filter);
    }

    /// Removes a filter by its ID, returning it if found.
    pub fn remove(&mut self, id: FilterId) -> Option<FilterDefinition> {
        if let Some(pos) = self.filters.iter().position(|f| f.id == id) {
            Some(self.filters.remove(pos))
        } else {
            None
        }
    }

    /// Moves a filter one position earlier in the execution order.
    pub fn move_up(&mut self, index: usize) {
        if index > 0 && index < self.filters.len() {
            self.filters.swap(index, index - 1);
        }
    }

    /// Moves a filter one position later in the execution order.
    pub fn move_down(&mut self, index: usize) {
        if index + 1 < self.filters.len() {
            self.filters.swap(index, index + 1);
        }
    }

    /// Toggles the enabled state of a filter by its ID.
    pub fn toggle_enabled(&mut self, id: FilterId) -> bool {
        if let Some(f) = self.filters.iter_mut().find(|f| f.id == id) {
            f.enabled = !f.enabled;
            f.enabled
        } else {
            false
        }
    }

    /// Returns true if the chain contains any filters.
    pub fn is_empty(&self) -> bool {
        self.filters.is_empty()
    }

    /// Returns the number of filters in the chain.
    pub fn len(&self) -> usize {
        self.filters.len()
    }
}
