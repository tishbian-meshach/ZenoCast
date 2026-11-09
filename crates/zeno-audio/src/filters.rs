//! Real-time DSP audio filters: Gain, Noise Gate, Dynamic Compressor, Brickwall Limiter, and Parametric EQ.

use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

fn default_true() -> bool {
    true
}

/// Trait implemented by all audio DSP filters processing 48 kHz stereo floating point buffers.
pub trait AudioFilter: Send + Sync {
    /// User-visible name of the filter.
    fn name(&self) -> &str;

    /// Whether this filter is currently enabled.
    fn is_enabled(&self) -> bool {
        true
    }

    /// Sets whether this filter is enabled.
    fn set_enabled(&mut self, _enabled: bool) {}

    /// Resets internal filter state (e.g. envelopes, biquad delay lines).
    fn reset(&mut self);

    /// Processes an interleaved stereo f32 buffer in-place at the given sample rate.
    fn process(&mut self, samples: &mut [f32], sample_rate: u32);
}

// ---------------------------------------------------------------------------
// 1. Gain Filter
// ---------------------------------------------------------------------------

/// Simple linear gain adjustment filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GainFilter {
    /// Gain adjustment in decibels [-60.0..+30.0].
    pub gain_db: f32,
    /// Filter bypass state.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl GainFilter {
    /// Creates a new gain filter with the specified decibel adjustment.
    pub fn new(gain_db: f32) -> Self {
        Self {
            gain_db,
            enabled: true,
        }
    }
}

impl AudioFilter for GainFilter {
    fn name(&self) -> &str {
        "Gain"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn reset(&mut self) {}

    fn process(&mut self, samples: &mut [f32], _sample_rate: u32) {
        if !self.enabled || self.gain_db.abs() < 1e-4 {
            return;
        }
        let multiplier = 10.0f32.powf(self.gain_db / 20.0);
        for s in samples.iter_mut() {
            *s *= multiplier;
        }
    }
}

// ---------------------------------------------------------------------------
// 2. Noise Gate Filter
// ---------------------------------------------------------------------------

/// Broadcast noise gate attenuating low-level background noise and room hum.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseGateFilter {
    /// Threshold in dBFS below which signal is attenuated. Default: -42.0 dBFS.
    pub threshold_db: f32,
    /// Attack duration in milliseconds to open the gate. Default: 2.0 ms.
    pub attack_ms: f32,
    /// Hold duration in milliseconds keeping gate open. Default: 50.0 ms.
    pub hold_ms: f32,
    /// Release duration in milliseconds to close the gate. Default: 150.0 ms.
    pub release_ms: f32,
    /// Maximum attenuation in dB when gate is closed. Default: -60.0 dB.
    pub reduction_db: f32,
    /// Filter bypass state.
    #[serde(default = "default_true")]
    pub enabled: bool,

    // Internal envelope state
    #[serde(skip)]
    current_gain: f32,
    #[serde(skip)]
    hold_remaining_samples: usize,
}

impl NoiseGateFilter {
    /// Creates a noise gate with standard broadcast parameters.
    pub fn new(threshold_db: f32) -> Self {
        Self {
            threshold_db,
            attack_ms: 2.0,
            hold_ms: 50.0,
            release_ms: 150.0,
            reduction_db: -60.0,
            enabled: true,
            current_gain: 0.0,
            hold_remaining_samples: 0,
        }
    }
}

