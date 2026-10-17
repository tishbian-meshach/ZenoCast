//! Multi-destination streaming management and multiplexing pipeline.
//!
//! Enables simultaneous broadcasting to multiple streaming platforms (e.g. YouTube, Twitch, Kick, SRT endpoints)
//! from a single hardware video encoding pass using FFmpeg's tee multiplexer with `onfail=ignore` fault isolation.

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{OutputError, OutputResult};
use crate::stats::LiveOutputStats;

/// An individual streaming destination configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamDestination {
    /// Unique identifier for this destination
    pub id: Uuid,
    /// Human-readable label (e.g. "YouTube Main Feed", "Twitch 1080p60", "Production SRT")
    pub name: String,
    /// Destination service kind (e.g. "YouTube", "Twitch", "Kick", "Custom RTMP", "SRT")
    pub service_name: String,
    /// Target ingestion URL (RTMP, RTMPS, or SRT)
    pub url: String,
    /// Optional private stream key or token
    pub stream_key: Option<String>,
    /// Whether this destination is enabled for the next broadcast session
    pub enabled: bool,
    /// Live telemetry stats for this destination
    #[serde(skip)]
    pub stats: Arc<RwLock<LiveOutputStats>>,
}

impl StreamDestination {
    /// Creates a new active stream destination.
    pub fn new(
        name: impl Into<String>,
        service_name: impl Into<String>,
        url: impl Into<String>,
        stream_key: Option<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            service_name: service_name.into(),
            url: url.into(),
            stream_key,
            enabled: true,
            stats: Arc::new(RwLock::new(LiveOutputStats::new())),
        }
    }

    /// Computes the complete target ingestion URL, incorporating the stream key where appropriate.
    pub fn resolved_target_url(&self) -> String {
        let base_url = self.url.trim_end_matches('/');
        if let Some(ref key) = self.stream_key {
            if key.is_empty() {
                base_url.to_string()
            } else if base_url.starts_with("srt://") {
                // For SRT, if key is provided and streamid is not yet in query, format as streamid
                if base_url.contains("streamid=") {
                    base_url.to_string()
                } else {
                    let sep = if base_url.contains('?') { "&" } else { "?" };
                    format!("{}{}{}streamid={}", base_url, sep, "streamid=", key)
                }
            } else {
                // Standard RTMP: rtmp://live.service.com/app/stream_key
                format!("{}/{}", base_url, key)
            }
        } else {
            base_url.to_string()
        }
    }

    /// Returns the FFmpeg muxer format string for this destination (e.g. "flv" for RTMP, "mpegts" for SRT).
    pub fn container_format(&self) -> &'static str {
        if self.url.starts_with("srt://") {
            "mpegts"
        } else {
            "flv"
        }
    }

    /// Formats an entry for the FFmpeg tee muxer string.
    pub fn to_tee_target(&self) -> String {
        let format = self.container_format();
        let target_url = self.resolved_target_url();
        format!("[f={}:onfail=ignore]{}", format, target_url)
    }
}

/// Manager for multi-destination broadcasting.
pub struct MultiStreamManager {
    destinations: Arc<RwLock<Vec<StreamDestination>>>,
}

