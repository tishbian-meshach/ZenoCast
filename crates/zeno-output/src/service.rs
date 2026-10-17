//! High-level Output Manager coordinating real-time encoding, streaming, and recording.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{bounded, Receiver, Sender};
use parking_lot::Mutex;
use tracing::{info, warn};

use crate::color_convert::{ColorConverter, Nv12Buffer};
use crate::encoder::{AudioEncoderSettings, VideoEncoderSettings};
use crate::error::OutputResult;
use crate::pipeline::{FfmpegPipeline, OutputDestination};
use crate::stats::LiveOutputStats;

/// Maximum number of uncompressed video frames buffered before shedding frames to prevent memory bloat.
const VIDEO_QUEUE_CAPACITY: usize = 32;
/// Maximum number of audio chunks buffered.
const AUDIO_QUEUE_CAPACITY: usize = 128;

/// A video frame submitted from the compositor render path.
pub struct QueuedVideoFrame {
    /// Contiguous RGBA8 pixel data.
    pub rgba_data: Vec<u8>,
    /// Frame width in pixels.
    pub width: u32,
    /// Frame height in pixels.
    pub height: u32,
    /// Master presentation timestamp in seconds.
    pub timestamp_seconds: f64,
}

/// An audio chunk submitted from the audio mixer.
pub struct QueuedAudioChunk {
    /// 32-bit float interleaved stereo samples.
    pub samples: Vec<f32>,
    /// Master presentation timestamp in seconds.
    pub timestamp_seconds: f64,
}

/// Central production output manager with dedicated concurrent video and audio dispatch threads.
pub struct OutputManager {
    video_sender: Sender<QueuedVideoFrame>,
    audio_sender: Sender<QueuedAudioChunk>,
    stats: Arc<Mutex<LiveOutputStats>>,
    pipeline: Arc<Mutex<Option<FfmpegPipeline>>>,
    is_streaming: Arc<AtomicBool>,
    is_recording: Arc<AtomicBool>,
    workers_running: Arc<AtomicBool>,
    video_worker_handle: Option<JoinHandle<()>>,
    audio_worker_handle: Option<JoinHandle<()>>,
    stream_url: Arc<Mutex<Option<String>>>,
    record_path: Arc<Mutex<Option<PathBuf>>>,
    video_settings: Arc<Mutex<VideoEncoderSettings>>,
    audio_settings: Arc<Mutex<AudioEncoderSettings>>,
    reconnect_attempts: Arc<AtomicU64>,
}

impl OutputManager {
    /// Creates and initializes the background output manager.
    pub fn new(video_settings: VideoEncoderSettings, audio_settings: AudioEncoderSettings) -> Self {
        let (video_tx, video_rx) = bounded::<QueuedVideoFrame>(VIDEO_QUEUE_CAPACITY);
        let (audio_tx, audio_rx) = bounded::<QueuedAudioChunk>(AUDIO_QUEUE_CAPACITY);

        let stats = Arc::new(Mutex::new(LiveOutputStats::new()));
        let pipeline = Arc::new(Mutex::new(None));
        let is_streaming = Arc::new(AtomicBool::new(false));
        let is_recording = Arc::new(AtomicBool::new(false));
        let workers_running = Arc::new(AtomicBool::new(true));
        let stream_url = Arc::new(Mutex::new(None));
        let record_path = Arc::new(Mutex::new(None));
        let v_settings = Arc::new(Mutex::new(video_settings));
        let a_settings = Arc::new(Mutex::new(audio_settings));
        let reconnect_attempts = Arc::new(AtomicU64::new(0));

        // Video Worker Thread
        let v_stats = Arc::clone(&stats);
        let v_pipeline = Arc::clone(&pipeline);
        let v_flag = Arc::clone(&workers_running);
        let v_reconnects = Arc::clone(&reconnect_attempts);
        let v_streaming = Arc::clone(&is_streaming);
        let v_recording = Arc::clone(&is_recording);

        let video_worker_handle = thread::spawn(move || {
            Self::video_worker_loop(
                video_rx,
                v_stats,
                v_pipeline,
                v_flag,
                v_streaming,
                v_recording,
                v_reconnects,
            );
        });

        // Audio Worker Thread
        let a_pipeline = Arc::clone(&pipeline);
        let a_flag = Arc::clone(&workers_running);

        let audio_worker_handle = thread::spawn(move || {
            Self::audio_worker_loop(audio_rx, a_pipeline, a_flag);
        });

        Self {
            video_sender: video_tx,
            audio_sender: audio_tx,
            stats,
            pipeline,
            is_streaming,
            is_recording,
            workers_running,
            video_worker_handle: Some(video_worker_handle),
            audio_worker_handle: Some(audio_worker_handle),
            stream_url,
            record_path,
            video_settings: v_settings,
            audio_settings: a_settings,
            reconnect_attempts,
        }
    }

