//! 60 fps Window Capture and Camera Capture standalone demo.

use pollster::FutureExt;
use std::time::Instant;
use zeno_engine::GpuContext;
use zeno_media::{CameraCaptureSource, DeviceManager, Source, WindowCaptureSource};

fn main() {
    println!("=== Zenocast M4 Capture Sources Demo ===");

    let manager = DeviceManager::new();
    println!("Enumerating attached hardware devices...");
    let displays = manager.list_displays();
    let windows = manager.list_windows();
    let cameras = manager.list_cameras();

    println!("Detected displays:");
    for d in &displays {
        println!(
            " - {} ({}x{} at {},{})",
            d.name, d.bounds.2, d.bounds.3, d.bounds.0, d.bounds.1
        );
    }

    println!("Detected application windows ({} total):", windows.len());
    for w in windows.iter().take(5) {
        println!(" - HWND 0x{:X}: \"{}\" ({})", w.hwnd, w.title, w.class_name);
    }

    println!("Detected camera devices:");
    for c in &cameras {
        println!(" - {} [ID: {}]", c.name, c.id);
    }

    let gpu = GpuContext::init_headless()
        .block_on()
        .expect("Headless GPU context initialization");

    let mut win_src = WindowCaptureSource::default_or_fallback();
    let mut cam_src = CameraCaptureSource::default_or_fallback();

    win_src.set_target_fps(60.0);
    win_src.start(&gpu.device, &gpu.queue).unwrap();
    cam_src.start(&gpu.device, &gpu.queue).unwrap();

    println!("Streaming at 60 fps for 120 frames (~2 seconds)...");
    let start_time = Instant::now();

    for i in 0..120 {
        let t = i as f64 / 60.0;
        win_src.update(&gpu.device, &gpu.queue, t);
        cam_src.update(&gpu.device, &gpu.queue, t);

        if i % 30 == 0 {
            if let Some(wf) = win_src.get_frame() {
                println!(
                    "Frame #{:3}: Window {}x{} @ {:.3}s",
                    i, wf.width, wf.height, wf.timestamp_seconds
                );
            }
            if let Some(cf) = cam_src.get_frame() {
                println!(
                    "Frame #{:3}: Camera {}x{} @ {:.3}s",
                    i, cf.width, cf.height, cf.timestamp_seconds
                );
            }
        }
    }

    let elapsed = start_time.elapsed();
    println!(
        "Completed 120 frames in {:.2}ms (avg {:.2}ms per pair)",
        elapsed.as_millis(),
        elapsed.as_millis() as f64 / 120.0
    );

    win_src.stop();
    cam_src.stop();
    println!("Demo finished successfully.");
}
