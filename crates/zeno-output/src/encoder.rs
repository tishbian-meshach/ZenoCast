//! Hardware and software encoder abstractions, parameters, and auto-probing.

use serde::{Deserialize, Serialize};
use std::process::Command;
use tracing::{info, warn};

/// Supported hardware and software video encoder implementations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EncoderKind {
    /// NVIDIA NVENC H.264 hardware encoder
    NvencH264,
    /// AMD Advanced Media Framework (AMF) H.264 hardware encoder
    AmfH264,
    /// Intel Quick Sync Video (QSV) H.264 hardware encoder
    QsvH264,
    /// Software CPU x264 encoder fallback
    X264,

    /// NVIDIA NVENC HEVC / H.265 hardware encoder
    NvencHevc,
    /// AMD AMF HEVC / H.265 hardware encoder
    AmfHevc,
    /// Intel QSV HEVC / H.265 hardware encoder
    QsvHevc,
    /// Software CPU x265 encoder
    X265,

    /// NVIDIA NVENC AV1 hardware encoder
    NvencAv1,
    /// AMD AMF AV1 hardware encoder
    AmfAv1,
    /// Intel QSV AV1 hardware encoder
    QsvAv1,
}

impl EncoderKind {
    /// Returns the corresponding FFmpeg video codec name string.
    pub fn ffmpeg_codec_name(&self) -> &'static str {
        match self {
            Self::NvencH264 => "h264_nvenc",
            Self::AmfH264 => "h264_amf",
            Self::QsvH264 => "h264_qsv",
            Self::X264 => "libx264",

            Self::NvencHevc => "hevc_nvenc",
            Self::AmfHevc => "hevc_amf",
            Self::QsvHevc => "hevc_qsv",
            Self::X265 => "libx265",

            Self::NvencAv1 => "av1_nvenc",
            Self::AmfAv1 => "av1_amf",
            Self::QsvAv1 => "av1_qsv",
        }
    }

    /// Whether this encoder utilizes dedicated GPU hardware acceleration.
    pub fn is_hardware(&self) -> bool {
        !matches!(self, Self::X264 | Self::X265)
    }

    /// Returns human-readable label for user interface display.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::NvencH264 => "NVIDIA NVENC H.264",
            Self::AmfH264 => "AMD AMF H.264",
            Self::QsvH264 => "Intel QuickSync H.264",
            Self::X264 => "x264 (Software CPU)",

            Self::NvencHevc => "NVIDIA NVENC HEVC",
            Self::AmfHevc => "AMD AMF HEVC",
            Self::QsvHevc => "Intel QuickSync HEVC",
            Self::X265 => "x265 (Software CPU)",

            Self::NvencAv1 => "NVIDIA NVENC AV1",
            Self::AmfAv1 => "AMD AMF AV1",
            Self::QsvAv1 => "Intel QuickSync AV1",
        }
    }
}

/// Bitrate rate-control mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RateControl {
    /// Constant Bitrate (standard for live streaming)
    Cbr,
    /// Variable Bitrate
    Vbr,
    /// Constant Quantization Parameter (useful for local archive recording)
    Cqp,
}

/// Encoder performance / speed preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncoderPreset {
    UltraFast,
    SuperFast,
    VeryFast,
    Faster,
    Fast,
    Medium,
    Slow,
    Slower,
    VerySlow,
    LowLatency,
    Quality,
}