    /// Dedicated concurrent worker for audio chunk consumption.
    fn audio_worker_loop(
        audio_rx: Receiver<QueuedAudioChunk>,
        pipeline: Arc<Mutex<Option<FfmpegPipeline>>>,
        workers_running: Arc<AtomicBool>,
    ) {
        while workers_running.load(Ordering::Relaxed) {
            if let Ok(chunk) = audio_rx.recv_timeout(Duration::from_millis(5)) {
                let p_guard = pipeline.lock();
                if let Some(ref p) = *p_guard {
                    let _ = p.write_audio_samples(&chunk.samples);
                }
            }
        }
    }

    /// Dedicated concurrent worker for video frame consumption and stats tracking.
    fn video_worker_loop(
        video_rx: Receiver<QueuedVideoFrame>,
        stats: Arc<Mutex<LiveOutputStats>>,
        pipeline: Arc<Mutex<Option<FfmpegPipeline>>>,
        workers_running: Arc<AtomicBool>,
        is_streaming: Arc<AtomicBool>,
        is_recording: Arc<AtomicBool>,
        reconnects: Arc<AtomicU64>,
    ) {
        let mut nv12_buf = Nv12Buffer::new(1920, 1080);
        let mut last_stats_update = Instant::now();
        let mut bytes_since_last_stats = 0u64;

        while workers_running.load(Ordering::Relaxed) {
            if let Ok(frame) = video_rx.recv_timeout(Duration::from_millis(5)) {
                let encode_start = Instant::now();

                // Convert RGBA to NV12 without per-frame allocation
                ColorConverter::rgba_to_nv12(
                    &frame.rgba_data,
                    frame.width,
                    frame.height,
                    &mut nv12_buf,
                );

                let mut write_err = false;
                {
                    let mut p_guard = pipeline.lock();
                    if let Some(ref mut p) = *p_guard {
                        if !p.is_alive() {
                            write_err = true;
                        } else if let Err(e) = p.write_video_frame(nv12_buf.as_bytes()) {
                            warn!("Video write failed: {}", e);
                            write_err = true;
                        } else {
                            bytes_since_last_stats += nv12_buf.as_bytes().len() as u64;
                        }
                    }
                }

                if write_err {
                    warn!("Pipeline disconnected.");
                    if let Some(mut p) = pipeline.lock().take() {
                        let _ = p.stop();
                    }
                }

                let encode_time = encode_start.elapsed().as_secs_f32() * 1000.0;

                // Update live statistics
                let mut s = stats.lock();
                s.record_frame(false);
                s.encode_time_ms = encode_time;
            }

            // Periodic stats update
            if last_stats_update.elapsed().as_secs_f32() >= 0.25 {
                let dt = last_stats_update.elapsed().as_secs_f32();
                let mut s = stats.lock();
                s.update_bitrate(bytes_since_last_stats, dt);
                bytes_since_last_stats = 0;
                last_stats_update = Instant::now();
                s.is_streaming = is_streaming.load(Ordering::Relaxed);
                s.is_recording = is_recording.load(Ordering::Relaxed);
                s.reconnect_attempts = reconnects.load(Ordering::Relaxed) as u32;
            }
        }
    }

    /// Submits an RGBA video frame from the compositor.
    ///
    /// Real-time guarantee: If the queue is congested, frames are shed instantly without blocking.
    pub fn send_video_frame(
        &self,
        rgba_data: &[u8],
        width: u32,
        height: u32,
        timestamp_seconds: f64,
    ) {
        let frame = QueuedVideoFrame {
            rgba_data: rgba_data.to_vec(),
            width,
            height,
            timestamp_seconds,
        };

        if self.video_sender.try_send(frame).is_err() {
            // Buffer full: drop frame to maintain real-time constraints
            let mut s = self.stats.lock();
            s.record_frame(true);
        }
    }

