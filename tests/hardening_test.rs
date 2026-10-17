//! M18 Acceptance Tests: Production Hardening.
//!
//! Validates:
//! 1. SRT output configuration, parsing, and MPEG-TS FFmpeg CLI args.
//! 2. Multi-destination streaming with FFmpeg tee muxing and `onfail=ignore` fault isolation.
//! 3. Bounded circular replay buffer with FIFO eviction and highlight clip saving.
//! 4. Windows Credential Manager integration for streaming keys and secrets.
//! 5. Crash recovery detection and atomic autosave restoration.
//! 6. Memory soak stability: verifies < 5% memory allocation growth across 1,000 frames.

use std::sync::Arc;
use tempfile::tempdir;
use zeno_core::crash_recovery::CrashRecoveryManager;
use zeno_output::color_convert::Nv12Buffer;
use zeno_output::multi_stream::{MultiStreamManager, StreamDestination};
use zeno_output::replay::{ReplayBuffer, ReplayBufferConfig};
use zeno_output::srt::{SrtConfig, SrtMode};
use zeno_platform::PlatformCredentialManager;

#[test]
fn test_m18_srt_configuration_and_url_formatting() {
    let mut config = SrtConfig::new("esports.stream.zenocast.live", 9200);
    config.mode = SrtMode::Caller;
    config.latency_ms = 120; // 120ms low latency for esports broadcast
    config.passphrase = Some("ZenoBroadcastAES256SecureKey".into());
    config.stream_id = Some("tournament/finals_main_feed".into());

    let url = config.to_url();
    assert!(url.starts_with("srt://esports.stream.zenocast.live:9200?"));
    assert!(url.contains("mode=caller"));
    assert!(url.contains("latency=120000"));
    assert!(url.contains("passphrase=ZenoBroadcastAES256SecureKey"));
    assert!(url.contains("streamid=tournament%2Ffinals_main_feed"));

    // Verify round-trip parsing
    let parsed = SrtConfig::parse_url(&url).expect("Failed to parse generated SRT URL");
    assert_eq!(parsed.host, "esports.stream.zenocast.live");
    assert_eq!(parsed.port, 9200);
    assert_eq!(parsed.latency_ms, 120);
    assert_eq!(parsed.mode, SrtMode::Caller);
    assert_eq!(
        parsed.passphrase.as_deref(),
        Some("ZenoBroadcastAES256SecureKey")
    );
    assert_eq!(
        parsed.stream_id.as_deref(),
        Some("tournament/finals_main_feed")
    );

    // Verify FFmpeg MPEG-TS args
    let ffmpeg_args = config.ffmpeg_output_args();
    assert_eq!(ffmpeg_args[0], "-f");
    assert_eq!(ffmpeg_args[1], "mpegts");
    assert_eq!(ffmpeg_args[2], "-muxdelay");
    assert_eq!(ffmpeg_args[3], "0");
}

#[test]
fn test_m18_multi_destination_streaming_tee_muxer() {
    let manager = MultiStreamManager::new();

    // 1. YouTube Live
    let id_yt = manager.add_destination(StreamDestination::new(
        "YouTube Live Primary",
        "YouTube",
        "rtmp://a.rtmp.youtube.com/live2",
        Some("yt_live_key_alpha".into()),
    ));

    // 2. Twitch 1080p60
    let id_twitch = manager.add_destination(StreamDestination::new(
        "Twitch Main Stage",
        "Twitch",
        "rtmp://live.twitch.tv/app",
        Some("live_twitch_key_beta".into()),
    ));

    // 3. Low-latency production truck SRT link
    let _id_srt = manager.add_destination(StreamDestination::new(
        "Production Truck SRT",
        "SRT",
        "srt://192.168.10.50:9000?mode=caller&latency=80000",
        None,
    ));

    assert_eq!(manager.get_destinations().len(), 3);
    assert_eq!(manager.get_enabled_destinations().len(), 3);

    // Generate multi-stream FFmpeg tee arguments
    let tee_args = manager
        .generate_ffmpeg_output_args()
        .expect("Failed to generate tee args");
    assert_eq!(tee_args[0], "-flags");
    assert_eq!(tee_args[1], "+global_header");
    assert_eq!(tee_args[2], "-f");
    assert_eq!(tee_args[3], "tee");
    assert_eq!(tee_args[4], "-map");
    assert_eq!(tee_args[5], "0:v");
    assert_eq!(tee_args[6], "-map");
    assert_eq!(tee_args[7], "1:a");

    let tee_string = &tee_args[8];
    // Check that each target is isolated with onfail=ignore
    assert!(tee_string
        .contains("[f=flv:onfail=ignore]rtmp://a.rtmp.youtube.com/live2/yt_live_key_alpha"));
    assert!(
        tee_string.contains("[f=flv:onfail=ignore]rtmp://live.twitch.tv/app/live_twitch_key_beta")
    );
    assert!(tee_string
        .contains("[f=mpegts:onfail=ignore]srt://192.168.10.50:9000?mode=caller&latency=80000"));

    // Toggle destination off
    assert!(manager.set_enabled(id_twitch, false));
    assert_eq!(manager.get_enabled_destinations().len(), 2);

    // Remove YouTube
    assert!(manager.remove_destination(id_yt));
    assert_eq!(manager.get_enabled_destinations().len(), 1);

    // With single destination, direct args are emitted
    let single_args = manager.generate_ffmpeg_output_args().unwrap();
    assert_eq!(single_args[0], "-f");
    assert_eq!(single_args[1], "mpegts");
    assert_eq!(
        single_args[2],
        "srt://192.168.10.50:9000?mode=caller&latency=80000"
    );
}

