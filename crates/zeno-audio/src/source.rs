//! Audio source representation with per-source volume, mute, track routing, DSP chain, and metering.

use crate::drift::DriftCorrector;
use crate::filters::AudioFilter;
use crate::meter::AudioMeter;
use rubato::{FftFixedInOut, Resampler};
use std::collections::VecDeque;
use zeno_core::SourceId;

/// An active audio input source (microphone, capture card, loopback, or media track).
pub struct AudioSource {
    /// Unique identifier for this source.
    pub id: SourceId,
    /// User-friendly name of the audio source.
    pub name: String,
    /// Fader volume level in dB [-60.0..+12.0]. Default: 0.0 dB.
    pub volume_db: f32,
    /// Mute toggle.
    pub muted: bool,
    /// Monitor-only toggle (not broadcast to stream/record).
    pub monitor_only: bool,
    /// 6-track broadcast routing flags. Track 0 is the main stream mix.
    pub track_routing: [bool; 6],
    /// Ordered DSP filter processing chain (Gain, Gate, Compressor, Limiter, EQ).
    pub filters: Vec<Box<dyn AudioFilter>>,
    /// True peak, RMS, and LUFS live meter readings.
    pub meter: AudioMeter,
    /// Audio-video clock drift corrector.
    pub drift_corrector: DriftCorrector,
    /// Native sample rate of the input hardware device (e.g. 44100, 48000, 96000).
    pub input_sample_rate: u32,
    /// High-performance Rubato FFT resampler to convert native input rate to 48000 Hz.
    resampler: Option<FftFixedInOut<f32>>,
    /// Ingest FIFO buffer holding incoming 48 kHz stereo samples ready for mixing.
    ingest_buffer: VecDeque<f32>,
    /// Intermediate processing scratch buffer.
    scratch_buffer: Vec<f32>,
}

impl AudioSource {
    /// Nominal master mixing sample rate (standard broadcast 48 kHz).
    pub const TARGET_SAMPLE_RATE: u32 = 48000;

    /// Creates a new audio source with standard defaults.
    pub fn new(id: SourceId, name: impl Into<String>, input_sample_rate: u32) -> Self {
        let resampler = if input_sample_rate != Self::TARGET_SAMPLE_RATE && input_sample_rate > 0 {
            // Rubato FFT resampler: chunk size 1024, 2 stereo channels
            FftFixedInOut::<f32>::new(
                input_sample_rate as usize,
                Self::TARGET_SAMPLE_RATE as usize,
                1024,
                2,
            )
            .ok()
        } else {
            None
        };

        let mut routing = [false; 6];
        routing[0] = true; // Route to Master Track 1 by default

        Self {
            id,
            name: name.into(),
            volume_db: 0.0,
            muted: false,
            monitor_only: false,
            track_routing: routing,
            filters: Vec::new(),
            meter: AudioMeter::new(),
            drift_corrector: DriftCorrector::new(Self::TARGET_SAMPLE_RATE),
            input_sample_rate,
            resampler,
            ingest_buffer: VecDeque::with_capacity(9600), // ~100 ms buffer
            scratch_buffer: Vec::with_capacity(1920),
        }
    }

    /// Sets source volume in decibels.
    pub fn set_volume_db(&mut self, db: f32) {
        self.volume_db = db.clamp(-60.0, 12.0);
    }

    /// Appends a DSP filter to the processing chain.
    pub fn add_filter(&mut self, filter: Box<dyn AudioFilter>) {
        self.filters.push(filter);
    }

    /// Ingests raw interleaved stereo samples from the device capture stream.
    pub fn ingest_samples(&mut self, raw_samples: &[f32], master_clock_seconds: f64) {
        if raw_samples.is_empty() {
            return;
        }

        self.scratch_buffer.clear();

        if let Some(ref mut resampler) = self.resampler {
            // De-interleave into Rubato channel buffers
            let mut left = Vec::with_capacity(raw_samples.len() / 2);
            let mut right = Vec::with_capacity(raw_samples.len() / 2);
            for chunk in raw_samples.chunks_exact(2) {
                left.push(chunk[0]);
                right.push(chunk[1]);
            }

            // Resample
            let waves_in = vec![left, right];
            if let Ok(waves_out) = resampler.process(&waves_in, None) {
                if waves_out.len() >= 2 {
                    for (&l, &r) in waves_out[0].iter().zip(waves_out[1].iter()) {
                        self.scratch_buffer.push(l);
                        self.scratch_buffer.push(r);
                    }
                }
            }
        } else {
            self.scratch_buffer.extend_from_slice(raw_samples);
        }

        // Apply clock drift correction against master broadcast video clock
        self.drift_corrector
            .process_and_correct(&mut self.scratch_buffer, master_clock_seconds);

        // Append to ingest FIFO (bounded to prevent runaway memory usage)
        if self.ingest_buffer.len() > 96000 {
            // Drop oldest 20 ms if buffer is backing up
            self.ingest_buffer.drain(0..1920);
        }
        self.ingest_buffer
            .extend(self.scratch_buffer.iter().copied());
    }

    /// Pulls and processes a chunk of `pair_count` stereo samples (2 * pair_count floats).
    pub fn pull_processed_chunk(&mut self, pair_count: usize, dt: f32) -> Vec<f32> {
        let total_floats = pair_count * 2;
        let mut chunk = Vec::with_capacity(total_floats);

        for _ in 0..total_floats {
            if let Some(sample) = self.ingest_buffer.pop_front() {
                chunk.push(sample);
            } else {
                chunk.push(0.0f32); // Silence underrun padding
            }
        }

        // Apply mute
        if self.muted {
            for s in chunk.iter_mut() {
                *s = 0.0;
            }
            self.meter.update(&chunk, dt);
            return chunk;
        }

        // Apply DSP filter chain
        for filter in &mut self.filters {
            filter.process(&mut chunk, Self::TARGET_SAMPLE_RATE);
        }

        // Apply fader gain
        if self.volume_db.abs() > 1e-4 {
            let mult = 10.0f32.powf(self.volume_db / 20.0);
            for s in chunk.iter_mut() {
                *s *= mult;
            }
        }

        // Update live peak, RMS, and LUFS meters
        self.meter.update(&chunk, dt);

        chunk
    }
}
