//! M6 Acceptance Test: Two audio sources mixed with live meters and 30-minute clock drift test under 20 ms.

use std::time::Instant;
use zeno_audio::{
    AudioMeter, AudioMixer, AudioSource, CompressorFilter, EqualizerFilter, GainFilter,
    LimiterFilter, NoiseGateFilter,
};
use zeno_core::SourceId;

#[test]
fn test_m6_two_sources_mixed_with_live_meters() {
    let mut mixer = AudioMixer::new();

    let mic_id = SourceId::new();
    let mut mic_source = AudioSource::new(mic_id, "Host Microphone", 48000);
    mic_source.add_filter(Box::new(NoiseGateFilter::new(-45.0)));
    mic_source.add_filter(Box::new(CompressorFilter::new(-16.0, 4.0)));
    mic_source.add_filter(Box::new(LimiterFilter::new(-1.0)));

    let desktop_id = SourceId::new();
    let mut desktop_source = AudioSource::new(desktop_id, "Desktop Game Audio", 48000);
    desktop_source.add_filter(Box::new(GainFilter::new(-3.0))); // -3 dB
    desktop_source.add_filter(Box::new(EqualizerFilter::new()));

    // Generate test audio signals (20 ms = 960 stereo pairs = 1920 floats)
    let chunk_pairs = 960;
    let mut mic_samples = Vec::with_capacity(chunk_pairs * 2);
    let mut desktop_samples = Vec::with_capacity(chunk_pairs * 2);

    for i in 0..chunk_pairs {
        // 440 Hz vocal tone on mic
        let t = i as f32 / 48000.0;
        let s_mic = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.25; // -12 dBFS
        mic_samples.push(s_mic);
        mic_samples.push(s_mic);

        // 220 Hz music tone on desktop
        let s_desk = (2.0 * std::f32::consts::PI * 220.0 * t).sin() * 0.125; // -18 dBFS
        desktop_samples.push(s_desk);
        desktop_samples.push(s_desk);
    }

    mic_source.ingest_samples(&mic_samples, 0.0);
    desktop_source.ingest_samples(&desktop_samples, 0.0);

    mixer.add_source(mic_source);
    mixer.add_source(desktop_source);

    // Process first mix chunk
    let result = mixer.process_mix(chunk_pairs, 0.02);
    assert_eq!(result.master_samples.len(), chunk_pairs * 2);

    // Verify live meters on sources and master
    let mic = mixer.get_source(mic_id).unwrap();
    let desk = mixer.get_source(desktop_id).unwrap();

    println!(
        "[M6 Acceptance] Initial Mix Meters - Mic: Peak {:.1} dBFS / RMS {:.1} dBFS | Desktop: Peak {:.1} dBFS / RMS {:.1} dBFS | Master: Peak {:.1} dBFS / LUFS {:.1}",
        mic.meter.peak_db[0],
        mic.meter.rms_db[0],
        desk.meter.peak_db[0],
        desk.meter.rms_db[0],
        mixer.master_meter.peak_db[0],
        mixer.master_meter.lufs
    );

    assert!(mic.meter.peak_db[0] > -30.0 && mic.meter.peak_db[0] <= 0.0);
    assert!(desk.meter.peak_db[0] > -35.0 && desk.meter.peak_db[0] <= 0.0);
    assert!(mixer.master_meter.peak_db[0] > -25.0);

    // Test Master Limiter Ceiling Safeguard with extreme signal (+20 dBFS input)
    let hot_samples = vec![10.0f32; chunk_pairs * 2];
    mixer
        .get_source_mut(mic_id)
        .unwrap()
        .ingest_samples(&hot_samples, 0.04);
    let hot_result = mixer.process_mix(chunk_pairs, 0.04);

    let ceiling_linear = AudioMeter::db_to_linear(-1.0);
    for s in &hot_result.master_samples {
        assert!(
            s.abs() <= ceiling_linear + 1e-4,
            "Master limiter ceiling breached: {}",
            s
        );
    }
}

