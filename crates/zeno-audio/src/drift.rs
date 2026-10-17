//! Clock drift detection and micro-resampling correction against the master video clock.

use serde::{Deserialize, Serialize};

/// Real-time audio clock drift detector and compensator.
///
/// Keeps audio sample timing synchronized to the master broadcast video clock
/// within a sub-20ms tolerance over continuous multi-hour broadcasts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftCorrector {
    /// Nominal audio sample rate (standard: 48000 Hz).
    pub sample_rate: u32,
    /// Total cumulative audio samples processed.
    pub total_samples_processed: u64,
    /// Master reference clock time in seconds.
    pub master_clock_seconds: f64,
    /// Maximum allowable drift threshold in seconds before micro-correction triggers (default: 0.005 s = 5 ms).
    pub drift_threshold_seconds: f64,
    /// Count of micro-corrected samples (inserted or dropped).
    pub total_corrections: i64,
}

impl DriftCorrector {
    /// Creates a drift corrector configured for 48 kHz broadcast audio.
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            total_samples_processed: 0,
            master_clock_seconds: 0.0,
            drift_threshold_seconds: 0.005, // 5 ms threshold
            total_corrections: 0,
        }
    }

    /// Computes the current calculated audio time based on processed samples.
    #[inline]
    pub fn audio_time_seconds(&self) -> f64 {
        self.total_samples_processed as f64 / (self.sample_rate as f64)
    }

    /// Computes the instantaneous drift between audio time and master video clock.
    /// Positive value means audio is ahead of video; negative means audio is behind.
    #[inline]
    pub fn current_drift_seconds(&self) -> f64 {
        self.audio_time_seconds() - self.master_clock_seconds
    }

    /// Updates the master video clock reference time.
    pub fn update_master_clock(&mut self, master_time_seconds: f64) {
        self.master_clock_seconds = master_time_seconds;
    }

    /// Applies drift compensation to an interleaved stereo f32 buffer in-place.
    ///
    /// If audio leads video by > 5 ms: smooth drops of 1 sample pair.
    /// If audio lags video by > 5 ms: smooth interpolation of 1 sample pair.
    pub fn process_and_correct(&mut self, buffer: &mut Vec<f32>, master_time_seconds: f64) {
        self.update_master_clock(master_time_seconds);

        let initial_pair_count = buffer.len() / 2;
        self.total_samples_processed += initial_pair_count as u64;

        let drift = self.current_drift_seconds();

        // Audio is running too fast (leading video by > threshold)
        if drift > self.drift_threshold_seconds && buffer.len() >= 4 {
            // Drop 1 stereo sample pair smoothly via linear crossfade
            let idx = buffer.len() - 4;
            buffer[idx] = (buffer[idx] + buffer[idx + 2]) * 0.5;
            buffer[idx + 1] = (buffer[idx + 1] + buffer[idx + 3]) * 0.5;
            buffer.truncate(buffer.len() - 2);
            self.total_samples_processed = self.total_samples_processed.saturating_sub(1);
            self.total_corrections -= 1;
        }
        // Audio is running too slow (lagging video by > threshold)
        else if drift < -self.drift_threshold_seconds && buffer.len() >= 4 {
            // Insert 1 interpolated stereo sample pair at the tail
            let len = buffer.len();
            let interp_l = (buffer[len - 4] + buffer[len - 2]) * 0.5;
            let interp_r = (buffer[len - 3] + buffer[len - 1]) * 0.5;
            buffer.push(interp_l);
            buffer.push(interp_r);
            self.total_samples_processed += 1;
            self.total_corrections += 1;
        }
    }

    /// Resets clock tracking to zero.
    pub fn reset(&mut self) {
        self.total_samples_processed = 0;
        self.master_clock_seconds = 0.0;
        self.total_corrections = 0;
    }
}
