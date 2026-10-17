//! Scrubbable and looping playback clock governing composition playback and animation states.

use crate::model::{AnimationState, StatePlayback};

/// Current state of the playback clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackState {
    /// Playhead is paused.
    #[default]
    Paused,
    /// Playhead is actively advancing forward in time.
    Playing,
}

/// Playback clock managing timeline scrubbing, looping, and state transitions.
#[derive(Debug, Clone)]
pub struct PlaybackClock {
    /// Current playhead position in seconds.
    pub current_time: f64,
    /// Total duration of the timeline in seconds.
    pub duration_seconds: f64,
    /// Target frame rate (e.g. 60.0 fps).
    pub fps: f32,
    /// Active playback state (Playing or Paused).
    pub state: PlaybackState,
    /// Active loop range in seconds [start_seconds, end_seconds].
    pub loop_range: Option<(f64, f64)>,
    /// Whether playback wraps seamlessly upon reaching the boundary.
    pub is_looping: bool,
    /// Name of the active animation state, if bound to one (e.g. "in", "loop", "out").
    pub active_state_name: Option<String>,
}

impl PlaybackClock {
    /// Initialize a new playback clock with specified duration and frame rate.
    pub fn new(duration_seconds: f64, fps: f32) -> Self {
        Self {
            current_time: 0.0,
            duration_seconds: duration_seconds.max(0.0),
            fps: fps.max(1.0),
            state: PlaybackState::Paused,
            loop_range: None,
            is_looping: true,
            active_state_name: None,
        }
    }

    /// Starts or resumes playback.
    pub fn play(&mut self) {
        self.state = PlaybackState::Playing;
    }

    /// Pauses playback.
    pub fn pause(&mut self) {
        self.state = PlaybackState::Paused;
    }

    /// Stops playback and resets playhead to start.
    pub fn stop(&mut self) {
        self.pause();
        self.reset();
    }

    /// Scrubs or seeks the playhead to an exact timestamp.
    #[inline]
    pub fn seek(&mut self, time_seconds: f64) {
        self.set_time(time_seconds);
    }

    /// Toggles between playing and paused states.
    pub fn toggle_playback(&mut self) {
        self.state = match self.state {
            PlaybackState::Playing => PlaybackState::Paused,
            PlaybackState::Paused => PlaybackState::Playing,
        };
    }

    /// Returns `true` if the playhead is currently advancing.
    #[inline]
    pub fn is_playing(&self) -> bool {
        self.state == PlaybackState::Playing
    }

    /// Current time in seconds.
    #[inline]
    pub fn time(&self) -> f64 {
        self.current_time
    }

    /// Scrubs the playhead to an exact timestamp, clamping or wrapping according to active boundaries.
    pub fn set_time(&mut self, time_seconds: f64) {
        let (start, end) = self.active_bounds();
        let span = end - start;
        if span <= 0.0 {
            self.current_time = start;
        } else if self.is_looping {
            let offset = (time_seconds - start).rem_euclid(span);
            self.current_time = start + offset;
        } else {
            self.current_time = time_seconds.clamp(start, end);
        }
    }

    /// Resets the playhead to the beginning of the active boundary.
    pub fn reset(&mut self) {
        let (start, _) = self.active_bounds();
        self.current_time = start;
    }

    /// Sets an explicit loop boundary range [start, end].
    pub fn set_loop_range(&mut self, start_seconds: f64, end_seconds: f64) {
        let start = start_seconds.clamp(0.0, self.duration_seconds);
        let end = end_seconds.clamp(start, self.duration_seconds);
        self.loop_range = Some((start, end));
        self.is_looping = true;
    }

    /// Clears the explicit loop boundary range, defaulting to full composition duration.
    pub fn clear_loop_range(&mut self) {
        self.loop_range = None;
    }

    /// Binds the clock to an AnimationState definition.
    pub fn bind_animation_state(&mut self, state: &AnimationState) {
        self.active_state_name = Some(state.name.clone());
        self.loop_range = Some((state.start_seconds, state.end_seconds));
        self.current_time = state.start_seconds;
        match state.playback {
            StatePlayback::Loop => {
                self.is_looping = true;
                self.state = PlaybackState::Playing;
            }
            StatePlayback::Once => {
                self.is_looping = false;
                self.state = PlaybackState::Playing;
            }
            StatePlayback::HoldAtEnd => {
                self.is_looping = false;
                self.state = PlaybackState::Playing;
            }
        }
    }

    /// Advances the clock by a time step in seconds if playing.
    pub fn step(&mut self, delta_seconds: f64) {
        if self.state != PlaybackState::Playing || delta_seconds <= 0.0 {
            return;
        }

        let (start, end) = self.active_bounds();
        let span = end - start;

        if span <= 0.0 {
            self.current_time = start;
            self.state = PlaybackState::Paused;
            return;
        }

        let next_time = self.current_time + delta_seconds;
        if next_time >= end {
            if self.is_looping {
                let overflow = next_time - end;
                self.current_time = start + (overflow % span);
            } else {
                self.current_time = end;
                self.state = PlaybackState::Paused;
            }
        } else {
            self.current_time = next_time;
        }
    }

    /// Steps forward by exactly one frame (1 / fps).
    pub fn step_frame(&mut self) {
        let frame_dt = 1.0 / (self.fps as f64);
        let was_playing = self.state == PlaybackState::Playing;
        self.state = PlaybackState::Playing;
        self.step(frame_dt);
        if !was_playing {
            self.state = PlaybackState::Paused;
        }
    }

    /// Steps backward by exactly one frame (1 / fps).
    pub fn step_back_frame(&mut self) {
        let frame_dt = 1.0 / (self.fps as f64);
        let (start, end) = self.active_bounds();
        let span = end - start;
        if span <= 0.0 {
            self.current_time = start;
            return;
        }

        let prev_time = self.current_time - frame_dt;
        if prev_time < start {
            if self.is_looping {
                let underflow = start - prev_time;
                self.current_time = end - (underflow % span);
            } else {
                self.current_time = start;
            }
        } else {
            self.current_time = prev_time;
        }
    }

    #[inline]
    fn active_bounds(&self) -> (f64, f64) {
        self.loop_range.unwrap_or((0.0, self.duration_seconds))
    }
}
