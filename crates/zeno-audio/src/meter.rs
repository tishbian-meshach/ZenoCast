//! Broadcast audio metering: true peak, RMS, and LUFS loudness calculation.

use serde::{Deserialize, Serialize};

/// Audio metering metrics for a stereo audio channel.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct AudioMeter {
    /// Instantaneous peak level per channel in dBFS (left, right). Range [-100.0..0.0].
    pub peak_db: [f32; 2],
    /// RMS power level per channel in dBFS (left, right). Range [-100.0..0.0].
    pub rms_db: [f32; 2],
    /// Peak hold level per channel in dBFS.
    pub peak_hold_db: [f32; 2],
    /// Integrated / short-term Loudness Units Full Scale (LUFS). Target broadcast standard: -23 or -14 LUFS.
    pub lufs: f32,
    /// Decay rate per second in dB for visual meter ballistics.
    pub decay_db_per_sec: f32,
}

impl AudioMeter {
    /// Floor in dBFS representing digital silence.
    pub const SILENCE_DB: f32 = -100.0;

    /// Creates an empty meter initialized to silence.
    pub fn new() -> Self {
        Self {
            peak_db: [Self::SILENCE_DB; 2],
            rms_db: [Self::SILENCE_DB; 2],
            peak_hold_db: [Self::SILENCE_DB; 2],
            lufs: Self::SILENCE_DB,
            decay_db_per_sec: 40.0, // 40 dB/sec broadcast standard decay
        }
    }

    /// Updates meter readings from an interleaved stereo f32 buffer with elapsed time `dt`.
    pub fn update(&mut self, interleaved_stereo: &[f32], dt: f32) {
        if interleaved_stereo.is_empty() {
            self.decay(dt);
            return;
        }

        let mut peak_left = 0.0f32;
        let mut peak_right = 0.0f32;
        let mut sum_sq_left = 0.0f64;
        let mut sum_sq_right = 0.0f64;
        let mut pair_count = 0usize;

        for chunk in interleaved_stereo.chunks_exact(2) {
            let left = chunk[0].abs();
            let right = chunk[1].abs();

            if left > peak_left {
                peak_left = left;
            }
            if right > peak_right {
                peak_right = right;
            }

            sum_sq_left += (left * left) as f64;
            sum_sq_right += (right * right) as f64;
            pair_count += 1;
        }

        if pair_count == 0 {
            self.decay(dt);
            return;
        }

        let current_peak_left_db = Self::linear_to_db(peak_left);
        let current_peak_right_db = Self::linear_to_db(peak_right);

        let rms_left = (sum_sq_left / pair_count as f64).sqrt() as f32;
        let rms_right = (sum_sq_right / pair_count as f64).sqrt() as f32;
        let current_rms_left_db = Self::linear_to_db(rms_left);
        let current_rms_right_db = Self::linear_to_db(rms_right);

        // Apply fast attack and exponential release decay
        let decay = self.decay_db_per_sec * dt;

        self.peak_db[0] = current_peak_left_db.max(self.peak_db[0] - decay);
        self.peak_db[1] = current_peak_right_db.max(self.peak_db[1] - decay);

        self.rms_db[0] = current_rms_left_db.max(self.rms_db[0] - decay);
        self.rms_db[1] = current_rms_right_db.max(self.rms_db[1] - decay);

        // Update peak hold
        for ch in 0..2 {
            if self.peak_db[ch] >= self.peak_hold_db[ch] {
                self.peak_hold_db[ch] = self.peak_db[ch];
            } else {
                self.peak_hold_db[ch] =
                    (self.peak_hold_db[ch] - (decay * 0.5)).max(Self::SILENCE_DB);
            }
        }

        // Compute short-term LUFS (ITU-R BS.1770 K-weighting power sum approximation)
        let mean_square = (sum_sq_left + sum_sq_right) / (2.0 * pair_count as f64);
        let lkfs = if mean_square > 1e-10 {
            -0.691 + 10.0 * mean_square.log10() as f32
        } else {
            Self::SILENCE_DB
        };
        self.lufs = lkfs.max(self.lufs - decay);
    }

    /// Decays meter values toward silence.
    pub fn decay(&mut self, dt: f32) {
        let decay = self.decay_db_per_sec * dt;
        self.peak_db[0] = (self.peak_db[0] - decay).max(Self::SILENCE_DB);
        self.peak_db[1] = (self.peak_db[1] - decay).max(Self::SILENCE_DB);
        self.rms_db[0] = (self.rms_db[0] - decay).max(Self::SILENCE_DB);
        self.rms_db[1] = (self.rms_db[1] - decay).max(Self::SILENCE_DB);
        self.peak_hold_db[0] = (self.peak_hold_db[0] - decay * 0.5).max(Self::SILENCE_DB);
        self.peak_hold_db[1] = (self.peak_hold_db[1] - decay * 0.5).max(Self::SILENCE_DB);
        self.lufs = (self.lufs - decay).max(Self::SILENCE_DB);
    }

    /// Converts linear amplitude to decibels full scale (dBFS).
    #[inline]
    pub fn linear_to_db(linear: f32) -> f32 {
        if linear <= 1e-5 {
            Self::SILENCE_DB
        } else {
            (20.0 * linear.log10()).max(Self::SILENCE_DB)
        }
    }

    /// Converts dBFS to linear amplitude multiplier.
    #[inline]
    pub fn db_to_linear(db: f32) -> f32 {
        if db <= Self::SILENCE_DB {
            0.0
        } else {
            10.0f32.powf(db / 20.0)
        }
    }
}

impl Default for AudioMeter {
    fn default() -> Self {
        Self::new()
    }
}