impl Default for MultiStreamManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MultiStreamManager {
    /// Creates an empty multi-stream manager.
    pub fn new() -> Self {
        Self {
            destinations: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Adds a destination and returns its ID.
    pub fn add_destination(&self, dest: StreamDestination) -> Uuid {
        let id = dest.id;
        self.destinations.write().push(dest);
        id
    }

    /// Removes a destination by ID.
    pub fn remove_destination(&self, id: Uuid) -> bool {
        let mut list = self.destinations.write();
        if let Some(pos) = list.iter().position(|d| d.id == id) {
            list.remove(pos);
            true
        } else {
            false
        }
    }

    /// Toggles the enabled state of a destination.
    pub fn set_enabled(&self, id: Uuid, enabled: bool) -> bool {
        let mut list = self.destinations.write();
        if let Some(dest) = list.iter_mut().find(|d| d.id == id) {
            dest.enabled = enabled;
            true
        } else {
            false
        }
    }

    /// Returns all configured destinations.
    pub fn get_destinations(&self) -> Vec<StreamDestination> {
        self.destinations.read().clone()
    }

    /// Returns all currently enabled destinations.
    pub fn get_enabled_destinations(&self) -> Vec<StreamDestination> {
        self.destinations
            .read()
            .iter()
            .filter(|d| d.enabled)
            .cloned()
            .collect()
    }

    /// Generates FFmpeg CLI arguments for tee multiplexing all enabled destinations.
    ///
    /// If multiple destinations are active, returns `-f tee -map 0:v -map 1:a "[f=flv]...|[f=mpegts]..."`.
    /// If only one destination is active, returns direct standard output arguments.
    pub fn generate_ffmpeg_output_args(&self) -> OutputResult<Vec<String>> {
        let enabled = self.get_enabled_destinations();
        if enabled.is_empty() {
            return Err(OutputError::InvalidParameters(
                "No stream destinations are enabled".into(),
            ));
        }

        if enabled.len() == 1 {
            let dest = &enabled[0];
            return Ok(vec![
                "-f".into(),
                dest.container_format().into(),
                dest.resolved_target_url(),
            ]);
        }

        // Multiple destinations: use FFmpeg tee muxer
        let tee_string = enabled
            .iter()
            .map(|d| d.to_tee_target())
            .collect::<Vec<_>>()
            .join("|");

        Ok(vec![
            "-flags".into(),
            "+global_header".into(),
            "-f".into(),
            "tee".into(),
            "-map".into(),
            "0:v".into(),
            "-map".into(),
            "1:a".into(),
            tee_string,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_multi_destination_tee_arg_generation() {
        let manager = MultiStreamManager::new();

        let id1 = manager.add_destination(StreamDestination::new(
            "Twitch Primary",
            "Twitch",
            "rtmp://live.twitch.tv/app",
            Some("live_twitch_key_123".into()),
        ));

        let id2 = manager.add_destination(StreamDestination::new(
            "YouTube Secondary",
            "YouTube",
            "rtmp://a.rtmp.youtube.com/live2",
            Some("yt_stream_key_456".into()),
        ));

        let _id3 = manager.add_destination(StreamDestination::new(
            "Production SRT",
            "SRT",
            "srt://10.0.0.50:9000?mode=caller&latency=120000",
            None,
        ));

        assert_eq!(manager.get_destinations().len(), 3);
        assert_eq!(manager.get_enabled_destinations().len(), 3);

        let args = manager.generate_ffmpeg_output_args().unwrap();
        assert_eq!(args[0], "-flags");
        assert_eq!(args[1], "+global_header");
        assert_eq!(args[2], "-f");
        assert_eq!(args[3], "tee");
        assert_eq!(args[4], "-map");
        assert_eq!(args[5], "0:v");
        assert_eq!(args[6], "-map");
        assert_eq!(args[7], "1:a");

        let tee_str = &args[8];
        assert!(
            tee_str.contains("[f=flv:onfail=ignore]rtmp://live.twitch.tv/app/live_twitch_key_123")
        );
        assert!(tee_str
            .contains("[f=flv:onfail=ignore]rtmp://a.rtmp.youtube.com/live2/yt_stream_key_456"));
        assert!(tee_str
            .contains("[f=mpegts:onfail=ignore]srt://10.0.0.50:9000?mode=caller&latency=120000"));

        // Disable Twitch and verify args update
        assert!(manager.set_enabled(id1, false));
        assert_eq!(manager.get_enabled_destinations().len(), 2);

        // Remove YouTube
        assert!(manager.remove_destination(id2));
        assert_eq!(manager.get_enabled_destinations().len(), 1);

        // Single destination: returns direct args
        let single_args = manager.generate_ffmpeg_output_args().unwrap();
        assert_eq!(single_args[0], "-f");
        assert_eq!(single_args[1], "mpegts");
        assert_eq!(
            single_args[2],
            "srt://10.0.0.50:9000?mode=caller&latency=120000"
        );
    }
}