impl EncoderPreset {
    /// Maps preset to FFmpeg CLI preset string appropriate for codec kind.
    pub fn as_ffmpeg_arg(&self, kind: EncoderKind) -> &'static str {
        match kind {
            EncoderKind::NvencH264 | EncoderKind::NvencHevc | EncoderKind::NvencAv1 => match self {
                Self::LowLatency | Self::UltraFast | Self::SuperFast => "p1",
                Self::VeryFast | Self::Faster => "p2",
                Self::Fast => "p3",
                Self::Medium => "p4",
                Self::Slow => "p5",
                Self::Slower => "p6",
                Self::Quality | Self::VerySlow => "p7",
            },
            EncoderKind::AmfH264 | EncoderKind::AmfHevc | EncoderKind::AmfAv1 => match self {
                Self::LowLatency | Self::UltraFast | Self::SuperFast => "speed",
                Self::VeryFast | Self::Faster | Self::Fast => "speed",
                Self::Medium => "balanced",
                _ => "quality",
            },
            EncoderKind::QsvH264 | EncoderKind::QsvHevc | EncoderKind::QsvAv1 => match self {
                Self::UltraFast | Self::SuperFast => "veryfast",
                Self::VeryFast | Self::Faster => "faster",
                Self::Fast => "fast",
                Self::Medium => "medium",
                _ => "slow",
            },
            EncoderKind::X264 | EncoderKind::X265 => match self {
                Self::UltraFast => "ultrafast",
                Self::SuperFast => "superfast",
                Self::VeryFast => "veryfast",
                Self::Faster => "faster",
                Self::Fast => "fast",
                Self::Medium => "medium",
                Self::Slow => "slow",
                Self::Slower => "slower",
                Self::VerySlow => "veryslow",
                Self::LowLatency => "veryfast",
                Self::Quality => "medium",
            },
        }
    }
}

/// H.264 profile setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncoderProfile {
    Baseline,
    Main,
    High,
    High10,
}

impl EncoderProfile {
    /// Returns FFmpeg profile string.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::Main => "main",
            Self::High => "high",
            Self::High10 => "high10",
        }
    }
}

/// Comprehensive video encoder settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoEncoderSettings {
    /// Encoder implementation kind
    pub encoder_kind: EncoderKind,
    /// Video frame width in pixels (e.g. 1920)
    pub width: u32,
    /// Video frame height in pixels (e.g. 1080)
    pub height: u32,
    /// Frame rate in frames per second (e.g. 60.0)
    pub fps: f32,
    /// Rate control model (CBR, VBR, CQP)
    pub rate_control: RateControl,
    /// Target bitrate in kilobits per second (e.g. 6000)
    pub bitrate_kbps: u32,
    /// Optional maximum bitrate for VBR mode
    pub max_bitrate_kbps: Option<u32>,
    /// Keyframe interval / GOP length in seconds (default 2.0s for broadcast)
    pub keyframe_interval_seconds: f32,
    /// Encoding speed / quality preset
    pub preset: EncoderPreset,
    /// Codec profile
    pub profile: EncoderProfile,
    /// Number of consecutive B-frames (0 for low-latency live streaming)
    pub b_frames: u32,
}

impl Default for VideoEncoderSettings {
    fn default() -> Self {
        Self {
            encoder_kind: EncoderKind::X264,
            width: 1920,
            height: 1080,
            fps: 60.0,
            rate_control: RateControl::Cbr,
            bitrate_kbps: 6000,
            max_bitrate_kbps: Some(6000),
            keyframe_interval_seconds: 2.0,
            preset: EncoderPreset::VeryFast,
            profile: EncoderProfile::High,
            b_frames: 0,
        }
    }
}

impl VideoEncoderSettings {
    /// Total GOP keyframe interval in frame count.
    pub fn gop_size(&self) -> u32 {
        ((self.fps * self.keyframe_interval_seconds).round() as u32).max(1)
    }
}

/// Audio encoder settings for broadcast streaming and recording.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioEncoderSettings {
    /// Audio sample rate in Hz (standard 48000 Hz)
    pub sample_rate: u32,
    /// Channel count (2 for stereo)
    pub channels: u32,
    /// Bitrate in kilobits per second (e.g. 160 kbps AAC)
    pub bitrate_kbps: u32,
}

impl Default for AudioEncoderSettings {
    fn default() -> Self {
        Self {
            sample_rate: 48000,
            channels: 2,
            bitrate_kbps: 160,
        }
    }
}

#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Creates a `std::process::Command` configured with `CREATE_NO_WINDOW` on Windows
/// to prevent console popups during background tool probes or encoding subprocesses.
#[inline]
pub fn create_hidden_command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

static PROBED_ENCODERS: std::sync::OnceLock<Vec<EncoderKind>> = std::sync::OnceLock::new();

/// System hardware encoder prober and auto-selector.
pub struct EncoderProbe;

