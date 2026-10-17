//! Time representations and continuous interval ranges.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Continuous time interval specified in seconds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TimeRange {
    /// Range start point in seconds (inclusive).
    pub start_seconds: f64,
    /// Range end point in seconds (inclusive).
    pub end_seconds: f64,
}

impl TimeRange {
    /// Create a new time range.
    #[inline]
    pub const fn new(start_seconds: f64, end_seconds: f64) -> Self {
        Self {
            start_seconds,
            end_seconds,
        }
    }

    /// Duration of the range in seconds.
    #[inline]
    pub fn duration(&self) -> f64 {
        (self.end_seconds - self.start_seconds).max(0.0)
    }

    /// Check if a given time is within this range.
    #[inline]
    pub fn contains(&self, time_seconds: f64) -> bool {
        time_seconds >= self.start_seconds && time_seconds <= self.end_seconds
    }

    /// Clamp a time value to this range.
    #[inline]
    pub fn clamp(&self, time_seconds: f64) -> f64 {
        time_seconds.clamp(self.start_seconds, self.end_seconds)
    }

    /// Normalized progress within the range [0.0, 1.0].
    #[inline]
    pub fn progress(&self, time_seconds: f64) -> f64 {
        let dur = self.duration();
        if dur <= f64::EPSILON {
            0.0
        } else {
            ((time_seconds - self.start_seconds) / dur).clamp(0.0, 1.0)
        }
    }
}

impl Default for TimeRange {
    fn default() -> Self {
        Self {
            start_seconds: 0.0,
            end_seconds: 1.0,
        }
    }
}

/// SMPTE-style timecode representation for broadcast frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timecode {
    /// Hours component.
    pub hours: u8,
    /// Minutes component (0..=59).
    pub minutes: u8,
    /// Seconds component (0..=59).
    pub seconds: u8,
    /// Frames component (0..fps).
    pub frames: u8,
}

impl Timecode {
    /// Construct timecode from total seconds and frame rate.
    pub fn from_seconds(time_seconds: f64, fps: f32) -> Self {
        let total_frames = (time_seconds.max(0.0) * fps as f64).round() as u64;
        let fps_u64 = fps.round().max(1.0) as u64;

        let frames = (total_frames % fps_u64) as u8;
        let total_secs = total_frames / fps_u64;
        let seconds = (total_secs % 60) as u8;
        let total_mins = total_secs / 60;
        let minutes = (total_mins % 60) as u8;
        let hours = (total_mins / 60).min(99) as u8;

        Self {
            hours,
            minutes,
            seconds,
            frames,
        }
    }

    /// Convert timecode to continuous seconds for a given frame rate.
    pub fn to_seconds(&self, fps: f32) -> f64 {
        let base_secs =
            (self.hours as f64 * 3600.0) + (self.minutes as f64 * 60.0) + (self.seconds as f64);
        let frame_secs = self.frames as f64 / (fps as f64).max(1.0);
        base_secs + frame_secs
    }
}

impl fmt::Display for Timecode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02}:{:02}:{:02}:{:02}",
            self.hours, self.minutes, self.seconds, self.frames
        )
    }
}
