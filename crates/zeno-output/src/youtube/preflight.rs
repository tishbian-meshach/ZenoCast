//! YouTube Live preflight checks and recommended encoder parameters.

use crate::encoder::{AudioEncoderSettings, EncoderProbe, VideoEncoderSettings};
use serde::{Deserialize, Serialize};

/// Recommended bitrate parameters from YouTube Live encoder documentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecommendedBitrate {
    /// Minimum recommended bitrate in kbps
    pub min_kbps: u32,
    /// Maximum recommended bitrate in kbps
    pub max_kbps: u32,
    /// Default optimal bitrate in kbps
    pub default_kbps: u32,
}

/// Official YouTube Live recommended bitrate reference lookup.
pub struct YouTubeBitrateTable;

impl YouTubeBitrateTable {
    /// Returns YouTube's official recommended bitrate range for given resolution and frame rate.
    pub fn lookup(width: u32, height: u32, fps: f32) -> RecommendedBitrate {
        let is_high_fps = fps >= 45.0;

        if width >= 3840 || height >= 2160 {
            // 4K (2160p)
            if is_high_fps {
                RecommendedBitrate {
                    min_kbps: 20000,
                    max_kbps: 51000,
                    default_kbps: 30000,
                }
            } else {
                RecommendedBitrate {
                    min_kbps: 13000,
                    max_kbps: 34000,
                    default_kbps: 20000,
                }
            }
        } else if width >= 2560 || height >= 1440 {
            // 1440p (2K)
            if is_high_fps {
                RecommendedBitrate {
                    min_kbps: 9000,
                    max_kbps: 18000,
                    default_kbps: 12000,
                }
            } else {
                RecommendedBitrate {
                    min_kbps: 6000,
                    max_kbps: 13000,
                    default_kbps: 9000,
                }
            }
        } else if width >= 1920 || height >= 1080 {
            // 1080p (Full HD)
            if is_high_fps {
                RecommendedBitrate {
                    min_kbps: 4500,
                    max_kbps: 9000,
                    default_kbps: 6000,
                }
            } else {
                RecommendedBitrate {
                    min_kbps: 3000,
                    max_kbps: 6000,
                    default_kbps: 4500,
                }
            }
        } else {
            // 720p (HD)
            if is_high_fps {
                RecommendedBitrate {
                    min_kbps: 2250,
                    max_kbps: 6000,
                    default_kbps: 3500,
                }
            } else {
                RecommendedBitrate {
                    min_kbps: 1500,
                    max_kbps: 4000,
                    default_kbps: 2500,
                }
            }
        }
    }
}

/// Preflight check report for in-app "Go Live" readiness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreflightReport {
    /// Overall readiness to transition to live on air
    pub can_go_live: bool,
    /// Whether user is successfully authenticated with a valid token
    pub is_authenticated: bool,
    /// Whether channel is enabled for live streaming (not pending 24h review)
    pub is_channel_live_enabled: bool,
    /// Whether hardware or software video encoder is functional
    pub is_encoder_healthy: bool,
    /// Whether audio is properly configured (48 kHz stereo)
    pub is_audio_ready: bool,
    /// Whether configured bitrate falls within YouTube recommendations
    pub is_bitrate_optimal: bool,
    /// User-facing advisory warnings (e.g. bitrate slightly high)
    pub warnings: Vec<String>,
    /// Blocking errors preventing broadcast start with actionable fix hints
    pub errors: Vec<String>,
}

/// Preflight validator for YouTube Live broadcasting.
pub struct PreflightValidator;

impl PreflightValidator {
    /// Validates production state against YouTube requirements prior to initiating broadcast.
    pub fn validate(
        is_authenticated: bool,
        is_channel_live_enabled: bool,
        video_settings: &VideoEncoderSettings,
        audio_settings: &AudioEncoderSettings,
    ) -> PreflightReport {
        let mut warnings = Vec::new();
        let mut errors = Vec::new();

        if !is_authenticated {
            errors.push(
                "Not signed in to YouTube. Please sign in via Settings > YouTube.".to_string(),
            );
        }

        if !is_channel_live_enabled {
            errors.push(
                "YouTube channel is not enabled for live streaming. Initial verification takes up to 24 hours. Visit https://www.youtube.com/live_dashboard to check status."
                    .to_string(),
            );
        }

        let is_encoder_healthy = EncoderProbe::find_ffmpeg().is_some();
        if !is_encoder_healthy {
            errors.push(
                "FFmpeg video encoder was not found. Please verify FFmpeg installation."
                    .to_string(),
            );
        }

        let is_audio_ready = audio_settings.sample_rate == 48000 && audio_settings.channels == 2;
        if !is_audio_ready {
            warnings.push(format!(
                "YouTube requires 48 kHz stereo audio. Current configuration: {} Hz, {} channels.",
                audio_settings.sample_rate, audio_settings.channels
            ));
        }

        // Verify GOP / Keyframe interval (YouTube requires between 1.0 and 4.0 seconds, recommended 2.0s)
        if video_settings.keyframe_interval_seconds < 1.0
            || video_settings.keyframe_interval_seconds > 4.0
        {
            warnings.push(format!(
                "YouTube requires keyframe interval between 1.0s and 4.0s (currently {:.1}s). 2.0s is recommended.",
                video_settings.keyframe_interval_seconds
            ));
        }

        // Verify Bitrate against official table
        let rec = YouTubeBitrateTable::lookup(
            video_settings.width,
            video_settings.height,
            video_settings.fps,
        );
        let is_bitrate_optimal = video_settings.bitrate_kbps >= rec.min_kbps
            && video_settings.bitrate_kbps <= rec.max_kbps;
        if !is_bitrate_optimal {
            warnings.push(format!(
                "Configured bitrate ({} kbps) is outside YouTube's recommended range for {}x{} at {:.0} fps ({} - {} kbps).",
                video_settings.bitrate_kbps, video_settings.width, video_settings.height, video_settings.fps, rec.min_kbps, rec.max_kbps
            ));
        }

        let can_go_live = errors.is_empty();

        PreflightReport {
            can_go_live,
            is_authenticated,
            is_channel_live_enabled,
            is_encoder_healthy,
            is_audio_ready,
            is_bitrate_optimal,
            warnings,
            errors,
        }
    }
}