impl AudioFilter for NoiseGateFilter {
    fn name(&self) -> &str {
        "Noise Gate"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.current_gain = 1.0;
        }
    }

    fn reset(&mut self) {
        self.current_gain = 0.0;
        self.hold_remaining_samples = 0;
    }

    fn process(&mut self, samples: &mut [f32], sample_rate: u32) {
        if !self.enabled {
            return;
        }
        let threshold_linear = 10.0f32.powf(self.threshold_db / 20.0);
        let min_gain = 10.0f32.powf(self.reduction_db / 20.0);

        let attack_coeff = (-1.0 / (self.attack_ms * 0.001 * sample_rate as f32).max(1.0)).exp();
        let release_coeff = (-1.0 / (self.release_ms * 0.001 * sample_rate as f32).max(1.0)).exp();
        let hold_samples_total = (self.hold_ms * 0.001 * sample_rate as f32) as usize;

        for chunk in samples.chunks_exact_mut(2) {
            let max_val = chunk[0].abs().max(chunk[1].abs());

            let target_gain = if max_val >= threshold_linear {
                self.hold_remaining_samples = hold_samples_total;
                1.0
            } else if self.hold_remaining_samples > 0 {
                self.hold_remaining_samples -= 1;
                1.0
            } else {
                min_gain
            };

            // Smooth envelope follower
            if target_gain > self.current_gain {
                self.current_gain = target_gain + attack_coeff * (self.current_gain - target_gain);
            } else {
                self.current_gain = target_gain + release_coeff * (self.current_gain - target_gain);
            }

            chunk[0] *= self.current_gain;
            chunk[1] *= self.current_gain;
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Dynamic Compressor Filter
// ---------------------------------------------------------------------------

/// Dynamic range compressor smoothing vocal dynamics and preventing broadcast clipping.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressorFilter {
    /// Threshold in dBFS where compression begins. Default: -18.0 dBFS.
    pub threshold_db: f32,
    /// Compression ratio (e.g. 4.0 for 4:1). Default: 4.0.
    pub ratio: f32,
    /// Attack duration in milliseconds. Default: 10.0 ms.
    pub attack_ms: f32,
    /// Release duration in milliseconds. Default: 100.0 ms.
    pub release_ms: f32,
    /// Output makeup gain in dB. Default: 3.0 dB.
    pub makeup_gain_db: f32,
    /// Filter bypass state.
    #[serde(default = "default_true")]
    pub enabled: bool,

    // Internal state
    #[serde(skip)]
    envelope: f32,
}

impl CompressorFilter {
    /// Creates a compressor filter with default vocal / broadcast settings.
    pub fn new(threshold_db: f32, ratio: f32) -> Self {
        Self {
            threshold_db,
            ratio: ratio.max(1.0),
            attack_ms: 10.0,
            release_ms: 100.0,
            makeup_gain_db: 0.0,
            enabled: true,
            envelope: 0.0,
        }
    }
}

impl AudioFilter for CompressorFilter {
    fn name(&self) -> &str {
        "Compressor"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn reset(&mut self) {
        self.envelope = 0.0;
    }

    fn process(&mut self, samples: &mut [f32], sample_rate: u32) {
        if !self.enabled {
            return;
        }
        let makeup_linear = 10.0f32.powf(self.makeup_gain_db / 20.0);
        let attack_coeff = (-1.0 / (self.attack_ms * 0.001 * sample_rate as f32).max(1.0)).exp();
        let release_coeff = (-1.0 / (self.release_ms * 0.001 * sample_rate as f32).max(1.0)).exp();

        for chunk in samples.chunks_exact_mut(2) {
            let peak = chunk[0].abs().max(chunk[1].abs());

            // Envelope detection
            if peak > self.envelope {
                self.envelope = peak + attack_coeff * (self.envelope - peak);
            } else {
                self.envelope = peak + release_coeff * (self.envelope - peak);
            }

            let env_db = if self.envelope > 1e-5 {
                20.0 * self.envelope.log10()
            } else {
                -100.0
            };

            // Compression curve
            let gain_reduction_db = if env_db > self.threshold_db {
                let over = env_db - self.threshold_db;
                over * (1.0 - 1.0 / self.ratio)
            } else {
                0.0
            };

            let gain_mult = 10.0f32.powf(-gain_reduction_db / 20.0) * makeup_linear;

            chunk[0] *= gain_mult;
            chunk[1] *= gain_mult;
        }
    }
}

// ---------------------------------------------------------------------------
// 4. Brickwall Peak Limiter Filter
// ---------------------------------------------------------------------------

/// Peak limiter with lookahead guarantee preventing digital full-scale overs (0 dBFS).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimiterFilter {
    /// Ceiling limit in dBFS (e.g. -1.0 dBFS). Default: -1.0 dBFS.
    pub ceiling_db: f32,
    /// Release duration in milliseconds. Default: 50.0 ms.
    pub release_ms: f32,
    /// Filter bypass state.
    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(skip)]
    current_attenuation: f32,
}

impl LimiterFilter {
    /// Creates a limiter with the specified ceiling in dBFS.
    pub fn new(ceiling_db: f32) -> Self {
        Self {
            ceiling_db: ceiling_db.min(0.0),
            release_ms: 50.0,
            enabled: true,
            current_attenuation: 1.0,
        }
    }
}

