//! Transition definitions, kinds, rule resolution, and active transition state machine.

use serde::{Deserialize, Serialize};
use zeno_core::{AssetId, Color, CompositionId, Easing, SceneId, TransitionId};

/// Slide direction for slide transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SlideDirection {
    /// Slide from right to left.
    #[default]
    Left,
    /// Slide from left to right.
    Right,
    /// Slide from bottom to top.
    Up,
    /// Slide from top to bottom.
    Down,
}

/// Wipe direction for linear and radial wipe transitions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum WipeDirection {
    /// Left-to-right linear wipe.
    #[default]
    Horizontal,
    /// Top-to-bottom linear wipe.
    Vertical,
    /// Center-out radial wipe.
    Radial,
}

/// Audio transition behavior when scenes switch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TransitionAudio {
    /// Crossfade audio smoothly over the transition duration.
    #[default]
    Crossfade,
    /// Instant cut at the switch point.
    Cut,
    /// Audio level dynamically tracks the video blend / stinger visibility.
    FollowVideo,
}

/// Transition algorithm and visual effect parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub enum TransitionKind {
    /// Instant 0 ms scene switch.
    Cut,
    /// Smooth linear / eased cross-dissolve.
    #[default]
    Fade,
    /// Fade down to a solid color (e.g. black or brand color), then fade up to new scene.
    DipToColor {
        /// Intermediate color to dip into.
        color: Color,
    },
    /// Slide transition: push (both scenes move) or cover (incoming moves over outgoing).
    Slide {
        /// Motion direction.
        direction: SlideDirection,
        /// If `true`, outgoing scene is pushed off-screen; if `false`, incoming scene covers outgoing.
        push: bool,
    },
    /// Wipe transition with optional feather/soft-edge setting.
    Wipe {
        /// Wipe orientation.
        direction: WipeDirection,
        /// Feathered edge softness width [0.0..0.5].
        soft_edge: f32,
    },
    /// Zoom transition expanding or contracting between scenes.
    Zoom {
        /// If `true`, zooms into the new scene; if `false`, zooms out.
        zoom_in: bool,
    },
    /// Stinger video with alpha channel plus switch-point marker.
    Stinger {
        /// Asset ID of the stinger video with alpha.
        asset: Option<AssetId>,
        /// Switch point percentage [0.0..1.0] when program cuts to new scene under stinger cover.
        switch_point: f32,
    },
    /// Branded transition authored in Zenocast (.arx) with In, Hold, and Out lifecycle.
    Overlay {
        /// Composition ID of the transition template.
        composition_id: Option<CompositionId>,
        /// Switch point percentage [0.0..1.0].
        switch_point: f32,
    },
}

/// Complete configuration of a scene transition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransitionDefinition {
    /// Unique identifier for this transition definition.
    pub id: TransitionId,
    /// User-visible transition name (e.g. "Smooth Fade", "Branded Stinger", "Quick Cut").
    pub name: String,
    /// Visual kind and parameters.
    pub kind: TransitionKind,
    /// Total duration in seconds (0.0 to 5.0 s).
    pub duration_seconds: f64,
    /// Animation easing curve.
    pub easing: Easing,
    /// Switch point percentage [0.0..1.0] for stingers and overlays.
    pub switch_point_percent: f32,
    /// Audio transition behavior.
    pub audio_behavior: TransitionAudio,
}

impl TransitionDefinition {
    /// Creates an instant Cut transition.
    pub fn cut() -> Self {
        Self {
            id: TransitionId::new(),
            name: "Cut".to_string(),
            kind: TransitionKind::Cut,
            duration_seconds: 0.0,
            easing: Easing::Hold,
            switch_point_percent: 1.0,
            audio_behavior: TransitionAudio::Cut,
        }
    }

    /// Creates a standard 300 ms crossfade transition.
    pub fn fade(duration_seconds: f64) -> Self {
        Self {
            id: TransitionId::new(),
            name: "Fade".to_string(),
            kind: TransitionKind::Fade,
            duration_seconds: duration_seconds.clamp(0.0, 5.0),
            easing: Easing::Linear,
            switch_point_percent: 0.5,
            audio_behavior: TransitionAudio::Crossfade,
        }
    }

    /// Creates a slide transition.
    pub fn slide(direction: SlideDirection, push: bool, duration_seconds: f64) -> Self {
        Self {
            id: TransitionId::new(),
            name: format!("Slide {:?}", direction),
            kind: TransitionKind::Slide { direction, push },
            duration_seconds: duration_seconds.clamp(0.0, 5.0),
            easing: Easing::ease_out(),
            switch_point_percent: 0.5,
            audio_behavior: TransitionAudio::Crossfade,
        }
    }

