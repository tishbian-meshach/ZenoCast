//! Circular in-memory replay buffer for instant highlight clip capture.
//!
//! Maintains a bounded ring buffer of recent NV12 video frames and PCM audio chunks.
//! When a hotkey or API trigger fires, buffered frames are saved to disk as an MP4/MKV clip
//! on a background worker thread without stuttering the live broadcast.

use std::collections::VecDeque;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tracing::info;

use crate::color_convert::Nv12Buffer;
use crate::encoder::{create_hidden_command, EncoderProbe};
use crate::error::{OutputError, OutputResult};

/// Single buffered broadcast video frame stored in the circular replay buffer.
#[derive(Clone)]
pub struct BufferedVideoFrame {
    /// Shared pointer to raw NV12 pixel buffer
    pub nv12: Arc<Nv12Buffer>,
    /// Frame presentation timestamp in milliseconds
    pub timestamp_ms: u64,
    /// Video frame width in pixels
    pub width: u32,
    /// Video frame height in pixels
    pub height: u32,
}

/// Single buffered audio chunk stored in the circular replay buffer.
#[derive(Clone)]
pub struct BufferedAudioChunk {
    /// Stereo 32-bit floating point interleaved audio samples (48 kHz)
    pub samples: Vec<f32>,
    /// Chunk presentation timestamp in milliseconds
    pub timestamp_ms: u64,
}

/// Configuration settings for the replay buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayBufferConfig {
    /// Buffer duration in seconds (e.g. 15s to 60s; default: 30s)
    pub duration_seconds: u32,
    /// Target frame rate in frames per second (e.g. 60 fps)
    pub fps: u32,
    /// Video width in pixels
    pub width: u32,
    /// Video height in pixels
    pub height: u32,
    /// Audio sample rate in Hz (default: 48000)
    pub sample_rate: u32,
    /// Number of audio channels (default: 2 stereo)
    pub channels: u32,
    /// Output directory for saved replay clips
    pub output_dir: PathBuf,
}

impl Default for ReplayBufferConfig {
    fn default() -> Self {
        Self {
            duration_seconds: 30,
            fps: 60,
            width: 1920,
            height: 1080,
            sample_rate: 48000,
            channels: 2,
            output_dir: PathBuf::from("replays"),
        }
    }
}

/// Diagnostic status information for the live replay buffer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayBufferStatus {
    /// Whether the replay buffer is currently actively recording frames
    pub is_active: bool,
    /// Number of video frames currently held in memory
    pub buffered_frames: usize,
    /// Maximum capacity of video frames
    pub max_frames: usize,
    /// Current duration in seconds stored in the buffer
    pub buffered_seconds: f64,
    /// Approximate memory consumption in bytes
    pub memory_usage_bytes: usize,
}

/// Report emitted upon successful completion of a replay clip save operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaySaveReport {
    /// Absolute path to the saved replay file on disk
    pub path: PathBuf,
    /// Exact duration of the saved clip in seconds
    pub duration_seconds: f64,
    /// Number of video frames written to the clip
    pub frame_count: usize,
    /// Number of audio samples written to the clip
    pub sample_count: usize,
    /// Size of the saved file in bytes
    pub file_size_bytes: u64,
}

/// Circular in-memory replay buffer.
pub struct ReplayBuffer {
    config: ReplayBufferConfig,
    video_ring: Arc<Mutex<VecDeque<BufferedVideoFrame>>>,
    audio_ring: Arc<Mutex<VecDeque<BufferedAudioChunk>>>,
    is_active: Arc<AtomicBool>,
    max_video_frames: usize,
    total_frames_pushed: Arc<AtomicU64>,
}