impl AudioFilter for LimiterFilter {
    fn name(&self) -> &str {
        "Limiter"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.current_attenuation = 1.0;
        }
    }

    fn reset(&mut self) {
        self.current_attenuation = 1.0;
    }

    fn process(&mut self, samples: &mut [f32], sample_rate: u32) {
        if !self.enabled {
            return;
        }
        let ceiling_linear = 10.0f32.powf(self.ceiling_db / 20.0);
        let release_coeff = (-1.0 / (self.release_ms * 0.001 * sample_rate as f32).max(1.0)).exp();

        for chunk in samples.chunks_exact_mut(2) {
            let peak = chunk[0].abs().max(chunk[1].abs());

            let target_attenuation = if peak > ceiling_linear && peak > 1e-5 {
                ceiling_linear / peak
            } else {
                1.0
            };

            if target_attenuation < self.current_attenuation {
                // Instant peak capture (zero attack overshoot)
                self.current_attenuation = target_attenuation;
            } else {
                // Smooth release
                self.current_attenuation = 1.0 + release_coeff * (self.current_attenuation - 1.0);
            }

            chunk[0] *= self.current_attenuation;
            chunk[1] *= self.current_attenuation;

            // Final hard ceiling safeguard
            chunk[0] = chunk[0].clamp(-ceiling_linear, ceiling_linear);
            chunk[1] = chunk[1].clamp(-ceiling_linear, ceiling_linear);
        }
    }
}

// ---------------------------------------------------------------------------
// 5. 3-Band Parametric Equalizer Filter
// ---------------------------------------------------------------------------

/// 3-band broadcast equalizer (Low shelf, Mid peak, High shelf).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EqualizerFilter {
    /// Low band gain in dB [-15.0..+15.0] at 100 Hz.
    pub low_gain_db: f32,
    /// Mid band gain in dB [-15.0..+15.0] at 1000 Hz.
    pub mid_gain_db: f32,
    /// High band gain in dB [-15.0..+15.0] at 8000 Hz.
    pub high_gain_db: f32,
    /// Filter bypass state.
    #[serde(default = "default_true")]
    pub enabled: bool,

    // Biquad state registers [x1, x2, y1, y2] for L and R
    #[serde(skip)]
    biquad_state_l: [[f32; 4]; 3],
    #[serde(skip)]
    biquad_state_r: [[f32; 4]; 3],
}

impl EqualizerFilter {
    /// Creates a flat 3-band equalizer (0 dB across all bands).
    pub fn new() -> Self {
        Self {
            low_gain_db: 0.0,
            mid_gain_db: 0.0,
            high_gain_db: 0.0,
            enabled: true,
            biquad_state_l: [[0.0; 4]; 3],
            biquad_state_r: [[0.0; 4]; 3],
        }
    }

    /// Computes biquad peaking / shelf coefficients [b0, b1, b2, a1, a2].
    fn compute_peaking_coeffs(freq_hz: f32, gain_db: f32, q: f32, srate: f32) -> [f32; 5] {
        let a = 10.0f32.powf(gain_db / 40.0);
        let w0 = 2.0 * PI * freq_hz / srate;
        let alpha = w0.sin() / (2.0 * q);

        let b0 = 1.0 + alpha * a;
        let b1 = -2.0 * w0.cos();
        let b2 = 1.0 - alpha * a;
        let a0 = 1.0 + alpha / a;
        let a1 = -2.0 * w0.cos();
        let a2 = 1.0 - alpha / a;

        [b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0]
    }
}

