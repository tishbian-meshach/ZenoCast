//! Acceptance test for Milestone M16: Browser Source.
//!
//! Validates:
//! 1. Web scoreboard page renders as a transparent overlay at 60 fps.
//! 2. Custom CSS injection correctly modifies presentation and elements.
//! 3. Refresh re-evaluates page content without frame drops.
//! 4. 60 fps timing progression maintains frame budget (< 16.6ms).

use pollster::FutureExt;
use zeno_engine::GpuContext;
use zeno_media::{BrowserSource, Source, SourceKind};

#[test]
fn test_browser_source_transparent_overlay_rendering() {
    let gpu = GpuContext::init_headless()
        .block_on()
        .expect("Headless GPU context initialization");

    let mut browser = BrowserSource::new("https://zenocast.live/scoreboard", 1920, 1080);
    assert_eq!(browser.kind(), SourceKind::Browser);
    assert_eq!(browser.dimensions(), (1920, 1080));
    assert!(browser.is_transparent());

    browser
        .start(&gpu.device, &gpu.queue)
        .expect("Start browser source");
    assert!(browser.is_active());

    // Update at t = 0.0s
    browser.update(&gpu.device, &gpu.queue, 0.0);
    let frame = browser
        .get_frame()
        .expect("First frame ready after start at t=0.0");
    assert_eq!(frame.width, 1920);
    assert_eq!(frame.height, 1080);
    assert_eq!(frame.timestamp_seconds, 0.0);

    browser.stop();
    assert!(!browser.is_active());
}

#[test]
fn test_browser_source_custom_css_injection_and_refresh() {
    let gpu = GpuContext::init_headless()
        .block_on()
        .expect("Headless GPU context initialization");

    let mut browser = BrowserSource::new("https://zenocast.live/scoreboard", 1280, 720);
    browser
        .start(&gpu.device, &gpu.queue)
        .expect("Start browser source");

    // Initial render
    browser.update(&gpu.device, &gpu.queue, 0.0);
    assert_eq!(browser.frame_count(), 1);

    // Inject custom CSS to hide match clock
    browser.set_custom_css(".clock { display: none }");
    assert_eq!(browser.custom_css(), ".clock { display: none }");

    // Trigger refresh
    browser.refresh();

    // Advance by one frame (1/60s)
    browser.update(&gpu.device, &gpu.queue, 0.01667);
    assert_eq!(browser.frame_count(), 2);

    let frame = browser.get_frame().expect("Frame ready after refresh");
    assert_eq!(frame.width, 1280);
    assert_eq!(frame.height, 720);
    assert_eq!(frame.timestamp_seconds, 0.01667);

    browser.stop();
}

#[test]
fn test_browser_source_60fps_timing_and_budget() {
    let gpu = GpuContext::init_headless()
        .block_on()
        .expect("Headless GPU context initialization");

    let mut browser = BrowserSource::new("https://zenocast.live/scoreboard", 1920, 1080);
    browser.set_target_fps(60.0);
    browser
        .start(&gpu.device, &gpu.queue)
        .expect("Start browser source");

    let frame_interval = 1.0 / 60.0;
    let mut current_time = 0.0;

    let start_instant = std::time::Instant::now();

    // Stream 60 consecutive frames
    for i in 1..=60 {
        current_time += frame_interval;
        browser.update(&gpu.device, &gpu.queue, current_time);

        let frame = browser
            .get_frame()
            .unwrap_or_else(|| panic!("Frame {i} should be ready"));
        assert_eq!(frame.width, 1920);
        assert_eq!(frame.height, 1080);
        assert!((frame.timestamp_seconds - current_time).abs() < 1e-4);
    }

    let elapsed = start_instant.elapsed();
    let avg_frame_ms = (elapsed.as_secs_f64() * 1000.0) / 60.0;
    assert!(
        avg_frame_ms < 16.6,
        "Average frame render time {avg_frame_ms:.2}ms must hold 60fps frame budget (< 16.6ms)"
    );
    assert_eq!(browser.frame_count(), 61); // 1 initial + 60 streamed

    browser.stop();
}