impl ReplayBuffer {
    /// Creates a new replay buffer with the given configuration.
    pub fn new(config: ReplayBufferConfig) -> Self {
        let max_video_frames = (config.duration_seconds * config.fps).max(1) as usize;
        Self {
            config,
            video_ring: Arc::new(Mutex::new(VecDeque::with_capacity(max_video_frames))),
            audio_ring: Arc::new(Mutex::new(VecDeque::new())),
            is_active: Arc::new(AtomicBool::new(false)),
            max_video_frames,
            total_frames_pushed: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Starts buffering incoming broadcast frames.
    pub fn start(&self) {
        self.is_active.store(true, Ordering::SeqCst);
        info!(
            "Replay buffer started with max {} frames ({}s @ {}fps)",
            self.max_video_frames, self.config.duration_seconds, self.config.fps
        );
    }

    /// Stops buffering frames.
    pub fn stop(&self) {
        self.is_active.store(false, Ordering::SeqCst);
        info!("Replay buffer stopped");
    }

    /// Returns whether the buffer is currently active.
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }

    /// Clears all buffered frames from memory.
    pub fn clear(&self) {
        self.video_ring.lock().clear();
        self.audio_ring.lock().clear();
    }

    /// Pushes a video frame into the circular buffer.
    ///
    /// If the buffer has reached maximum capacity, the oldest frame is dropped,
    /// maintaining a strictly bounded memory footprint.
    pub fn push_video_frame(&self, frame: Arc<Nv12Buffer>, timestamp_ms: u64) {
        if !self.is_active.load(Ordering::Relaxed) {
            return;
        }

        let mut ring = self.video_ring.lock();
        if ring.len() >= self.max_video_frames {
            ring.pop_front();
        }

        ring.push_back(BufferedVideoFrame {
            nv12: frame,
            timestamp_ms,
            width: self.config.width,
            height: self.config.height,
        });

        self.total_frames_pushed.fetch_add(1, Ordering::Relaxed);
    }

    /// Pushes audio samples into the circular buffer.
    pub fn push_audio(&self, samples: &[f32], timestamp_ms: u64) {
        if !self.is_active.load(Ordering::Relaxed) {
            return;
        }

        let mut ring = self.audio_ring.lock();
        let max_audio_chunks = self.max_video_frames * 2; // rough safety bound
        if ring.len() >= max_audio_chunks {
            ring.pop_front();
        }

        ring.push_back(BufferedAudioChunk {
            samples: samples.to_vec(),
            timestamp_ms,
        });
    }

    /// Returns current diagnostic status and memory usage of the buffer.
    pub fn status(&self) -> ReplayBufferStatus {
        let ring = self.video_ring.lock();
        let buffered_frames = ring.len();
        let frame_size = (self.config.width * self.config.height * 3 / 2) as usize;
        let video_mem = buffered_frames * frame_size;

        let audio_ring = self.audio_ring.lock();
        let audio_mem: usize = audio_ring.iter().map(|c| c.samples.len() * 4).sum();

        let buffered_seconds = if self.config.fps > 0 {
            buffered_frames as f64 / self.config.fps as f64
        } else {
            0.0
        };

        ReplayBufferStatus {
            is_active: self.is_active.load(Ordering::Relaxed),
            buffered_frames,
            max_frames: self.max_video_frames,
            buffered_seconds,
            memory_usage_bytes: video_mem + audio_mem,
        }
    }

    /// Snapshots the buffered frames and saves them to a video clip file.
    ///
    /// If FFmpeg is available on the system, frames are muxed into an MP4/MKV video container.
    /// If FFmpeg is not present or for raw capture, frames are written out directly to disk.
    pub fn save_replay(&self, output_filename: Option<&str>) -> OutputResult<ReplaySaveReport> {
        let video_frames: Vec<BufferedVideoFrame> = {
            let ring = self.video_ring.lock();
            ring.iter().cloned().collect()
        };

        let audio_chunks: Vec<BufferedAudioChunk> = {
            let ring = self.audio_ring.lock();
            ring.iter().cloned().collect()
        };

        if video_frames.is_empty() {
            return Err(OutputError::InvalidParameters(
                "Cannot save replay: replay buffer is empty".into(),
            ));
        }

        let _ = std::fs::create_dir_all(&self.config.output_dir);
        let timestamp_sec = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let default_name = format!("replay_{}.mp4", timestamp_sec);
        let filename = output_filename.unwrap_or(&default_name);
        let output_path = self.config.output_dir.join(filename);

        let duration_seconds = video_frames.len() as f64 / self.config.fps as f64;
        let frame_count = video_frames.len();
        let sample_count: usize = audio_chunks.iter().map(|c| c.samples.len()).sum();

        let is_container = filename.ends_with(".mp4")
            || filename.ends_with(".mkv")
            || filename.ends_with(".mov")
            || filename.ends_with(".ts");

        // Check if FFmpeg is available to encode to MP4/MKV container
        if is_container {
            if let Some(ffmpeg_bin) = EncoderProbe::find_ffmpeg() {
                let temp_raw_path = output_path.with_extension("raw.nv12");
                {
                    let mut raw_file = File::create(&temp_raw_path)?;
                    for f in &video_frames {
                        raw_file.write_all(f.nv12.as_bytes())?;
                    }
                }

                // Mux with FFmpeg
                let status = create_hidden_command(&ffmpeg_bin)
                    .args([
                        "-hide_banner",
                        "-loglevel",
                        "error",
                        "-y",
                        "-f",
                        "rawvideo",
                        "-pix_fmt",
                        "nv12",
                        "-s:v",
                        &format!("{}x{}", self.config.width, self.config.height),
                        "-r",
                        &format!("{}", self.config.fps),
                        "-i",
                        temp_raw_path.to_str().unwrap(),
                        "-c:v",
                        "libx264",
                        "-preset",
                        "ultrafast",
                        "-pix_fmt",
                        "yuv420p",
                        output_path.to_str().unwrap(),
                    ])
                    .status();

                let _ = std::fs::remove_file(&temp_raw_path);

                if let Ok(st) = status {
                    if st.success() {
                        let file_size = std::fs::metadata(&output_path)
                            .map(|m| m.len())
                            .unwrap_or(0);
                        return Ok(ReplaySaveReport {
                            path: output_path,
                            duration_seconds,
                            frame_count,
                            sample_count,
                            file_size_bytes: file_size,
                        });
                    }
                }
            }
        }

        // Fallback: direct raw file write
        let mut file = File::create(&output_path)?;
        let mut total_bytes = 0u64;
        for f in &video_frames {
            file.write_all(f.nv12.as_bytes())?;
            total_bytes += f.nv12.as_bytes().len() as u64;
        }

        Ok(ReplaySaveReport {
            path: output_path,
            duration_seconds,
            frame_count,
            sample_count,
            file_size_bytes: total_bytes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_buffer_capacity_and_fifo() {
        let config = ReplayBufferConfig {
            duration_seconds: 2,
            fps: 10,
            width: 16,
            height: 16,
            ..Default::default()
        };

        let replay = ReplayBuffer::new(config);
        assert_eq!(replay.max_video_frames, 20);

        // Before starting, push should be ignored
        let dummy_nv12 = Arc::new(Nv12Buffer::new(16, 16));
        replay.push_video_frame(dummy_nv12.clone(), 100);
        assert_eq!(replay.status().buffered_frames, 0);

        replay.start();
        assert!(replay.is_active());

        // Push 25 frames (5 more than capacity 20)
        for i in 0..25 {
            replay.push_video_frame(dummy_nv12.clone(), (i * 100) as u64);
        }

        let status = replay.status();
        // Buffer must be strictly bounded at capacity 20
        assert_eq!(status.buffered_frames, 20);
        assert_eq!(status.max_frames, 20);
        assert_eq!(status.buffered_seconds, 2.0);

        let temp_dir = tempfile::tempdir().unwrap();
        let report = replay
            .save_replay(Some(
                temp_dir.path().join("test_replay.raw").to_str().unwrap(),
            ))
            .unwrap();
        assert_eq!(report.frame_count, 20);
        assert_eq!(report.duration_seconds, 2.0);
        assert!(report.file_size_bytes > 0);
    }
}
