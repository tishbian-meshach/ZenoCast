//! Performance HUD diagnostics overlay for broadcast telemetry.
//!
//! Visual design adheres strictly to Section 7 & 12: flat surfaces,
//! zero color literals outside `theme.rs`.

use serde::{Deserialize, Serialize};

use crate::theme::{
    ACCENT, AMBER_WARN, BORDER_WIDTH, CORNER_RADIUS, GREEN_OK, GRID_UNIT, RED_LIVE, SURFACE_1,
    SURFACE_3, TEXT, TEXT_MUTED,
};

/// Live broadcast performance telemetry snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSnapshot {
    /// Rendered frames per second
    pub fps: f32,
    /// Overall frame duration in milliseconds
    pub frame_time_ms: f32,
    /// GPU compositor execution time in milliseconds (target: < 4.0 ms)
    pub compositor_time_ms: f32,
    /// Total broadcast frames rendered
    pub total_frames: u64,
    /// Dropped frame count
    pub dropped_frames: u64,
    /// Dropped frame percentage (target: < 0.1%)
    pub dropped_percent: f64,
    /// Video encoder bitrate in kilobits per second
    pub bitrate_kbps: u32,
    /// Audio mixer drift / latency in milliseconds (target: < 20 ms)
    pub audio_latency_ms: f32,
    /// Active replay buffer buffered duration in seconds
    pub replay_seconds: f64,
    /// Replay buffer memory consumption in megabytes
    pub replay_memory_mb: f32,
    /// Estimated process memory footprint in megabytes
    pub process_memory_mb: f32,
}

impl Default for PerformanceSnapshot {
    fn default() -> Self {
        Self {
            fps: 60.0,
            frame_time_ms: 16.6,
            compositor_time_ms: 3.2,
            total_frames: 3600,
            dropped_frames: 0,
            dropped_percent: 0.0,
            bitrate_kbps: 6000,
            audio_latency_ms: 8.5,
            replay_seconds: 30.0,
            replay_memory_mb: 240.0,
            process_memory_mb: 480.0,
        }
    }
}

/// Floating diagnostic HUD overlay.
#[derive(Default)]
pub struct PerformanceOverlay {
    pub is_visible: bool,
    pub snapshot: PerformanceSnapshot,
}

impl PerformanceOverlay {
    /// Creates a new performance overlay widget.
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates current diagnostic telemetry.
    pub fn update_snapshot(&mut self, snapshot: PerformanceSnapshot) {
        self.snapshot = snapshot;
    }

    /// Renders the performance diagnostic HUD overlay if visible.
    pub fn render(&mut self, ctx: &egui::Context) {
        if !self.is_visible {
            return;
        }

        egui::Window::new("PERFORMANCE TELEMETRY")
            .resizable(false)
            .collapsible(true)
            .frame(
                egui::Frame::new()
                    .fill(SURFACE_1)
                    .stroke(egui::Stroke::new(BORDER_WIDTH, SURFACE_3))
                    .corner_radius(egui::CornerRadius::same(CORNER_RADIUS as u8))
                    .inner_margin(egui::Margin::same(GRID_UNIT as i8)),
            )
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    // Row 1: FPS and Frame Time
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("FRAME RATE:")
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                        let fps_color = if self.snapshot.fps >= 58.0 {
                            GREEN_OK
                        } else if self.snapshot.fps >= 45.0 {
                            AMBER_WARN
                        } else {
                            RED_LIVE
                        };
                        ui.label(
                            egui::RichText::new(format!("{:.1} FPS", self.snapshot.fps))
                                .size(12.0)
                                .monospace()
                                .strong()
                                .color(fps_color),
                        );

                        ui.add_space(GRID_UNIT);

                        ui.label(
                            egui::RichText::new("TOTAL TIME:")
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                        ui.label(
                            egui::RichText::new(format!("{:.2} ms", self.snapshot.frame_time_ms))
                                .size(12.0)
                                .monospace()
                                .color(TEXT),
                        );
                    });

                    // Row 2: Compositor Frame Time (Section 2 budget: < 4ms)
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("GPU COMPOSITOR:")
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                        let comp_color = if self.snapshot.compositor_time_ms <= 4.0 {
                            GREEN_OK
                        } else if self.snapshot.compositor_time_ms <= 8.0 {
                            AMBER_WARN
                        } else {
                            RED_LIVE
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{:.2} ms (budget < 4 ms)",
                                self.snapshot.compositor_time_ms
                            ))
                            .size(12.0)
                            .monospace()
                            .strong()
                            .color(comp_color),
                        );
                    });

                    // Row 3: Dropped Frames (Section 2 budget: < 0.1%)
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("DROPPED FRAMES:")
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                        let drop_color = if self.snapshot.dropped_percent <= 0.1 {
                            GREEN_OK
                        } else {
                            RED_LIVE
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{} / {} ({:.2}%)",
                                self.snapshot.dropped_frames,
                                self.snapshot.total_frames,
                                self.snapshot.dropped_percent
                            ))
                            .size(12.0)
                            .monospace()
                            .strong()
                            .color(drop_color),
                        );
                    });

                    // Row 4: Bitrate and Audio Latency
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("ENCODER BITRATE:")
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                        ui.label(
                            egui::RichText::new(format!("{} kbps", self.snapshot.bitrate_kbps))
                                .size(12.0)
                                .monospace()
                                .color(ACCENT),
                        );

                        ui.add_space(GRID_UNIT);

                        ui.label(
                            egui::RichText::new("AUDIO DRIFT:")
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                        let audio_color = if self.snapshot.audio_latency_ms <= 20.0 {
                            GREEN_OK
                        } else {
                            AMBER_WARN
                        };
                        ui.label(
                            egui::RichText::new(format!(
                                "{:.1} ms",
                                self.snapshot.audio_latency_ms
                            ))
                            .size(12.0)
                            .monospace()
                            .color(audio_color),
                        );
                    });

                    // Row 5: Replay buffer and Memory
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("REPLAY BUFFER:")
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{:.1}s ({:.1} MB)",
                                self.snapshot.replay_seconds, self.snapshot.replay_memory_mb
                            ))
                            .size(12.0)
                            .monospace()
                            .color(TEXT),
                        );

                        ui.add_space(GRID_UNIT);

                        ui.label(
                            egui::RichText::new("MEMORY RSS:")
                                .size(12.0)
                                .color(TEXT_MUTED),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "{:.1} MB",
                                self.snapshot.process_memory_mb
                            ))
                            .size(12.0)
                            .monospace()
                            .color(TEXT),
                        );
                    });
                });
            });
    }
}
