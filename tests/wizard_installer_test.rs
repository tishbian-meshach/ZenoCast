//! Milestone M20 Acceptance Tests: Installer & First-Run Wizard.
//!
//! Acceptance criteria (Section 13):
//! "Clean Windows VM installs and streams within 5 minutes."
//!
//! Verifies:
//! 1. Hardware device auto-detection across displays, webcams, microphones, and speakers.
//! 2. Hardware video encoder capability probe and performance benchmark.
//! 3. Bitrate and canvas resolution recommendation (1080p60 vs 720p60).
//! 4. First-run wizard state progression, UI interaction, and settings persistence.
//! 5. Remote software update verification and semver auditing.
//! 6. WiX installer configuration (`wix/main.wxs`) packaging zenocast binary, shortcuts, and FFmpeg DLLs.

use std::fs;
use std::path::Path;

use zeno_ui::settings::StudioSettings;
use zeno_ui::wizard::{
    DestinationSetup, EncoderRecommendation, FirstRunWizard, HardwareInventory, UpdateChecker,
    WizardStep,
};

#[test]
fn test_m20_hardware_device_detection() {
    let inventory = HardwareInventory::scan();

    // 1. Displays: At least one display must be detected
    assert!(
        !inventory.displays.is_empty(),
        "Expected at least 1 display monitor"
    );
    let primary_display = inventory.displays.iter().find(|d| d.is_primary);
    assert!(
        primary_display.is_some(),
        "Primary display monitor should be identified"
    );
    let disp = primary_display.unwrap();
    assert!(
        disp.width >= 1280 && disp.height >= 720,
        "Broadcast display should support at least 720p/1080p resolution"
    );

    // 2. Webcams / Cameras: Must have at least 1 camera capture device
    assert!(
        !inventory.cameras.is_empty(),
        "Expected at least 1 camera capture device"
    );
    let cam = &inventory.cameras[0];
    assert!(!cam.name.is_empty());
    assert!(cam.max_width > 0 && cam.max_height > 0);

    // 3. Audio Ingest & Output
    assert!(
        !inventory.microphones.is_empty(),
        "Expected at least 1 audio capture microphone"
    );
    assert!(
        !inventory.speakers.is_empty(),
        "Expected at least 1 audio playback / monitor device"
    );

    println!(
        "[M20 Acceptance] Hardware Scan: {} displays, {} cameras, {} microphones, {} speakers",
        inventory.displays.len(),
        inventory.cameras.len(),
        inventory.microphones.len(),
        inventory.speakers.len()
    );
}

#[test]
fn test_m20_encoder_benchmark_and_bitrate_suggestion() {
    let recommendation = EncoderRecommendation::benchmark_and_recommend();

    assert!(
        !recommendation.encoder_label.is_empty(),
        "Encoder name must be resolved"
    );
    assert!(
        recommendation.benchmark_fps > 0.0,
        "Encoder throughput must be measured"
    );

    if recommendation.is_hardware {
        // Hardware encoder (NVENC, AMF, QSV) recommends 1080p60 at 6,000 kbps
        assert_eq!(recommendation.output_width, 1920);
        assert_eq!(recommendation.output_height, 1080);
        assert_eq!(recommendation.output_fps, 60.0);
        assert!(recommendation.video_bitrate_kbps >= 6000);
        println!(
            "[M20 Acceptance] Hardware Encoder Detected: {} (Benchmark: {:.1} FPS, 1080p60 @ {} kbps)",
            recommendation.encoder_label, recommendation.benchmark_fps, recommendation.video_bitrate_kbps
        );
    } else {
        // Software fallback recommends 720p60 at 4,500 kbps for CPU safety
        assert_eq!(recommendation.output_width, 1280);
        assert_eq!(recommendation.output_height, 720);
        assert_eq!(recommendation.output_fps, 60.0);
        assert_eq!(recommendation.video_bitrate_kbps, 4500);
        println!(
            "[M20 Acceptance] Software Encoder Detected: {} (Benchmark: {:.1} FPS, 720p60 @ {} kbps)",
            recommendation.encoder_label, recommendation.benchmark_fps, recommendation.video_bitrate_kbps
        );
    }
}

