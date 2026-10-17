//! In/Loop/Out lifecycle controller for overlays and screens.
//!
//! Provides independent three-phase lifecycle execution (In, Loop, Out, Hidden),
//! interrupt handling with smooth blending (no 1-frame pop), ping-pong repeat,
//! and multi-overlay manager.

use uuid::Uuid;
use zeno_core::{InterruptBehavior, Lifecycle, LifecyclePhase};

/// Runtime controller governing the lifecycle of a single overlay or screen item.
#[derive(Debug, Clone)]
pub struct LifecycleController {
    /// Unique controller identifier.
    pub id: Uuid,
    /// Human-readable label (e.g. "Lower Third", "Scoreboard", "Sponsor Bug").
    pub name: String,
    /// Configuration specifying timeline ranges and interrupt behavior.
    pub lifecycle: Lifecycle,
    /// Current active playback phase.
    pub phase: LifecyclePhase,
    /// Current timeline evaluation time in seconds.
    pub current_time: f64,
    /// Elapsed seconds inside the current phase.
    pub phase_elapsed: f64,
    /// Number of full loop repetitions completed.
    pub loops_completed: u32,
    /// Whether looping oscillates back-and-forth instead of wrapping.
    pub ping_pong: bool,
    /// Current direction in ping-pong mode (`true` = backwards).
    pub ping_pong_reverse: bool,
    /// Whether item is currently visible / active.
    pub visible: bool,
    /// Interrupt blend origin: starting alpha when an interrupt occurs to prevent 1-frame popping.
    pub interrupt_blend_start_alpha: f32,
}

impl Default for LifecycleController {
    fn default() -> Self {
        Self::new(Uuid::new_v4(), "Overlay", Lifecycle::default())
    }
}

impl LifecycleController {
    /// Creates a new lifecycle controller initialized in `Hidden` phase.
    pub fn new(id: Uuid, name: impl Into<String>, lifecycle: Lifecycle) -> Self {
        Self {
            id,
            name: name.into(),
            lifecycle,
            phase: LifecyclePhase::Hidden,
            current_time: 0.0,
            phase_elapsed: 0.0,
            loops_completed: 0,
            ping_pong: false,
            ping_pong_reverse: false,
            visible: false,
            interrupt_blend_start_alpha: 0.0,
        }
    }

    /// Triggers the intro entrance ("In") animation.
    pub fn play_in(&mut self) {
        match self.phase {
            LifecyclePhase::Hidden => {
                self.phase = LifecyclePhase::In;
                self.current_time = self.lifecycle.in_range.start_seconds;
                self.phase_elapsed = 0.0;
                self.visible = true;
                self.interrupt_blend_start_alpha = 0.0;
            }
            LifecyclePhase::Out => {
                if self.lifecycle.show_during_out == InterruptBehavior::Immediately {
                    let cur_alpha = self.effective_alpha();
                    self.phase = LifecyclePhase::In;
                    self.phase_elapsed = 0.0;
                    self.interrupt_blend_start_alpha = cur_alpha;
                    self.current_time = self.lifecycle.in_range.start_seconds;
                    self.visible = true;
                }
            }
            LifecyclePhase::In | LifecyclePhase::Loop => {}
        }
    }

    /// Triggers the exit transition ("Out") animation.
    pub fn play_out(&mut self) {
        match self.phase {
            LifecyclePhase::In => {
                if self.lifecycle.hide_during_in == InterruptBehavior::Immediately {
                    let cur_alpha = self.effective_alpha();
                    self.phase = LifecyclePhase::Out;
                    self.phase_elapsed = 0.0;
                    self.interrupt_blend_start_alpha = cur_alpha;
                    self.current_time = self.lifecycle.out_range.start_seconds;
                }
            }
            LifecyclePhase::Loop => {
                self.phase = LifecyclePhase::Out;
                self.phase_elapsed = 0.0;
                self.interrupt_blend_start_alpha = 1.0;
                self.current_time = self.lifecycle.out_range.start_seconds;
            }
            LifecyclePhase::Hidden | LifecyclePhase::Out => {}
        }
    }

    /// Toggles between In and Out.
    pub fn toggle(&mut self) {
        match self.phase {
            LifecyclePhase::Hidden | LifecyclePhase::Out => self.play_in(),
            LifecyclePhase::In | LifecyclePhase::Loop => self.play_out(),
        }
    }

