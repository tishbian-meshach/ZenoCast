//! Overlay and screen lifecycle phase state machine definitions.

use crate::TimeRange;
use serde::{Deserialize, Serialize};

/// Current active lifecycle phase of an overlay item or scene.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum LifecyclePhase {
    /// Completely hidden / off-air. Not rendered.
    #[default]
    Hidden,
    /// In animation playing (intro entrance).
    In,
    /// Loop animation playing (continuous on-air state).
    Loop,
    /// Out animation playing (outro exit).
    Out,
}

/// Behavior when an action interrupts an in-progress transition phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum InterruptBehavior {
    /// Interrupt immediately and transition to the target phase from the current visual state.
    #[default]
    Immediately,
    /// Allow the current phase animation to complete before initiating the next phase.
    FinishFirst,
}

/// Three-phase (In, Loop, Out) animation authoring and runtime lifecycle settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lifecycle {
    /// Timeline range for the entrance animation.
    pub in_range: TimeRange,
    /// Timeline range for the looping animation.
    pub loop_range: TimeRange,
    /// Timeline range for the exit animation.
    pub out_range: TimeRange,
    /// Loop repeat count. `None` indicates infinite looping while on air.
    pub loop_count: Option<u32>,
    /// How to handle a hide request occurring while the In animation is still playing.
    pub hide_during_in: InterruptBehavior,
    /// How to handle a show request occurring while the Out animation is still playing.
    pub show_during_out: InterruptBehavior,
}

impl Default for Lifecycle {
    fn default() -> Self {
        Self {
            in_range: TimeRange::new(0.0, 1.0),
            loop_range: TimeRange::new(1.0, 3.0),
            out_range: TimeRange::new(3.0, 4.0),
            loop_count: None,
            hide_during_in: InterruptBehavior::Immediately,
            show_during_out: InterruptBehavior::Immediately,
        }
    }
}

impl Lifecycle {
    /// Total duration covering all three phases sequentially.
    pub fn total_sequence_duration(&self) -> f64 {
        self.in_range.duration() + self.loop_range.duration() + self.out_range.duration()
    }
}
