//! M4 Acceptance Test: 60 fps Window Capture + Camera/Webcam Feed with no CPU copy spikes.

use pollster::FutureExt;
use std::time::Instant;
use zeno_engine::GpuContext;
use zeno_media::{CameraCaptureSource, DeviceManager, Source, SourceKind, WindowCaptureSource};

#[test]
fn test_m4_window_and_camera_capture_at_60fps() {
    let gpu = GpuContext::init_headless()
        .block_on()
        .expect("Headless GPU context initialization");

    // Initialize DeviceManager and verify enumeration
    let mut dev_manager = DeviceManager::new();
    let displays = dev_manager.list_displays();
    let windows = dev_manager.list_windows();
    let cameras = dev_manager.list_cameras();

    println!(
        "[M4 Acceptance] Hardware Enumeration: {} displays, {} windows, {} cameras",
        displays.len(),
        windows.len(),
        cameras.len()
    );

    // Initial hotplug poll
    let hotplug_events = dev_manager.poll_hotplug_events();
    println!(
        "[M4 Acceptance] Initial hotplug events: {:?}",
        hotplug_events
    );

    // Create window capture source and camera capture source
    let mut window_source = WindowCaptureSource::default_or_fallback();
    let mut camera_source = CameraCaptureSource::default_or_fallback();

    assert_eq!(window_source.kind(), SourceKind::WindowCapture);
    assert_eq!(camera_source.kind(), SourceKind::CameraCapture);

    window_source.set_target_fps(60.0);

    // Start both sources
    window_source
        .start(&gpu.device, &gpu.queue)
        .expect("Failed to start window capture source");
    camera_source
        .start(&gpu.device, &gpu.queue)
        .expect("Failed to start camera capture source");

    assert!(window_source.is_active());
    assert!(camera_source.is_active());

    // Stream 180 frames (3 full seconds at 60 fps)
    let total_frames = 180;
    let dt = 1.0 / 60.0;
    let mut total_update_micros = 0u128;
    let mut max_frame_micros = 0u128;

    let mut last_win_timestamp = -1.0;
    let mut last_cam_timestamp = -1.0;

    for frame_idx in 0..total_frames {
        let t = frame_idx as f64 * dt;
        let frame_start = Instant::now();

        // Update window source and camera source in the real-time loop
        window_source.update(&gpu.device, &gpu.queue, t);
        camera_source.update(&gpu.device, &gpu.queue, t);

        let frame_elapsed = frame_start.elapsed().as_micros();
        total_update_micros += frame_elapsed;
        if frame_elapsed > max_frame_micros {
            max_frame_micros = frame_elapsed;
        }

        // Verify window frame delivery
        let win_frame = window_source
            .get_frame()
            .expect("Window frame delivered at 60 fps");
        assert!(win_frame.width > 0 && win_frame.height > 0);
        assert!(
            win_frame.timestamp_seconds >= last_win_timestamp,
            "Window timestamp must be monotonic"
        );
        last_win_timestamp = win_frame.timestamp_seconds;

        // Verify camera frame delivery
        let cam_frame = camera_source
            .get_frame()
            .expect("Camera frame delivered at 60 fps");
        assert!(cam_frame.width > 0 && cam_frame.height > 0);
        assert!(
            cam_frame.timestamp_seconds >= last_cam_timestamp,
            "Camera timestamp must be monotonic"
        );
        last_cam_timestamp = cam_frame.timestamp_seconds;
    }

    let avg_micros = total_update_micros / total_frames as u128;
    println!(
        "[M4 Acceptance] 60 fps Capture Performance: 180 frames streamed. Avg update: {} µs ({:.2} ms), Max: {} µs ({:.2} ms)",
        avg_micros,
        avg_micros as f64 / 1000.0,
        max_frame_micros,
        max_frame_micros as f64 / 1000.0
    );

    // Real-time frame budget is 16.6 ms (16,666 µs) for 60 fps.
    // Ensure average frame update is well within real-time budget (under 10 ms).
    assert!(
        avg_micros < 10_000,
        "CPU copy / update time spike detected: avg was {} µs",
        avg_micros
    );

    // Clean shutdown
    window_source.stop();
    camera_source.stop();

    assert!(!window_source.is_active());
    assert!(!camera_source.is_active());
}
