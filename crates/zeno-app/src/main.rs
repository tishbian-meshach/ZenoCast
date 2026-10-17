//! # Zenocast Desktop Application
//!
//! Main entry point for the Zenocast live esports production studio and motion-graphics suite.

use eframe::{run_native, NativeOptions};
use egui::vec2;
use zeno_ui::ZenocastApp;

fn main() -> eframe::Result<()> {
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