    /// Creates a wipe transition with soft-edge feathering.
    pub fn wipe(direction: WipeDirection, soft_edge: f32, duration_seconds: f64) -> Self {
        Self {
            id: TransitionId::new(),
            name: format!("Wipe {:?}", direction),
            kind: TransitionKind::Wipe {
                direction,
                soft_edge: soft_edge.clamp(0.0, 0.5),
            },
            duration_seconds: duration_seconds.clamp(0.0, 5.0),
            easing: Easing::ease_in_out(),
            switch_point_percent: 0.5,
            audio_behavior: TransitionAudio::Crossfade,
        }
    }

    /// Creates a Dip to Color transition.
    pub fn dip_to_color(color: Color, duration_seconds: f64) -> Self {
        Self {
            id: TransitionId::new(),
            name: "Dip to Color".to_string(),
            kind: TransitionKind::DipToColor { color },
            duration_seconds: duration_seconds.clamp(0.0, 5.0),
            easing: Easing::ease_in_out(),
            switch_point_percent: 0.5,
            audio_behavior: TransitionAudio::Crossfade,
        }
    }

    /// Validates transition parameters (duration clamped 0..5, switch point clamped 0..1).
    pub fn validate(&self) -> Result<(), String> {
        if self.duration_seconds < 0.0 || self.duration_seconds > 5.0 {
            return Err("Transition duration must be between 0.0 and 5.0 seconds".to_string());
        }
        if self.switch_point_percent < 0.0 || self.switch_point_percent > 1.0 {
            return Err("Switch point percent must be between 0.0 and 1.0".to_string());
        }
        Ok(())
    }
}

impl Default for TransitionDefinition {
    fn default() -> Self {
        Self::fade(0.3)
    }
}

/// Transition rule matching source scene to destination scene.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransitionRule {
    /// Outgoing scene (`None` matches any source scene).
    pub from_scene: Option<SceneId>,
    /// Target destination scene.
    pub to_scene: SceneId,
    /// Transition definition to execute.
    pub transition: TransitionId,
}

/// Runtime state machine tracking an in-flight scene transition.
#[derive(Debug, Clone)]
pub struct ActiveTransition {
    /// Outgoing source scene ID.
    pub from_scene: SceneId,
    /// Incoming destination scene ID.
    pub to_scene: SceneId,
    /// Transition specification.
    pub definition: TransitionDefinition,
    /// Elapsed playback time in seconds.
    pub elapsed_seconds: f64,
    /// Manual T-bar override progress [0.0..1.0].
    pub manual_progress: Option<f32>,
    /// If `true`, the source frame is sampled from the captured blended snapshot rather than re-rendering `from_scene`.
    pub from_snapshot: bool,
}

impl ActiveTransition {
    /// Creates a new active transition between two scenes.
    pub fn new(from: SceneId, to: SceneId, definition: TransitionDefinition) -> Self {
        Self {
            from_scene: from,
            to_scene: to,
            definition,
            elapsed_seconds: 0.0,
            manual_progress: None,
            from_snapshot: false,
        }
    }

    /// Creates an active transition starting seamlessly from an in-flight blended snapshot.
    pub fn from_snapshot(from: SceneId, to: SceneId, definition: TransitionDefinition) -> Self {
        Self {
            from_scene: from,
            to_scene: to,
            definition,
            elapsed_seconds: 0.0,
            manual_progress: None,
            from_snapshot: true,
        }
    }

    /// Advances the transition clock by `dt` seconds.
    pub fn tick(&mut self, dt: f64) {
        if self.manual_progress.is_none() {
            self.elapsed_seconds += dt;
        }
    }

    /// Returns normalized linear progress [0.0..1.0].
    pub fn linear_progress(&self) -> f32 {
        if let Some(p) = self.manual_progress {
            p.clamp(0.0, 1.0)
        } else if self.definition.duration_seconds <= 0.0 {
            1.0
        } else {
            (self.elapsed_seconds / self.definition.duration_seconds).clamp(0.0, 1.0) as f32
        }
    }

    /// Returns eased progress [0.0..1.0] used for GPU blend calculations.
    pub fn eased_progress(&self) -> f32 {
        let linear = self.linear_progress();
        self.definition.easing.evaluate(linear)
    }

    /// Returns `true` when the transition has reached 100% completion.
    pub fn is_finished(&self) -> bool {
        self.linear_progress() >= 1.0
    }

    /// Returns `true` if the transition switch point has been passed.
    pub fn has_switched(&self) -> bool {
        self.linear_progress() >= self.definition.switch_point_percent
    }
}
