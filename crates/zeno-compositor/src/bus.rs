//! Broadcast bus architecture: Preview bus, Program bus, Studio mode, and T-bar mixer.

use serde::{Deserialize, Serialize};
use zeno_core::SceneId;

/// Operating mode of the broadcast switcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum StudioMode {
    /// Direct mode: scene selections go straight to Program with the resolved transition.
    #[default]
    Direct,
    /// Studio mode: separate Preview and Program buses with Cut, Auto, and T-bar fader.
    Studio,
}

/// Bus state manager tracking active Program and Preview scenes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BusManager {
    /// Active scene currently broadcasting on the Program bus (live on air).
    pub program_scene: SceneId,
    /// Staged scene currently cued on the Preview bus.
    pub preview_scene: SceneId,
    /// Switcher operating mode.
    pub mode: StudioMode,
    /// Manual T-bar fader position [0.0..1.0].
    pub t_bar_position: f32,
}

impl BusManager {
    /// Creates a new bus manager initialized with a starting scene.
    pub fn new(initial_scene: SceneId) -> Self {
        Self {
            program_scene: initial_scene,
            preview_scene: initial_scene,
            mode: StudioMode::Studio,
            t_bar_position: 0.0,
        }
    }

    /// Sets the active Program scene.
    pub fn set_program(&mut self, scene: SceneId) {
        self.program_scene = scene;
    }

    /// Sets the staged Preview scene.
    pub fn set_preview(&mut self, scene: SceneId) {
        self.preview_scene = scene;
    }

    /// Swaps Preview and Program buses (called upon completion of Auto or Cut).
    pub fn swap_buses(&mut self) {
        std::mem::swap(&mut self.program_scene, &mut self.preview_scene);
        self.t_bar_position = 0.0;
    }

    /// Sets switcher mode.
    pub fn set_mode(&mut self, mode: StudioMode) {
        self.mode = mode;
    }

    /// Updates T-bar position [0.0..1.0].
    pub fn set_t_bar(&mut self, position: f32) {
        self.t_bar_position = position.clamp(0.0, 1.0);
    }
}
