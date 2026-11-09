//! # zeno-audio
//!
//! Multi-track audio capture (WASAPI mic and desktop loopback), mixer, meters (peak, RMS, LUFS),
//! DSP filter chain (gain, gate, compressor, limiter, EQ), and clock drift correction against video.

pub mod drift;
pub mod error;
pub mod filters;
pub mod meter;
pub mod mixer;
pub mod source;
pub mod wasapi;

// Re-exports
pub use drift::DriftCorrector;
pub use error::AudioError;
pub use filters::{
    AudioFilter, CompressorFilter, EqualizerFilter, GainFilter, LimiterFilter, NoiseGateFilter,
    NoiseSuppressionFilter,
};
pub use meter::AudioMeter;
pub use mixer::{AudioMixResult, AudioMixer};
pub use source::AudioSource;
pub use wasapi::{enumerate_audio_devices, AudioDeviceInfo};

#[cfg(test)]
mod tests {
    use super::*;
    use zeno_core::SourceId;

    #[test]
    fn test_gain_filter() {
        let mut gain = GainFilter::new(6.0); // +6 dB ~ 2x amplitude
        let mut samples = vec![0.5f32, -0.5f32];
        gain.process(&mut samples, 48000);

        // 0.5 * 10^(6/20) ~= 0.5 * 1.995 ~= 0.9976
        assert!((samples[0] - 0.9976).abs() < 1e-3);
        assert!((samples[1] - (-0.9976)).abs() < 1e-3);
    }

    #[test]
    fn test_limiter_brickwall_ceiling() {
        let mut limiter = LimiterFilter::new(-1.0); // -1.0 dBFS ceiling = ~0.891
        let ceiling_linear = AudioMeter::db_to_linear(-1.0);

        let mut samples = vec![1.5f32, -2.0f32, 3.0f32, -1.2f32];
        limiter.process(&mut samples, 48000);

        for s in samples {
            assert!(
                s.abs() <= ceiling_linear + 1e-5,
                "Sample {} exceeded brickwall ceiling {}",
                s,
                ceiling_linear
            );
        }
    }

    #[test]
    fn test_noise_gate_attenuation() {
        let mut gate = NoiseGateFilter::new(-40.0);
        // Audio at -60 dBFS (below gate threshold)
        let low_val = AudioMeter::db_to_linear(-60.0);
        let mut samples = vec![low_val; 960];

        // Process several chunks to allow gate release to close
        for _ in 0..10 {
            gate.process(&mut samples, 48000);
        }

        // Gate should attenuate the low-level signal
        assert!(samples[0].abs() < low_val);
    }

    #[test]
    fn test_audio_meter_peak_and_rms() {
        let mut meter = AudioMeter::new();
        let samples = [0.5f32, -0.5f32].repeat(480);
        meter.update(&samples, 0.01);

        // 0.5 linear peak is 20*log10(0.5) = -6.02 dBFS
        assert!((meter.peak_db[0] - (-6.02)).abs() < 0.5);
        assert!((meter.rms_db[0] - (-6.02)).abs() < 0.5);
    }

    #[test]
    fn test_drift_corrector_micro_resampling() {
        let mut corrector = DriftCorrector::new(48000);

        // Feed 1 second of audio (48,000 sample pairs = 96,000 floats)
        let mut buffer = vec![0.1f32; 96000];

        // Master clock reports 1.010 s (audio is lagging video by 10 ms > 5 ms threshold)
        corrector.process_and_correct(&mut buffer, 1.010);

        // Correction must have inserted an interpolated sample pair to compensate
        assert!(corrector.total_corrections > 0);
        assert_eq!(buffer.len(), 96002);
    }