#[test]
fn test_m6_thirty_minute_audio_clock_drift_under_20ms() {
    let mut mixer = AudioMixer::new();

    let mic_id = SourceId::new();
    let desktop_id = SourceId::new();

    let mic_source = AudioSource::new(mic_id, "Microphone Stream", 48000);
    let desktop_source = AudioSource::new(desktop_id, "Desktop Stream", 48000);

    mixer.add_source(mic_source);
    mixer.add_source(desktop_source);

    // 30 minutes of broadcast audio = 1,800 seconds.
    // At 20 ms per chunk (960 sample pairs = 1,920 floats):
    // 1,800 s / 0.020 s = 90,000 chunks.
    let total_chunks = 90_000;
    let chunk_duration_sec = 0.020f64; // 20 ms
    let chunk_pairs = 960usize;

    // Simulate real hardware clock skew: sound card crystal running +40 ppm fast
    // (without drift correction, +40 ppm over 1800 s accumulates to 72 ms of drift!)
    let hardware_skew_factor = 1.0 + 0.000040; // +40 ppm

    let mut max_observed_drift_mic: f64 = 0.0;
    let mut max_observed_drift_desk: f64 = 0.0;

    let test_start = Instant::now();
    let base_chunk = vec![0.1f32; chunk_pairs * 2];

    for chunk_idx in 0..total_chunks {
        let master_video_time = chunk_idx as f64 * chunk_duration_sec;
        // Ingest slightly faster hardware timestamp to simulate clock skew
        let skewed_audio_time = master_video_time * hardware_skew_factor;

        mixer
            .get_source_mut(mic_id)
            .unwrap()
            .ingest_samples(&base_chunk, skewed_audio_time);

        mixer
            .get_source_mut(desktop_id)
            .unwrap()
            .ingest_samples(&base_chunk, skewed_audio_time);

        let _ = mixer.process_mix(chunk_pairs, master_video_time);

        // Periodically verify drift metrics (every 5 minutes = every 15,000 chunks)
        if chunk_idx % 15_000 == 0 || chunk_idx == total_chunks - 1 {
            let mic_drift = mixer
                .get_source(mic_id)
                .unwrap()
                .drift_corrector
                .current_drift_seconds()
                .abs();

            let desk_drift = mixer
                .get_source(desktop_id)
                .unwrap()
                .drift_corrector
                .current_drift_seconds()
                .abs();

            if mic_drift > max_observed_drift_mic {
                max_observed_drift_mic = mic_drift;
            }
            if desk_drift > max_observed_drift_desk {
                max_observed_drift_desk = desk_drift;
            }

            println!(
                "[M6 Acceptance] 30-Min Drift Check at {:.1} min: Mic drift = {:.3} ms, Desktop drift = {:.3} ms (Limit: < 20.0 ms)",
                master_video_time / 60.0,
                mic_drift * 1000.0,
                desk_drift * 1000.0
            );

            // Strict acceptance criterion: drift must remain under 20 ms
            assert!(
                mic_drift < 0.020,
                "Microphone drift exceeded 20ms at minute {:.1}: was {:.3} ms",
                master_video_time / 60.0,
                mic_drift * 1000.0
            );
            assert!(
                desk_drift < 0.020,
                "Desktop drift exceeded 20ms at minute {:.1}: was {:.3} ms",
                master_video_time / 60.0,
                desk_drift * 1000.0
            );
        }
    }

    let elapsed = test_start.elapsed();
    println!(
        "[M6 Acceptance] 30-Minute Simulation Completed: 90,000 chunks (1,800 s broadcast time) simulated in {:.2} s. Max mic drift: {:.3} ms, Max desk drift: {:.3} ms",
        elapsed.as_secs_f64(),
        max_observed_drift_mic * 1000.0,
        max_observed_drift_desk * 1000.0
    );

    assert!(
        max_observed_drift_mic < 0.020,
        "Max mic drift exceeded 20 ms: {:.3} ms",
        max_observed_drift_mic * 1000.0
    );
    assert!(
        max_observed_drift_desk < 0.020,
        "Max desk drift exceeded 20 ms: {:.3} ms",
        max_observed_drift_desk * 1000.0
    );
}
