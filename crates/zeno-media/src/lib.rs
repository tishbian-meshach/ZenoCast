//! # zeno-media
//!
//! Live video and frame capture sources: Windows Graphics Capture, Media Foundation cameras,
//! and physical monitor/window ingest pipelines.

pub mod camera_source;
pub mod device_manager;
pub mod display_source;
pub mod error;
pub mod source;
pub mod texture_buffer;
pub mod window_source;

// Re-exports
pub use camera_source::CameraCaptureSource;
pub use device_manager::{DeviceManager, HotplugEvent};
pub use display_source::DisplayCaptureSource;
pub use error::SourceError;
pub use source::{Source, SourceKind, VideoFrame};
pub use texture_buffer::DoubleBufferedTexture;
pub use window_source::WindowCaptureSource;

// Platform re-exports
pub use zeno_platform::{CameraDeviceInfo, CameraFormat, DisplayInfo, WindowInfo};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_enumeration() {
        let manager = DeviceManager::new();
        let displays = manager.list_displays();
        let windows = manager.list_windows();
        let cameras = manager.list_cameras();

        println!(
            "Enumerated: {} displays, {} windows, {} cameras",
            displays.len(),
            windows.len(),
            cameras.len()
        );

        // On any graphical Windows system, at least one display exists
        assert!(
            !displays.is_empty(),
            "Expected at least 1 connected display"
        );
    }

    #[test]
    fn test_hotplug_polling() {
        let mut manager = DeviceManager::new();
        let events = manager.poll_hotplug_events();
        // Since no physical device connected in 1 millisecond, initial events list is empty
        assert!(events.is_empty());
    }

    #[test]
    fn test_camera_capture_source_simulation_and_lifecycle() {
        use pollster::FutureExt;
        use zeno_engine::GpuContext;

        let gpu = GpuContext::init_headless()
            .block_on()
            .expect("Headless GPU context initialization");

        let mut source = CameraCaptureSource::default_or_fallback();
        assert_eq!(source.kind(), SourceKind::CameraCapture);
        assert!(!source.is_active());
        assert_eq!(source.dimensions(), (1920, 1080));

        source
            .start(&gpu.device, &gpu.queue)
            .expect("Start camera source");
        assert!(source.is_active());

        // Update frame at t = 0.0s
        source.update(&gpu.device, &gpu.queue, 0.0);
        let frame1 = source.get_frame().expect("Frame ready at t=0.0");
        assert_eq!(frame1.width, 1920);
        assert_eq!(frame1.height, 1080);
        assert_eq!(frame1.timestamp_seconds, 0.0);

        // Update frame at t = 0.020s (exceeding the 1/60s = 0.01667s frame interval)
        source.update(&gpu.device, &gpu.queue, 0.020);
        let frame2 = source.get_frame().expect("Frame ready at t=0.020");
        assert_eq!(frame2.timestamp_seconds, 0.020);

        source.stop();
        assert!(!source.is_active());
    }

    #[test]
    fn test_window_source_properties() {
        let mut window_source = WindowCaptureSource::new(0x1234, "Visual Studio Code");
        assert_eq!(window_source.name(), "Visual Studio Code");
        assert_eq!(window_source.hwnd(), 0x1234);
        assert_eq!(window_source.kind(), SourceKind::WindowCapture);

        window_source.set_capture_cursor(false);
        window_source.set_borderless(true);
        window_source.set_target_fps(60.0);
    }

    #[test]
    fn test_display_source_primary_initialization() {
        let display_res = DisplayCaptureSource::primary();
        assert!(
            display_res.is_ok(),
            "Primary display should be detected on Windows"
        );
        let display = display_res.unwrap();
        assert_eq!(display.kind(), SourceKind::DisplayCapture);
        let (w, h) = display.dimensions();
        assert!(w > 0 && h > 0);
    }
}