    #[test]
    fn test_mixer_source_routing_and_master() {
        let mut mixer = AudioMixer::new();
        let src1_id = SourceId::new();
        let src2_id = SourceId::new();

        let mut src1 = AudioSource::new(src1_id, "Microphone", 48000);
        let mut src2 = AudioSource::new(src2_id, "Desktop Audio", 48000);

        // Ingest audio into both sources
        let audio1 = vec![0.2f32; 1920];
        let audio2 = vec![0.3f32; 1920];

        src1.ingest_samples(&audio1, 0.0);
        src2.ingest_samples(&audio2, 0.0);

        mixer.add_source(src1);
        mixer.add_source(src2);

        // Mix 960 pairs (20 ms at 48 kHz)
        let result = mixer.process_mix(960, 0.02);
        assert_eq!(result.master_samples.len(), 1920);

        // Track 0 (Master) should sum both sources (0.2 + 0.3 = 0.5)
        assert!((result.master_samples[0] - 0.5).abs() < 1e-3);
    }

    #[test]
    fn test_noise_suppression_attenuates_background_noise_and_preserves_voice() {
        let mut suppressor = NoiseSuppressionFilter::new(-24.0); // -24 dB attenuation

        // 1. Feed steady low-level background noise (0.02 amplitude, approx -34 dBFS)
        let noise_samples_per_frame = 960 * 2;
        let noise_val = 0.02f32;
        let mut noise_chunk = vec![noise_val; noise_samples_per_frame];

        for _ in 0..10 {
            suppressor.process(&mut noise_chunk, 48000);
        }

        // Measure noise attenuation after convergence
        let final_noise = noise_chunk[0].abs();
        let attenuation_db = 20.0 * (noise_val / final_noise.max(1e-6)).log10();
        assert!(
            attenuation_db >= 12.0,
            "Noise suppression must attenuate steady background noise by at least 12 dB (got {:.1} dB)",
            attenuation_db
        );

        // 2. Feed voice utterance (0.6 amplitude, approx -4.4 dBFS)
        let voice_val = 0.6f32;
        let mut voice_chunk = vec![voice_val; noise_samples_per_frame];
        for _ in 0..5 {
            suppressor.process(&mut voice_chunk, 48000);
        }

        // Voice must pass through without voice distortion (< 1.5 dB attenuation)
        let voice_out = voice_chunk[voice_chunk.len() - 1].abs();
        let voice_loss_db = 20.0 * (voice_val / voice_out.max(1e-6)).log10();
        assert!(
            voice_loss_db < 1.5,
            "Voice must pass through with negligible loss (loss was {:.2} dB)",
            voice_loss_db
        );

        // 3. Verify bypass toggle
        suppressor.set_enabled(false);
        assert!(!suppressor.is_enabled());
        let mut test_chunk = vec![0.02f32; noise_samples_per_frame];
        suppressor.process(&mut test_chunk, 48000);
        assert_eq!(
            test_chunk[0], 0.02f32,
            "Bypassed filter must not alter audio"
        );
    }

    #[test]
    fn test_source_filter_chain_reorder_and_bypass() {
        let src_id = SourceId::new();
        let mut src = AudioSource::new(src_id, "Filter Mic", 48000);

        // Add Gain (+6 dB) and Noise Suppression
        src.add_filter(Box::new(GainFilter::new(6.0)));
        src.add_filter(Box::new(NoiseSuppressionFilter::new(-20.0)));
        assert_eq!(src.filter_count(), 2);
        assert_eq!(src.filters[0].name(), "Gain");
        assert_eq!(src.filters[1].name(), "Noise Suppression");

        // Reorder filters
        src.move_filter(0, 1);
        assert_eq!(src.filters[0].name(), "Noise Suppression");
        assert_eq!(src.filters[1].name(), "Gain");

        // Toggle bypass on first filter
        src.set_filter_enabled(0, false);
        assert!(!src.filters[0].is_enabled());

        // Process a chunk and verify no panic or audio glitch
        let samples = vec![0.1f32; 1920];
        src.ingest_samples(&samples, 0.0);
        let processed = src.pull_processed_chunk(960, 0.02);
        assert_eq!(processed.len(), 1920);
        // Gain is second (enabled, +6 dB = ~2x multiplier)
        assert!((processed[0] - 0.2).abs() < 0.02);
    }
}