    /// Advances the controller playback clock by `delta_seconds`.
    pub fn step(&mut self, delta_seconds: f64) {
        if delta_seconds <= 0.0 || self.phase == LifecyclePhase::Hidden {
            return;
        }

        self.phase_elapsed += delta_seconds;

        match self.phase {
            LifecyclePhase::In => {
                let in_dur = self.lifecycle.in_range.duration().max(0.001);
                self.current_time += delta_seconds;

                if self.phase_elapsed >= in_dur {
                    // Transition to Loop phase
                    self.phase = LifecyclePhase::Loop;
                    self.phase_elapsed = 0.0;
                    self.current_time = self.lifecycle.loop_range.start_seconds;
                    self.interrupt_blend_start_alpha = 1.0;
                }
            }
            LifecyclePhase::Loop => {
                let loop_dur = self.lifecycle.loop_range.duration().max(0.001);

                if self.ping_pong {
                    if self.ping_pong_reverse {
                        self.current_time -= delta_seconds;
                        if self.current_time <= self.lifecycle.loop_range.start_seconds {
                            self.current_time = self.lifecycle.loop_range.start_seconds;
                            self.ping_pong_reverse = false;
                            self.loops_completed += 1;
                        }
                    } else {
                        self.current_time += delta_seconds;
                        if self.current_time >= self.lifecycle.loop_range.end_seconds {
                            self.current_time = self.lifecycle.loop_range.end_seconds;
                            self.ping_pong_reverse = true;
                        }
                    }
                } else {
                    self.current_time += delta_seconds;
                    if self.current_time >= self.lifecycle.loop_range.end_seconds {
                        let span = loop_dur;
                        let excess = self.current_time - self.lifecycle.loop_range.end_seconds;
                        self.current_time = self.lifecycle.loop_range.start_seconds + excess % span;
                        self.loops_completed += 1;
                    }
                }

                // Check finite loop limit
                if let Some(limit) = self.lifecycle.loop_count {
                    if self.loops_completed >= limit {
                        self.play_out();
                    }
                }
            }
            LifecyclePhase::Out => {
                let out_dur = self.lifecycle.out_range.duration().max(0.001);
                self.current_time += delta_seconds;

                if self.phase_elapsed >= out_dur {
                    // Cut cleanly to hidden
                    self.phase = LifecyclePhase::Hidden;
                    self.visible = false;
                    self.phase_elapsed = 0.0;
                    self.current_time = 0.0;
                    self.interrupt_blend_start_alpha = 0.0;
                }
            }
            LifecyclePhase::Hidden => {}
        }
    }

    /// Evaluates the normalized effective opacity (0.0 to 1.0).
    /// Prevents 1-frame pops when interrupted mid-transition.
    pub fn effective_alpha(&self) -> f32 {
        match self.phase {
            LifecyclePhase::Hidden => 0.0,
            LifecyclePhase::In => {
                let in_dur = self.lifecycle.in_range.duration().max(0.001);
                let progress = (self.phase_elapsed / in_dur).clamp(0.0, 1.0) as f32;
                self.interrupt_blend_start_alpha
                    + (1.0 - self.interrupt_blend_start_alpha) * progress
            }
            LifecyclePhase::Loop => 1.0,
            LifecyclePhase::Out => {
                let out_dur = self.lifecycle.out_range.duration().max(0.001);
                let progress = (self.phase_elapsed / out_dur).clamp(0.0, 1.0) as f32;
                self.interrupt_blend_start_alpha * (1.0 - progress)
            }
        }
    }
}

/// Manager maintaining multiple independent overlay and screen lifecycle controllers.
#[derive(Debug, Default)]
pub struct LifecycleManager {
    controllers: Vec<LifecycleController>,
}

impl LifecycleManager {
    /// Creates a new empty manager.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new lifecycle controller.
    pub fn register(&mut self, controller: LifecycleController) {
        self.controllers.push(controller);
    }

    /// Retrieves an immutable slice of all active controllers.
    pub fn controllers(&self) -> &[LifecycleController] {
        &self.controllers
    }

    /// Retrieves a mutable slice of all active controllers.
    pub fn controllers_mut(&mut self) -> &mut [LifecycleController] {
        &mut self.controllers
    }

    /// Finds a controller by its unique ID.
    pub fn find(&self, id: Uuid) -> Option<&LifecycleController> {
        self.controllers.iter().find(|c| c.id == id)
    }

    /// Finds a mutable controller by its unique ID.
    pub fn find_mut(&mut self, id: Uuid) -> Option<&mut LifecycleController> {
        self.controllers.iter_mut().find(|c| c.id == id)
    }

    /// Advances all registered overlay controllers simultaneously.
    pub fn step_all(&mut self, delta_seconds: f64) {
        for c in &mut self.controllers {
            c.step(delta_seconds);
        }
    }

    /// Fires In on all registered overlays.
    pub fn play_in_all(&mut self) {
        for c in &mut self.controllers {
            c.play_in();
        }
    }

    /// Fires Out on all registered overlays.
    pub fn play_out_all(&mut self) {
        for c in &mut self.controllers {
            c.play_out();
        }
    }
}
