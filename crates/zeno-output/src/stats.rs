//! Live performance and broadcast output statistics.

use serde::{Deserialize, Serialize};

/// Live output statistics captured across encoding, streaming, and recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveOutputStats {
    /// Total number of video frames dispatched to the encoder.
    pub total_frames: u64,
    /// Number of video frames dropped due to network or encoder buffer congestion.
    pub dropped_frames: u64,
    /// Calculated dropped frame ratio (0.0 to 1.0).
    pub dropped_ratio: f32,
    /// Dropped frame percentage (0.0 to 100.0).
    pub dropped_percent: f32,
    /// Current real-time transmission / encoding bitrate in kilobits per second (kbps).
    pub current_bitrate_kbps: f32,
    /// Instantaneous frame encode latency in milliseconds.
    pub encode_time_ms: f32,
    /// Average compositor frame render latency in milliseconds.
    pub render_time_ms: f32,
    /// Estimated CPU usage percentage for output subsystem (0.0 to 100.0).
    pub cpu_usage_percent: f32,
    /// Estimated GPU usage percentage for encoding / color conversion (0.0 to 100.0).
    pub gpu_usage_percent: f32,
    /// Total bytes written to streaming or recording endpoints.
    pub total_bytes_written: u64,
    /// Active streaming duration in seconds.
    pub stream_duration_seconds: f64,
    /// Active recording duration in seconds.
    pub record_duration_seconds: f64,
    /// Whether streaming is currently active.
    pub is_streaming: bool,
    /// Whether recording is currently active.
    pub is_recording: bool,
    /// Reconnect attempt count if stream experienced disconnection.
    pub reconnect_attempts: u32,
}

impl Default for LiveOutputStats {
    fn default() -> Self {
        Self {
            total_frames: 0,
            dropped_frames: 0,
            dropped_ratio: 0.0,
            dropped_percent: 0.0,
            current_bitrate_kbps: 0.0,
            encode_time_ms: 0.0,
            render_time_ms: 0.0,
            cpu_usage_percent: 0.0,
            gpu_usage_percent: 0.0,
            total_bytes_written: 0,
            stream_duration_seconds: 0.0,
            record_duration_seconds: 0.0,
            is_streaming: false,
            is_recording: false,
            reconnect_attempts: 0,
        }
    }
}

impl LiveOutputStats {
    /// Creates a fresh statistics tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a submitted frame and updates dropped ratios.
    pub fn record_frame(&mut self, dropped: bool) {
        self.total_frames += 1;
        if dropped {
            self.dropped_frames += 1;
        }
        if self.total_frames > 0 {
            self.dropped_ratio = self.dropped_frames as f32 / self.total_frames as f32;
            self.dropped_percent = self.dropped_ratio * 100.0;
        }
    }

    /// Updates throughput and bitrate based on bytes written over an interval.
    pub fn update_bitrate(&mut self, bytes_interval: u64, dt_seconds: f32) {
        self.total_bytes_written += bytes_interval;
        if dt_seconds > 0.0 {
            // (bytes * 8 bits/byte) / (dt_seconds * 1000 bits/kbit)
            let bits = bytes_interval as f32 * 8.0;
            self.current_bitrate_kbps = bits / (dt_seconds * 1000.0);
        }
    }
}
