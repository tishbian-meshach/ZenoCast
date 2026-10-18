//! # Zenocast Desktop Application
//!
//! Main entry point for the Zenocast live esports production studio and motion-graphics suite.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::{run_native, NativeOptions};
use egui::vec2;
use std::backtrace::Backtrace;
use std::fs::{create_dir_all, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use zeno_ui::ZenocastApp;

/// Resolves the application log directory (%LOCALAPPDATA%/Zenocast/logs/ on Windows).
pub fn get_log_dir() -> PathBuf {
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        PathBuf::from(local_app_data).join("Zenocast").join("logs")
    } else if let Some(home) = std::env::var_os("USERPROFILE") {
        PathBuf::from(home)
            .join("AppData")
            .join("Local")
            .join("Zenocast")
            .join("logs")
    } else {
        std::env::temp_dir().join("Zenocast").join("logs")
    }
}

/// Writes a structured crash report to a persistent file when a panic occurs.
pub fn write_panic_report(
    panic_message: &str,
    location: Option<&str>,
    backtrace: &Backtrace,
    log_dir: &Path,
) -> PathBuf {
    let _ = create_dir_all(log_dir);
    let log_path = log_dir.join("panic.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let now = std::time::SystemTime::now();
        let _ = writeln!(
            file,
            "==================== CRASH / PANIC REPORT ===================="
        );
        let _ = writeln!(file, "Timestamp: {:?}", now);
        if let Some(loc) = location {
            let _ = writeln!(file, "Location: {}", loc);
        }
        let _ = writeln!(file, "Panic details: {}", panic_message);
        let _ = writeln!(file, "Backtrace:\n{}", backtrace);
        let _ = writeln!(
            file,
            "=============================================================\n"
        );
    }
    log_path
}

/// Installs a persistent panic hook so headless release builds write diagnosable crash traces.
pub fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let log_dir = get_log_dir();
        let backtrace = Backtrace::force_capture();
        let msg = panic_info.to_string();
        let loc = panic_info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()));
        write_panic_report(&msg, loc.as_deref(), &backtrace, &log_dir);
        default_hook(panic_info);
    }));
}

fn main() -> eframe::Result<()> {
    install_panic_hook();

    let native_options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Zenocast Studio")
            .with_inner_size(vec2(1440.0, 900.0))
            .with_min_inner_size(vec2(960.0, 600.0)),
        ..Default::default()
    };

    run_native(
        "Zenocast Studio",
        native_options,
        Box::new(|cc| Ok(Box::new(ZenocastApp::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_panic_logging_produces_diagnosable_trace() {
        let dir = tempdir().unwrap();
        let backtrace = Backtrace::force_capture();
        let log_file = write_panic_report(
            "Explicit test panic: renderer out of memory",
            Some("crates/zeno-app/src/main.rs:42:1"),
            &backtrace,
            dir.path(),
        );

        assert!(log_file.exists());
        let content = std::fs::read_to_string(&log_file).unwrap();
        assert!(content.contains("CRASH / PANIC REPORT"));
        assert!(content.contains("renderer out of memory"));
        assert!(content.contains("crates/zeno-app/src/main.rs:42:1"));
    }

    #[test]
    fn test_log_dir_resolution() {
        let log_dir = get_log_dir();
        assert!(log_dir.ends_with("Zenocast\\logs") || log_dir.ends_with("Zenocast/logs"));
    }
}