#[test]
fn test_m20_first_run_wizard_state_progression_and_apply() {
    let mut wizard = FirstRunWizard::new();
    assert_eq!(wizard.step, WizardStep::Welcome);
    assert!(wizard.is_open);
    assert!(!wizard.is_completed);

    // Step 1: Welcome -> Step 2: Hardware
    wizard.next();
    assert_eq!(wizard.step, WizardStep::Hardware);

    // Step 2: Hardware -> Step 3: Encoder Benchmark
    wizard.next();
    assert_eq!(wizard.step, WizardStep::EncoderBenchmark);

    // Adjust bitrate preference in benchmark step
    wizard.recommendation.video_bitrate_kbps = 7500;

    // Step 3: Encoder Benchmark -> Step 4: Destination
    wizard.next();
    assert_eq!(wizard.step, WizardStep::Destination);

    // Configure Twitch streaming destination
    wizard.destination = DestinationSetup {
        service_name: "Twitch".into(),
        server_url: "rtmp://live.twitch.tv/app".into(),
        stream_key: "live_user_test_token_987".into(),
        srt_latency_ms: 120,
    };

    // Step 4: Destination -> Step 5: Summary
    wizard.next();
    assert_eq!(wizard.step, WizardStep::Summary);

    // Test back button functionality
    wizard.back();
    assert_eq!(wizard.step, WizardStep::Destination);
    wizard.next();
    assert_eq!(wizard.step, WizardStep::Summary);

    // Apply configuration to studio settings
    let mut settings = StudioSettings::default();
    wizard.apply_to_settings(&mut settings);

    // Verify settings applied correctly
    assert_eq!(
        settings.video.output_width,
        wizard.recommendation.output_width
    );
    assert_eq!(
        settings.video.output_height,
        wizard.recommendation.output_height
    );
    assert_eq!(settings.video.fps, wizard.recommendation.output_fps);
    assert_eq!(
        settings.output.encoder_kind,
        wizard.recommendation.encoder_label
    );
    assert_eq!(settings.output.video_bitrate_kbps, 7500);
    assert_eq!(settings.output.stream_service, "Twitch");
    assert_eq!(settings.output.stream_url, "rtmp://live.twitch.tv/app");
    assert_eq!(settings.output.stream_key, "live_user_test_token_987");
}

#[test]
fn test_m20_update_checker_semver_and_feed() {
    // 1. Semver Comparison Tests
    assert!(UpdateChecker::is_newer("0.1.0", "0.2.0"));
    assert!(UpdateChecker::is_newer("0.1.0", "0.1.1"));
    assert!(UpdateChecker::is_newer("0.9.9", "1.0.0"));
    assert!(UpdateChecker::is_newer("v0.1.0", "v0.1.1"));

    assert!(!UpdateChecker::is_newer("0.2.0", "0.1.0"));
    assert!(!UpdateChecker::is_newer("0.1.0", "0.1.0"));
    assert!(!UpdateChecker::is_newer("1.0.0", "0.9.9"));

    // 2. Feed Payload Parsing
    let feed_json = r#"{
        "version": "0.2.0",
        "download_url": "https://zenocast.live/downloads/Zenocast-0.2.0-x64.msi",
        "release_notes": "Added glTF 3D mesh layers and automated setup wizard."
    }"#;

    let update = UpdateChecker::check_feed(feed_json, "0.1.0").expect("Update feed parse failed");
    assert_eq!(update.current_version, "0.1.0");
    assert_eq!(update.latest_version, "0.2.0");
    assert!(update.update_available);
    assert_eq!(
        update.download_url,
        "https://zenocast.live/downloads/Zenocast-0.2.0-x64.msi"
    );
    assert!(update.release_notes.contains("glTF 3D mesh layers"));

    // Test when already up to date
    let current_feed = r#"{ "version": "0.1.0" }"#;
    let up_to_date = UpdateChecker::check_feed(current_feed, "0.1.0").unwrap();
    assert!(!up_to_date.update_available);
}

#[test]
fn test_m20_wix_installer_xml_structure() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let wix_file = if manifest_dir.join("wix").join("main.wxs").exists() {
        manifest_dir.join("wix").join("main.wxs")
    } else {
        manifest_dir
            .join("..")
            .join("..")
            .join("wix")
            .join("main.wxs")
    };

    assert!(
        wix_file.exists(),
        "Wix installer configuration file wix/main.wxs must exist at {:?}",
        wix_file
    );

    let content = fs::read_to_string(&wix_file).expect("Failed to read wix/main.wxs");

    // Validate WiX 3 XML structure and attributes
    assert!(content.contains(r#"<Wix xmlns="http://schemas.microsoft.com/wix/2006/wi">"#));
    assert!(content.contains(r#"UpgradeCode="B83C5642-124E-4BC3-A88E-32529C52E521""#));
    assert!(content.contains(r#"Name="zenocast.exe""#));
    assert!(content.contains(r#"Platform="x64""#));

    // Validate bundled FFmpeg DLL components
    assert!(content.contains("avcodec-61.dll"));
    assert!(content.contains("avformat-61.dll"));
    assert!(content.contains("avutil-59.dll"));
    assert!(content.contains("swresample-5.dll"));
    assert!(content.contains("swscale-8.dll"));

    // Validate shortcuts and feature tree
    assert!(content.contains(r#"Id="sc_desktop_zenocast""#));
    assert!(content.contains(r#"Id="sc_startmenu_zenocast""#));
    assert!(content.contains(r#"Id="MainApplication""#));
}