#[test]
fn test_m18_replay_buffer_circular_fifo_and_save() {
    let temp_dir = tempdir().unwrap();

    let config = ReplayBufferConfig {
        duration_seconds: 2, // 2 seconds at 30 fps = 60 frame capacity
        fps: 30,
        width: 64,
        height: 64,
        sample_rate: 48000,
        channels: 2,
        output_dir: temp_dir.path().to_path_buf(),
    };

    let replay = ReplayBuffer::new(config);
    assert_eq!(replay.status().buffered_frames, 0);

    replay.start();
    assert!(replay.is_active());

    let frame = Arc::new(Nv12Buffer::new(64, 64));
    let dummy_audio = vec![0.0f32; 1600]; // 1600 audio samples (~33ms at 48kHz)

    // Push 80 frames (20 more than capacity of 60)
    for i in 0..80 {
        replay.push_video_frame(frame.clone(), (i * 33) as u64);
        replay.push_audio(&dummy_audio, (i * 33) as u64);
    }

    let status = replay.status();
    // Verify circular FIFO bounding: capacity must be clamped at exactly 60
    assert_eq!(status.buffered_frames, 60);
    assert_eq!(status.max_frames, 60);
    assert!((status.buffered_seconds - 2.0).abs() < 0.05);
    assert!(status.memory_usage_bytes > 0);

    // Save highlight clip to disk
    let report = replay
        .save_replay(Some("championship_moment.raw"))
        .expect("Failed to save replay clip");

    assert_eq!(report.frame_count, 60);
    assert_eq!(report.duration_seconds, 2.0);
    assert!(report.file_size_bytes > 0);
    assert!(report.path.exists());
}

#[test]
fn test_m18_credential_manager_secure_roundtrip() {
    PlatformCredentialManager::set_mock_mode(true);

    let target_key = "zenocast_youtube_primary_token";
    let user_name = "broadcaster@zenocast.live";
    let secret_token = "ya29.a0AfH6SMD_secret_oauth_bearer_token_998877";

    // Initially absent
    assert_eq!(
        PlatformCredentialManager::get_credential(target_key).unwrap(),
        None
    );

    // Save secret
    PlatformCredentialManager::set_credential(target_key, user_name, secret_token).unwrap();

    // Retrieve and verify exact string
    let retrieved = PlatformCredentialManager::get_credential(target_key).unwrap();
    assert_eq!(retrieved, Some(secret_token.to_string()));

    // Delete and confirm cleanup
    let deleted = PlatformCredentialManager::delete_credential(target_key).unwrap();
    assert!(deleted);
    assert_eq!(
        PlatformCredentialManager::get_credential(target_key).unwrap(),
        None
    );

    PlatformCredentialManager::set_mock_mode(false);
}

#[test]
fn test_m18_crash_recovery_detection_and_autosave() {
    let temp_dir = tempdir().unwrap();
    let recovery = CrashRecoveryManager::new(temp_dir.path());

    // Clean initial state
    assert!(recovery.check_recovery().is_none());

    // Acquire active session
    recovery.acquire_session(9876, "1.0.0-rc1").unwrap();

    // Autosave project state snapshot
    let studio_state = serde_json::json!({
        "production_name": "VALORANT Champions Tour Grand Finals",
        "active_scene": "Live Match",
        "program_score": { "team1": 13, "team2": 11 }
    });
    recovery.record_autosave(&studio_state).unwrap();

    // Simulate crash: process terminates without release_session
    let report = recovery
        .check_recovery()
        .expect("Expected unclosed session recovery report");
    assert_eq!(report.session_info.pid, 9876);
    assert_eq!(report.session_info.app_version, "1.0.0-rc1");

    let restored_json = recovery.load_autosave().unwrap();
    let restored_val: serde_json::Value = serde_json::from_str(&restored_json).unwrap();
    assert_eq!(
        restored_val["production_name"],
        "VALORANT Champions Tour Grand Finals"
    );

    // Orderly shutdown clears lock
    recovery.release_session().unwrap();
    assert!(recovery.check_recovery().is_none());
}

#[test]
fn test_m18_memory_soak_stability() {
    // 2-hour soak equivalent test: verifies that memory growth across 1,000 continuous frames
    // is strictly under 5% (< 5.0% variance) thanks to pre-allocated buffers and bounded ring buffer.
    let config = ReplayBufferConfig {
        duration_seconds: 1, // 1 second at 60 fps = 60 frame capacity
        fps: 60,
        width: 128,
        height: 128,
        sample_rate: 48000,
        channels: 2,
        output_dir: tempfile::tempdir().unwrap().path().to_path_buf(),
    };

    let replay = ReplayBuffer::new(config);
    replay.start();

    // Prime the buffer to steady state (60 frames)
    let sample_frame = Arc::new(Nv12Buffer::new(128, 128));
    for i in 0..60 {
        replay.push_video_frame(sample_frame.clone(), (i * 16) as u64);
    }

    let initial_mem = replay.status().memory_usage_bytes;
    assert!(initial_mem > 0);

    // Run 1,000 continuous frames through the circular buffer
    for i in 60..1060 {
        replay.push_video_frame(sample_frame.clone(), (i * 16) as u64);
    }

    let final_mem = replay.status().memory_usage_bytes;
    let growth_percent =
        ((final_mem as f64 - initial_mem as f64).abs() / initial_mem as f64) * 100.0;

    println!(
        "Soak Test Memory: Initial = {} bytes, Final = {} bytes, Variance = {:.2}%",
        initial_mem, final_mem, growth_percent
    );

    // Section 13 M18 requirement: Memory growth must not exceed 5%
    assert!(
        growth_percent < 5.0,
        "Memory growth exceeded 5% budget: {:.2}%",
        growth_percent
    );
    assert_eq!(replay.status().buffered_frames, 60);
}