impl EncoderProbe {
    /// Locates the `ffmpeg` executable path on system PATH or fallback directory.
    pub fn find_ffmpeg() -> Option<String> {
        // First check standard PATH
        if let Ok(output) = create_hidden_command("ffmpeg").arg("-version").output() {
            if output.status.success() {
                return Some("ffmpeg".to_string());
            }
        }

        // Check C:\ffmpeg\bin\ffmpeg.exe
        let standard_win_path = r"C:\ffmpeg\bin\ffmpeg.exe";
        if std::path::Path::new(standard_win_path).exists() {
            return Some(standard_win_path.to_string());
        }

        None
    }

    /// Tests if a specific encoder works on the host machine by running a 1-frame probe.
    pub fn probe_encoder(ffmpeg_path: &str, kind: EncoderKind) -> bool {
        let codec = kind.ffmpeg_codec_name();
        let status = create_hidden_command(ffmpeg_path)
            .args([
                "-f",
                "lavfi",
                "-i",
                "color=c=black:s=64x64:d=0.04",
                "-c:v",
                codec,
                "-f",
                "null",
                "-",
            ])
            .output();

        match status {
            Ok(out) => out.status.success(),
            Err(_) => false,
        }
    }

    /// Probes the system and returns all functional video encoders.
    /// Results are cached for the lifetime of the process so probes run at most once.
    pub fn probe_available_encoders() -> Vec<EncoderKind> {
        PROBED_ENCODERS
            .get_or_init(|| {
                let Some(ffmpeg) = Self::find_ffmpeg() else {
                    return vec![EncoderKind::X264];
                };

                let candidates = [
                    EncoderKind::NvencH264,
                    EncoderKind::AmfH264,
                    EncoderKind::QsvH264,
                    EncoderKind::X264,
                    EncoderKind::NvencHevc,
                    EncoderKind::AmfHevc,
                    EncoderKind::QsvHevc,
                    EncoderKind::X265,
                    EncoderKind::NvencAv1,
                    EncoderKind::AmfAv1,
                    EncoderKind::QsvAv1,
                ];

                let mut available = Vec::new();
                for kind in candidates {
                    if Self::probe_encoder(&ffmpeg, kind) {
                        available.push(kind);
                    }
                }

                if available.is_empty() {
                    available.push(EncoderKind::X264);
                }

                available
            })
            .clone()
    }

    /// Auto-selects the optimal H.264 encoder: NVENC -> AMF -> QSV -> x264 fallback.
    pub fn auto_select_best_h264() -> (EncoderKind, Option<String>) {
        let available = Self::probe_available_encoders();

        if available.contains(&EncoderKind::NvencH264) {
            info!("Auto-selected NVIDIA NVENC H.264 hardware encoder");
            (EncoderKind::NvencH264, None)
        } else if available.contains(&EncoderKind::AmfH264) {
            info!("Auto-selected AMD AMF H.264 hardware encoder");
            (EncoderKind::AmfH264, None)
        } else if available.contains(&EncoderKind::QsvH264) {
            info!("Auto-selected Intel QuickSync H.264 hardware encoder");
            (EncoderKind::QsvH264, None)
        } else {
            let notice =
                "Hardware encoder missing. Falling back to libx264 software encoder.".to_string();
            warn!("{}", notice);
            (EncoderKind::X264, Some(notice))
        }
    }

    /// Auto-selects the optimal HEVC encoder, falling back to H.264 if HEVC is unsupported.
    pub fn auto_select_best_hevc() -> (EncoderKind, Option<String>) {
        let available = Self::probe_available_encoders();

        if available.contains(&EncoderKind::NvencHevc) {
            (EncoderKind::NvencHevc, None)
        } else if available.contains(&EncoderKind::AmfHevc) {
            (EncoderKind::AmfHevc, None)
        } else if available.contains(&EncoderKind::QsvHevc) {
            (EncoderKind::QsvHevc, None)
        } else if available.contains(&EncoderKind::X265) {
            (EncoderKind::X265, None)
        } else {
            let notice = "HEVC encoder unsupported on host GPU. Defaulting to H.264.".to_string();
            warn!("{}", notice);
            let (h264, _) = Self::auto_select_best_h264();
            (h264, Some(notice))
        }
    }
}
