//! Studio settings configuration: Video, Audio, Output, and Hotkeys.

use serde::{Deserialize, Serialize};

/// Master video output configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoSettings {
    pub base_width: u32,
    pub base_height: u32,
    pub output_width: u32,
    pub output_height: u32,
    pub fps: f32,
}

impl Default for VideoSettings {
    fn default() -> Self {
        Self {
            base_width: 1920,
            base_height: 1080,
            output_width: 1920,
            output_height: 1080,
            fps: 60.0,
        }
    }
}

/// Audio ingest and master mixing configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    pub sample_rate: u32,
    pub channels: u32,
    pub bitrate_kbps: u32,
    pub mic_device: Option<String>,
    pub desktop_device: Option<String>,
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            bitrate_kbps: 160,
            mic_device: None,
            desktop_device: None,
        }
    }
}

/// Encoder and streaming destination configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputSettings {
    pub encoder_kind: String,
    pub video_bitrate_kbps: u32,
    pub keyframe_interval_seconds: f32,
    pub rate_control: String,
    pub stream_service: String,
    pub stream_url: String,
    pub stream_key: String,
    pub record_directory: String,
    pub record_format: String,
    pub remux_to_mp4: bool,
}

impl Default for OutputSettings {
    fn default() -> Self {
        Self {
            encoder_kind: "Auto Hardware (NVENC / AMF / QSV)".into(),
            video_bitrate_kbps: 6000,
            keyframe_interval_seconds: 2.0,
            rate_control: "CBR".into(),
            stream_service: "YouTube Live".into(),
            stream_url: "rtmp://a.rtmp.youtube.com/live2".into(),
            stream_key: "".into(),
            record_directory: "recordings".into(),
            record_format: "MKV".into(),
            remux_to_mp4: true,
        }
    }
}

/// Operator keybindings for live studio actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotkeySettings {
    pub cut_transition: String,
    pub auto_transition: String,
    pub start_streaming: String,
    pub stop_streaming: String,
    pub start_recording: String,
    pub stop_recording: String,
    pub mute_master: String,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self {
            cut_transition: "Space".into(),
            auto_transition: "Enter".into(),
            start_streaming: "F9".into(),
            stop_streaming: "Shift+F9".into(),
            start_recording: "F10".into(),
            stop_recording: "Shift+F10".into(),
            mute_master: "M".into(),
        }
    }
}

/// Root studio settings collection.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StudioSettings {
    pub video: VideoSettings,
    pub audio: AudioSettings,
    pub output: OutputSettings,
    pub hotkeys: HotkeySettings,
}
