//! Master 48 kHz multi-track audio mixer with master brickwall limiter and metering.

use crate::filters::{AudioFilter, LimiterFilter};
use crate::meter::AudioMeter;
use crate::source::AudioSource;
use std::collections::HashMap;
use zeno_core::SourceId;

/// Output packet from the audio mixer containing 48 kHz stereo floating point buffers.
pub struct AudioMixResult {
    /// Main master broadcast mix (48 kHz stereo interleaved f32).
    pub master_samples: Vec<f32>,
    /// Individual mixes for each of the 6 broadcast audio tracks.
    pub track_samples: [Vec<f32>; 6],
    /// Capture timestamp in seconds matching master broadcast video clock.
    pub timestamp_seconds: f64,
}

/// Multi-track broadcast audio mixer operating at fixed 48 kHz float.
pub struct AudioMixer {
    /// Active audio sources.
    sources: HashMap<SourceId, AudioSource>,
    /// Master broadcast output volume in dB [-60.0..+12.0]. Default: 0.0 dB.
    pub master_volume_db: f32,
    /// Master broadcast mute.
    pub master_muted: bool,
    /// Final brickwall limiter on master bus (default: -1.0 dBFS ceiling).
    pub master_limiter: LimiterFilter,
    /// Master output peak, RMS, and LUFS meter readings.
    pub master_meter: AudioMeter,
    /// Per-track meters for tracks 1 through 6.
    pub track_meters: [AudioMeter; 6],
}

impl AudioMixer {
    /// Standard broadcast audio sample rate (48 kHz).
    pub const SAMPLE_RATE: u32 = 48000;

    /// Creates an empty audio mixer with default master limiter and meters.
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            master_volume_db: 0.0,
            master_muted: false,
            master_limiter: LimiterFilter::new(-1.0),
            master_meter: AudioMeter::new(),
            track_meters: [AudioMeter::new(); 6],
        }
    }

    /// Adds an audio source to the mixer.
    pub fn add_source(&mut self, source: AudioSource) {
        self.sources.insert(source.id, source);
    }

    /// Retrieves reference to an audio source by ID.
    pub fn get_source(&self, id: SourceId) -> Option<&AudioSource> {
        self.sources.get(&id)
    }

    /// Retrieves mutable reference to an audio source by ID.
    pub fn get_source_mut(&mut self, id: SourceId) -> Option<&mut AudioSource> {
        self.sources.get_mut(&id)
    }

    /// Removes an audio source by ID.
    pub fn remove_source(&mut self, id: SourceId) -> Option<AudioSource> {
        self.sources.remove(&id)
    }

    /// Returns iterator over all active audio sources.
    pub fn sources(&self) -> impl Iterator<Item = &AudioSource> {
        self.sources.values()
    }

    /// Returns mutable iterator over all active audio sources.
    pub fn sources_mut(&mut self) -> impl Iterator<Item = &mut AudioSource> {
        self.sources.values_mut()
    }

    /// Mixes and processes a block of `pair_count` stereo samples across all active sources.
    ///
    /// Real-time path guarantee: fixed-size processing with zero file IO and no blocking locks.
    pub fn process_mix(&mut self, pair_count: usize, master_clock_seconds: f64) -> AudioMixResult {
        let total_floats = pair_count * 2;
        let dt = pair_count as f32 / Self::SAMPLE_RATE as f32;

        // Initialize 6 track accumulator buffers
        let mut tracks: [Vec<f32>; 6] = [
            vec![0.0f32; total_floats],
            vec![0.0f32; total_floats],
            vec![0.0f32; total_floats],
            vec![0.0f32; total_floats],
            vec![0.0f32; total_floats],
            vec![0.0f32; total_floats],
        ];

        // Process and sum each source into routed tracks
        for source in self.sources.values_mut() {
            let chunk = source.pull_processed_chunk(pair_count, dt);

            if source.monitor_only {
                continue;
            }

            for (track_idx, is_routed) in source.track_routing.iter().enumerate() {
                if *is_routed && track_idx < 6 {
                    for (dst, src) in tracks[track_idx].iter_mut().zip(chunk.iter()) {
                        *dst += *src;
                    }
                }
            }
        }

        // Master Track 0 processing: apply master volume, mute, and brickwall limiter
        let mut master = tracks[0].clone();

        if self.master_muted {
            for s in master.iter_mut() {
                *s = 0.0;
            }
        } else {
            // Master volume gain
            if self.master_volume_db.abs() > 1e-4 {
                let gain = 10.0f32.powf(self.master_volume_db / 20.0);
                for s in master.iter_mut() {
                    *s *= gain;
                }
            }

            // Brickwall limiter safeguarding against clipping overs (> 0 dBFS)
            self.master_limiter.process(&mut master, Self::SAMPLE_RATE);
        }

        // Update master and track meters
        self.master_meter.update(&master, dt);
        for (i, track) in tracks.iter().enumerate() {
            self.track_meters[i].update(track, dt);
        }

        AudioMixResult {
            master_samples: master,
            track_samples: tracks,
            timestamp_seconds: master_clock_seconds,
        }
    }
}

impl Default for AudioMixer {
    fn default() -> Self {
        Self::new()
    }
}