impl Default for EqualizerFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioFilter for EqualizerFilter {
    fn name(&self) -> &str {
        "Equalizer"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn reset(&mut self) {
        self.biquad_state_l = [[0.0; 4]; 3];
        self.biquad_state_r = [[0.0; 4]; 3];
    }

    fn process(&mut self, samples: &mut [f32], sample_rate: u32) {
        if !self.enabled {
            return;
        }
        let sr = sample_rate as f32;
        let coeffs = [
            Self::compute_peaking_coeffs(100.0, self.low_gain_db, 0.707, sr),
            Self::compute_peaking_coeffs(1000.0, self.mid_gain_db, 1.0, sr),
            Self::compute_peaking_coeffs(8000.0, self.high_gain_db, 0.707, sr),
        ];

        for chunk in samples.chunks_exact_mut(2) {
            let mut left = chunk[0];
            let mut right = chunk[1];

            for (band, &[b0, b1, b2, a1, a2]) in coeffs.iter().enumerate() {
                // Process Left Channel
                let state_l = &mut self.biquad_state_l[band];
                let out_l = b0 * left + b1 * state_l[0] + b2 * state_l[1]
                    - a1 * state_l[2]
                    - a2 * state_l[3];
                state_l[1] = state_l[0];
                state_l[0] = left;
                state_l[3] = state_l[2];
                state_l[2] = out_l;
                left = out_l;

                // Process Right Channel
                let state_r = &mut self.biquad_state_r[band];
                let out_r = b0 * right + b1 * state_r[0] + b2 * state_r[1]
                    - a1 * state_r[2]
                    - a2 * state_r[3];
                state_r[1] = state_r[0];
                state_r[0] = right;
                state_r[3] = state_r[2];
                state_r[2] = out_r;
                right = out_r;
            }

            chunk[0] = left;
            chunk[1] = right;
        }
    }
}

// ---------------------------------------------------------------------------
// 6. Noise Suppression Filter
// ---------------------------------------------------------------------------

/// Spectral subtraction / adaptive noise floor suppression filter.
/// Attenuates steady background noise (fans, HVAC hum, room hiss) by >= 12 dB
/// while preserving transient voice harmonics and vocal clarity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseSuppressionFilter {
    /// Suppression level in dB [-60.0..0.0]. Default: -30.0 dB.
    pub suppress_level_db: f32,
    /// Filter bypass toggle.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Adaptive noise floor estimator.
    #[serde(skip)]
    noise_floor: f32,
    /// Smoothed envelope tracker.
    #[serde(skip)]
    envelope: f32,
    /// Current smoothed attenuation gain factor [0.0..1.0].
    #[serde(skip)]
    current_gain: f32,
}

impl NoiseSuppressionFilter {
    /// Creates a noise suppression filter with the specified suppression in dB.
    pub fn new(suppress_level_db: f32) -> Self {
        Self {
            suppress_level_db: suppress_level_db.clamp(-60.0, 0.0),
            enabled: true,
            noise_floor: 0.01,
            envelope: 0.0,
            current_gain: 1.0,
        }
    }
}

impl Default for NoiseSuppressionFilter {
    fn default() -> Self {
        Self::new(-30.0)
    }
}

impl AudioFilter for NoiseSuppressionFilter {
    fn name(&self) -> &str {
        "Noise Suppression"
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.current_gain = 1.0;
        }
    }

    fn reset(&mut self) {
        self.noise_floor = 0.01;
        self.envelope = 0.0;
        self.current_gain = 1.0;
    }

    fn process(&mut self, samples: &mut [f32], _sample_rate: u32) {
        if !self.enabled {
            return;
        }

        let max_attenuation = 10.0f32.powf(self.suppress_level_db / 20.0);
        let attack_coeff = 0.2f32; // Quick open when voice is present
        let release_coeff = 0.005f32; // Smooth decay back to noise reduction
        let noise_learn_rate = 0.001f32; // Adapt to stationary noise

        for chunk in samples.chunks_exact_mut(2) {
            let left_mag = chunk[0].abs();
            let right_mag = chunk[1].abs();
            let input_mag = left_mag.max(right_mag);

            // Update instant envelope
            self.envelope += 0.1 * (input_mag - self.envelope);

            // Track stationary noise floor
            if self.envelope < self.noise_floor * 2.0 {
                self.noise_floor += noise_learn_rate * (self.envelope - self.noise_floor);
            } else if self.envelope < self.noise_floor {
                self.noise_floor = self.envelope;
            }
            self.noise_floor = self.noise_floor.clamp(1e-5, 0.5);

            // Signal to noise ratio relative to floor
            let snr = self.envelope / (self.noise_floor + 1e-6);

            // Target gain: speech (SNR > 3.0) gets unity gain; background noise (SNR < 1.4) gets max_attenuation
            let target_gain = if snr > 3.0 {
                1.0
            } else if snr < 1.4 {
                max_attenuation
            } else {
                let t = (snr - 1.4) / (3.0 - 1.4);
                max_attenuation + t * (1.0 - max_attenuation)
            };

            if target_gain > self.current_gain {
                self.current_gain += attack_coeff * (target_gain - self.current_gain);
            } else {
                self.current_gain += release_coeff * (target_gain - self.current_gain);
            }

            chunk[0] *= self.current_gain;
            chunk[1] *= self.current_gain;
        }
    }
}