    /// Submits interleaved 32-bit float stereo audio samples from the mixer.
    pub fn send_audio_samples(&self, samples: &[f32], timestamp_seconds: f64) {
        let chunk = QueuedAudioChunk {
            samples: samples.to_vec(),
            timestamp_seconds,
        };
        let _ = self.audio_sender.try_send(chunk);
    }

    /// Starts streaming to an RTMP or SRT destination.
    pub fn start_streaming(&self, url: impl Into<String>) -> OutputResult<()> {
        let url_str = url.into();
        *self.stream_url.lock() = Some(url_str.clone());
        self.is_streaming.store(true, Ordering::SeqCst);

        // Ensure active pipeline
        self.ensure_pipeline()?;
        Ok(())
    }

    /// Stops the active live stream.
    pub fn stop_streaming(&self) {
        self.is_streaming.store(false, Ordering::SeqCst);
        *self.stream_url.lock() = None;

        if !self.is_recording.load(Ordering::SeqCst) {
            if let Some(mut p) = self.pipeline.lock().take() {
                let _ = p.stop();
            }
        }
    }

    /// Starts recording to an MKV file destination.
    pub fn start_recording(&self, path: impl Into<PathBuf>) -> OutputResult<()> {
        let path_buf = path.into();
        *self.record_path.lock() = Some(path_buf);
        self.is_recording.store(true, Ordering::SeqCst);

        // Ensure active pipeline
        self.ensure_pipeline()?;
        Ok(())
    }

    /// Ensures that the underlying FFmpeg pipeline is launched and connected.
    fn ensure_pipeline(&self) -> OutputResult<()> {
        let mut p_guard = self.pipeline.lock();
        if p_guard.is_none() {
            let destination = OutputDestination {
                stream_url: self.stream_url.lock().clone(),
                record_path: self.record_path.lock().clone(),
            };
            let v = self.video_settings.lock().clone();
            let a = self.audio_settings.lock().clone();

            let pipeline = FfmpegPipeline::start(destination, v, a)?;
            info!("FFmpeg output pipeline successfully initialized and connected");
            *p_guard = Some(pipeline);
        }
        Ok(())
    }

    /// Stops recording and optionally remuxes the MKV archive to MP4.
    pub fn stop_recording(&self, remux_to_mp4: bool) -> OutputResult<Option<PathBuf>> {
        self.is_recording.store(false, Ordering::SeqCst);
        let path = self.record_path.lock().take();

        if let Some(ref mkv_path) = path {
            // Stop the pipeline now so container headers are finalized on disk
            if !self.is_streaming.load(Ordering::SeqCst) {
                if let Some(mut p) = self.pipeline.lock().take() {
                    let _ = p.stop();
                }
            }

            if remux_to_mp4 {
                let mp4_path = mkv_path.with_extension("mp4");
                FfmpegPipeline::remux_mkv_to_mp4(mkv_path, &mp4_path)?;
                return Ok(Some(mp4_path));
            }
        }

        Ok(path)
    }

    /// Returns a snapshot of live output statistics.
    pub fn stats(&self) -> LiveOutputStats {
        self.stats.lock().clone()
    }

    /// Updates video encoder settings.
    pub fn set_video_settings(&self, settings: VideoEncoderSettings) {
        *self.video_settings.lock() = settings;
    }

    /// Updates audio encoder settings.
    pub fn set_audio_settings(&self, settings: AudioEncoderSettings) {
        *self.audio_settings.lock() = settings;
    }

    /// Returns the total reconnection attempts logged by the manager.
    pub fn reconnect_attempts(&self) -> u64 {
        self.reconnect_attempts.load(Ordering::Relaxed)
    }
}

impl Drop for OutputManager {
    fn drop(&mut self) {
        self.workers_running.store(false, Ordering::SeqCst);
        if let Some(mut p) = self.pipeline.lock().take() {
            let _ = p.stop();
        }
        if let Some(handle) = self.video_worker_handle.take() {
            let _ = handle.join();
        }
        if let Some(handle) = self.audio_worker_handle.take() {
            let _ = handle.join();
        }
    }
}
