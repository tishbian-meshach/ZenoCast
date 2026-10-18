//! # zeno-output
//!
//! Output engine: hardware encoders (NVENC, AMF, QSV, x264), GPU color conversion (RGBA -> NV12),
//! multi-destination RTMP/SRT streaming, direct YouTube Live integration, and crash-resilient MKV/MP4 recording.

pub mod color_convert;
pub mod encoder;
pub mod error;
pub mod multi_stream;
pub mod ndi;
pub mod pipeline;
pub mod replay;
pub mod service;
pub mod spout;
pub mod srt;
pub mod stats;
pub mod youtube;

pub use color_convert::{ColorConverter, Nv12Buffer};
pub use encoder::{
    create_hidden_command, AudioEncoderSettings, EncoderKind, EncoderPreset, EncoderProbe,
    EncoderProfile, RateControl, VideoEncoderSettings,
};
pub use error::{OutputError, OutputResult};
pub use multi_stream::{MultiStreamManager, StreamDestination};
pub use ndi::NdiOutput;
pub use pipeline::{FfmpegPipeline, OutputDestination};
pub use replay::{
    BufferedAudioChunk, BufferedVideoFrame, ReplayBuffer, ReplayBufferConfig, ReplayBufferStatus,
    ReplaySaveReport,
};
pub use service::OutputManager;
pub use spout::SpoutOutput;
pub use srt::{SrtConfig, SrtMode};
pub use stats::LiveOutputStats;
pub use zeno_platform::PlatformCredentialManager;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encoder_kind_properties() {
        assert_eq!(EncoderKind::NvencH264.ffmpeg_codec_name(), "h264_nvenc");
        assert_eq!(EncoderKind::AmfH264.ffmpeg_codec_name(), "h264_amf");
        assert_eq!(EncoderKind::QsvH264.ffmpeg_codec_name(), "h264_qsv");
        assert_eq!(EncoderKind::X264.ffmpeg_codec_name(), "libx264");

        assert!(EncoderKind::NvencH264.is_hardware());
        assert!(EncoderKind::AmfH264.is_hardware());
        assert!(!EncoderKind::X264.is_hardware());
    }

    #[test]
    fn test_rgba_to_nv12_color_conversion() {
        let width = 4;
        let height = 4;
        let mut rgba = vec![0u8; (width * height * 4) as usize];

        // Fill with pure white (R=255, G=255, B=255, A=255)
        for chunk in rgba.chunks_exact_mut(4) {
            chunk[0] = 255;
            chunk[1] = 255;
            chunk[2] = 255;
            chunk[3] = 255;
        }

        let mut out = Nv12Buffer::new(width, height);
        ColorConverter::rgba_to_nv12(&rgba, width, height, &mut out);

        // White in BT.709 studio range has Y ~ 235, U ~ 128, V ~ 128
        assert_eq!(out.y_plane().len(), 16);
        assert_eq!(out.uv_plane().len(), 8);

        for &y in out.y_plane() {
            assert!(
                (230..=235).contains(&y),
                "Expected white Y ~ 235, got {}",
                y
            );
        }

        for chunk in out.uv_plane().chunks_exact(2) {
            let u = chunk[0];
            let v = chunk[1];
            assert!(
                (u as i32 - 128).abs() <= 2,
                "Expected white U ~ 128, got {}",
                u
            );
            assert!(
                (v as i32 - 128).abs() <= 2,
                "Expected white V ~ 128, got {}",
                v
            );
        }
    }

    #[test]
    fn test_live_stats_dropped_frame_tracking() {
        let mut stats = LiveOutputStats::new();
        assert_eq!(stats.dropped_frames, 0);
        assert_eq!(stats.dropped_percent, 0.0);

        // Record 1000 normal frames
        for _ in 0..999 {
            stats.record_frame(false);
        }
        // Record 1 dropped frame (1 in 1000 = 0.1%)
        stats.record_frame(true);

        assert_eq!(stats.total_frames, 1000);
        assert_eq!(stats.dropped_frames, 1);
        assert!((stats.dropped_percent - 0.1).abs() < 0.001);
    }

    #[test]
    fn test_encoder_auto_probing_finds_available_encoder() {
        let (best_h264, notice) = EncoderProbe::auto_select_best_h264();
        // Host has AMF or x264 fallback
        assert!(
            best_h264 == EncoderKind::AmfH264
                || best_h264 == EncoderKind::NvencH264
                || best_h264 == EncoderKind::QsvH264
                || best_h264 == EncoderKind::X264
        );
        if best_h264 == EncoderKind::X264 {
            assert!(notice.is_some());
        }
    }
}
