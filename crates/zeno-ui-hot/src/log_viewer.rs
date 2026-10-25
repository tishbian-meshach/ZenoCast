//! In-memory log viewer panel and tracing event subscriber.
//!
//! Visual design adheres strictly to Section 7 & 12: flat surfaces,
//! zero color literals outside `theme.rs`.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;

use crate::theme::{
    ACCENT, AMBER_WARN, BORDER_WIDTH, CORNER_RADIUS, GRID_UNIT, RED_LIVE, SURFACE_0, SURFACE_3,
    TEXT, TEXT_MUTED,
};

/// Severity level for broadcast event log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogSeverity {
    Info,
    Warn,
    Error,
}

impl LogSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warn => "WARN",
            Self::Error => "ERROR",
        }
    }
}

/// A structured broadcast log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// Unix timestamp in seconds
    pub timestamp_sec: u64,
    /// Severity level
    pub severity: LogSeverity,
    /// Source subsystem or module name
    pub target: String,
    /// Log event message body
    pub message: String,
}

/// Thread-safe in-memory circular log ring buffer.
#[derive(Clone)]
pub struct LogRingBuffer {
    entries: Arc<RwLock<VecDeque<LogEntry>>>,
    max_entries: usize,
}

impl Default for LogRingBuffer {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl LogRingBuffer {
    /// Creates a log ring buffer with the given capacity.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(VecDeque::with_capacity(max_entries))),
            max_entries,
        }
    }

    /// Pushes a new log entry. Drops the oldest when at capacity.
    pub fn push(
        &self,
        severity: LogSeverity,
        target: impl Into<String>,
        message: impl Into<String>,
    ) {
        let timestamp_sec = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut ring = self.entries.write();
        if ring.len() >= self.max_entries {
            ring.pop_front();
        }

        ring.push_back(LogEntry {
            timestamp_sec,
            severity,
            target: target.into(),
            message: message.into(),
        });
    }

    /// Clears all stored entries.
    pub fn clear(&self) {
        self.entries.write().clear();
    }

    /// Retrieves all entries.
    pub fn entries(&self) -> Vec<LogEntry> {
        self.entries.read().iter().cloned().collect()
    }
}

/// UI state and renderer for the Studio Log Viewer panel.
pub struct LogViewerPanel {
    buffer: LogRingBuffer,
    filter_severity: Option<LogSeverity>,
    search_query: String,
    auto_scroll: bool,
}

impl LogViewerPanel {
    /// Creates a new log viewer panel connected to a shared log buffer.
    pub fn new(buffer: LogRingBuffer) -> Self {
        Self {
            buffer,
            filter_severity: None,
            search_query: String::new(),
            auto_scroll: true,
        }
    }

    /// Renders the log viewer panel in the specified UI container.
    pub fn render(&mut self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            // Header Toolbar
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("BROADCAST LOGS")
                        .size(13.0)
                        .strong()
                        .color(TEXT),
                );

                ui.separator();

                // Severity Filter Buttons
                if ui
                    .selectable_label(self.filter_severity.is_none(), "ALL")
                    .clicked()
                {
                    self.filter_severity = None;
                }
                if ui
                    .selectable_label(
                        self.filter_severity == Some(LogSeverity::Info),
                        egui::RichText::new("INFO").color(TEXT_MUTED),
                    )
                    .clicked()
                {
                    self.filter_severity = Some(LogSeverity::Info);
                }
                if ui
                    .selectable_label(
                        self.filter_severity == Some(LogSeverity::Warn),
                        egui::RichText::new("WARN").color(AMBER_WARN),
                    )
                    .clicked()
                {
                    self.filter_severity = Some(LogSeverity::Warn);
                }
                if ui
                    .selectable_label(
                        self.filter_severity == Some(LogSeverity::Error),
                        egui::RichText::new("ERROR").color(RED_LIVE),
                    )
                    .clicked()
                {
                    self.filter_severity = Some(LogSeverity::Error);
                }

                ui.separator();

                // Search query filter
                ui.add(
                    egui::TextEdit::singleline(&mut self.search_query)
                        .hint_text("Filter logs...")
                        .desired_width(140.0),
                );

                if ui.button("Clear").clicked() {
                    self.buffer.clear();
                }

                ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
            });

            ui.add_space(GRID_UNIT);

            // Log table container
            egui::Frame::new()
                .fill(SURFACE_0)
                .stroke(egui::Stroke::new(BORDER_WIDTH, SURFACE_3))
                .corner_radius(egui::CornerRadius::same(CORNER_RADIUS as u8))
                .inner_margin(egui::Margin::same(GRID_UNIT as i8))
                .show(ui, |ui| {
                    let entries = self.buffer.entries();
                    let query = self.search_query.to_lowercase();

                    let filtered: Vec<&LogEntry> = entries
                        .iter()
                        .filter(|e| {
                            if let Some(sev) = self.filter_severity {
                                if e.severity != sev {
                                    return false;
                                }
                            }
                            if !query.is_empty() {
                                let match_msg = e.message.to_lowercase().contains(&query);
                                let match_tgt = e.target.to_lowercase().contains(&query);
                                return match_msg || match_tgt;
                            }
                            true
                        })
                        .collect();

                    egui::ScrollArea::vertical()
                        .max_height(240.0)
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if filtered.is_empty() {
                                ui.label(
                                    egui::RichText::new("No log events recorded.")
                                        .color(TEXT_MUTED)
                                        .size(12.0),
                                );
                            } else {
                                for entry in &filtered {
                                    ui.horizontal(|ui| {
                                        // Timestamp (monospace)
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:02}:{:02}:{:02}",
                                                (entry.timestamp_sec / 3600) % 24,
                                                (entry.timestamp_sec / 60) % 60,
                                                entry.timestamp_sec % 60
                                            ))
                                            .monospace()
                                            .color(TEXT_MUTED)
                                            .size(12.0),
                                        );

                                        // Badge
                                        let (badge_color, label) = match entry.severity {
                                            LogSeverity::Info => (TEXT_MUTED, "[INFO]"),
                                            LogSeverity::Warn => (AMBER_WARN, "[WARN]"),
                                            LogSeverity::Error => (RED_LIVE, "[ERROR]"),
                                        };

                                        ui.label(
                                            egui::RichText::new(label)
                                                .color(badge_color)
                                                .strong()
                                                .size(12.0),
                                        );

                                        // Target
                                        ui.label(
                                            egui::RichText::new(&entry.target)
                                                .color(ACCENT)
                                                .size(12.0),
                                        );

                                        // Message
                                        ui.label(
                                            egui::RichText::new(&entry.message)
                                                .color(TEXT)
                                                .size(12.0),
                                        );
                                    });
                                }
                            }
                        });
                });
        });
    }
}
