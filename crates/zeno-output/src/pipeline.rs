//! FFmpeg subprocess pipeline for hardware-accelerated video/audio encoding, multiplexing, RTMP/SRT streaming, and MKV recording.

use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Instant;

use parking_lot::Mutex;
use tracing::{error, info, warn};

use crate::encoder::{AudioEncoderSettings, EncoderKind, EncoderProbe, VideoEncoderSettings};
use crate::error::{OutputError, OutputResult};

/// Active destination for the FFmpeg encoding and multiplexing pipeline.
#[derive(Debug, Clone)]
pub struct OutputDestination {
    /// Optional RTMP/RTMPS or SRT streaming endpoint (e.g. "rtmp://127.0.0.1:1935/live/stream")
    pub stream_url: Option<String>,
    /// Optional local MKV disk recording destination path
    pub record_path: Option<PathBuf>,
}

/// The active FFmpeg subprocess encoding and multiplexing pipeline.
pub struct FfmpegPipeline {
    process: Option<Child>,
    video_sender: Option<Arc<Mutex<TcpStream>>>,
    audio_sender: Option<Arc<Mutex<TcpStream>>>,
    video_port: u16,
    audio_port: u16,
    is_running: Arc<AtomicBool>,
    bytes_sent: Arc<AtomicU64>,
    frames_sent: Arc<AtomicU64>,
    destination: OutputDestination,
    video_settings: VideoEncoderSettings,
    audio_settings: AudioEncoderSettings,
    ffmpeg_bin: String,
    start_time: Instant,
}

impl FfmpegPipeline {
    /// Launches an FFmpeg pipeline process with dedicated video and audio TCP feeds.
    pub fn start(
        destination: OutputDestination,
        video_settings: VideoEncoderSettings,
        audio_settings: AudioEncoderSettings,
    ) -> OutputResult<Self> {
        let ffmpeg_bin = EncoderProbe::find_ffmpeg().ok_or_else(|| {
            OutputError::FfmpegNotFound(
                "Neither 'ffmpeg' on PATH nor 'C:\\ffmpeg\\bin\\ffmpeg.exe' was located".into(),
            )
        })?;

        // Allocate dynamic ephemeral loopback ports for uncompressed video and audio ingest
        let video_listener = TcpListener::bind("127.0.0.1:0")?;
        let video_port = video_listener.local_addr()?.port();

        let audio_listener = TcpListener::bind("127.0.0.1:0")?;
        let audio_port = audio_listener.local_addr()?.port();

        info!(
            "Starting FFmpeg pipeline on video port {} and audio port {}",
            video_port, audio_port
        );

        // Build FFmpeg command arguments
        let mut cmd = Command::new(&ffmpeg_bin);
        cmd.arg("-hide_banner")
            .arg("-loglevel")
            .arg("warning")
            .arg("-y");

        // Input 0: Raw NV12 Video Stream
        cmd.args([
            "-f",
            "rawvideo",
            "-pix_fmt",
            "nv12",
            "-s:v",
            &format!("{}x{}", video_settings.width, video_settings.height),
            "-r",
            &format!("{}", video_settings.fps),
            "-i",
            &format!("tcp://127.0.0.1:{}", video_port),
        ]);

        // Input 1: Raw 32-bit Float Stereo Audio Stream
        cmd.args([
            "-f",
            "f32le",
            "-ar",
            &format!("{}", audio_settings.sample_rate),
            "-ac",
            &format!("{}", audio_settings.channels),
            "-i",
            &format!("tcp://127.0.0.1:{}", audio_port),
        ]);

        // Video Encoding Parameters
        let codec_name = video_settings.encoder_kind.ffmpeg_codec_name();
        cmd.args(["-c:v", codec_name]);

        let preset_arg = video_settings
            .preset
            .as_ffmpeg_arg(video_settings.encoder_kind);
        cmd.args(["-preset", preset_arg]);

        let gop = video_settings.gop_size();
        cmd.args(["-g", &format!("{}", gop)]);

        // Bitrate and buffer size
        let b_str = format!("{}k", video_settings.bitrate_kbps);
        cmd.args(["-b:v", &b_str]);

        if let Some(max_b) = video_settings.max_bitrate_kbps {
            cmd.args([
                "-maxrate",
                &format!("{}k", max_b),
                "-bufsize",
                &format!("{}k", max_b * 2),
            ]);
        } else {
            cmd.args(["-bufsize", &format!("{}k", video_settings.bitrate_kbps * 2)]);
        }

        // Color and pixel format
        if video_settings.encoder_kind == EncoderKind::X264 {
            cmd.args([
                "-profile:v",
                video_settings.profile.as_str(),
                "-pix_fmt",
                "yuv420p",
            ]);
        }

        // Audio Encoding Parameters
        cmd.args([
            "-c:a",
            "aac",
            "-b:a",
            &format!("{}k", audio_settings.bitrate_kbps),
            "-ar",
            &format!("{}", audio_settings.sample_rate),
            "-ac",
            &format!("{}", audio_settings.channels),
        ]);

        // Configure Outputs (Stream and/or Local Record)
        let mut has_output = false;

        if let Some(ref rtmp_url) = destination.stream_url {
            if rtmp_url == "null" || rtmp_url.starts_with("mock://") {
                cmd.args(["-f", "null", "-"]);
            } else {
                let format = if rtmp_url.starts_with("srt://") {
                    "mpegts"
                } else {
                    "flv"
                };
                cmd.args(["-f", format, rtmp_url]);
            }
            has_output = true;
        }

        if let Some(ref rec_path) = destination.record_path {
            if let Some(parent) = rec_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            cmd.args(["-f", "matroska", rec_path.to_str().unwrap_or("output.mkv")]);
            has_output = true;
        }

        if !has_output {
            return Err(OutputError::InvalidParameters(
                "At least one stream_url or record_path must be specified".into(),
            ));
        }

        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());

        let mut child = cmd.spawn().map_err(|e| OutputError::EncoderFailed {
            encoder: codec_name.to_string(),
            details: format!("Failed to spawn FFmpeg process: {}", e),
        })?;

        // Accept connection from FFmpeg for Video
        let (mut video_stream, _) = video_listener.accept().map_err(|e| {
            let _ = child.kill();
            OutputError::Io(e)
        })?;
        video_stream.set_nodelay(true)?;

        // Send 1 initial blank NV12 frame to satisfy FFmpeg's stream probe for Input 0
        let initial_frame_size = (video_settings.width * video_settings.height * 3 / 2) as usize;
        let mut initial_nv12 = vec![16u8; initial_frame_size]; // Y = 16 (black)
        let uv_start = (video_settings.width * video_settings.height) as usize;
        initial_nv12[uv_start..].fill(128); // U/V = 128 (neutral chroma)
        let _ = video_stream.write_all(&initial_nv12);
        let _ = video_stream.flush();

        // Accept connection from FFmpeg for Audio
        let (mut audio_stream, _) = audio_listener.accept().map_err(|e| {
            let _ = child.kill();
            OutputError::Io(e)
        })?;
        audio_stream.set_nodelay(true)?;

        // Send 1 initial audio chunk to satisfy FFmpeg's stream probe for Input 1
        let initial_audio = vec![0.0f32; 1024 * audio_settings.channels as usize];
        let audio_bytes = bytemuck::cast_slice::<f32, u8>(&initial_audio);
        let _ = audio_stream.write_all(audio_bytes);
        let _ = audio_stream.flush();

        Ok(Self {
            process: Some(child),
            video_sender: Some(Arc::new(Mutex::new(video_stream))),
            audio_sender: Some(Arc::new(Mutex::new(audio_stream))),
            video_port,
            audio_port,
            is_running: Arc::new(AtomicBool::new(true)),
            bytes_sent: Arc::new(AtomicU64::new(0)),
            frames_sent: Arc::new(AtomicU64::new(0)),
            destination,
            video_settings,
            audio_settings,
            ffmpeg_bin,
            start_time: Instant::now(),
        })
    }

    /// Returns the pipeline's configured output destination.
    pub fn destination(&self) -> &OutputDestination {
        &self.destination
    }

    /// Returns the active video encoder settings.
    pub fn video_settings(&self) -> &VideoEncoderSettings {
        &self.video_settings
    }

    /// Returns the active audio encoder settings.
    pub fn audio_settings(&self) -> &AudioEncoderSettings {
        &self.audio_settings
    }

    /// Returns the FFmpeg executable path.
    pub fn ffmpeg_bin(&self) -> &str {
        &self.ffmpeg_bin
    }

    /// Returns the internal video stream TCP loopback port.
    pub fn video_port(&self) -> u16 {
        self.video_port
    }

    /// Returns the internal audio stream TCP loopback port.
    pub fn audio_port(&self) -> u16 {
        self.audio_port
    }

    /// Writes raw NV12 video bytes into the video ingest socket.
    pub fn write_video_frame(&self, nv12_bytes: &[u8]) -> OutputResult<()> {
        if let Some(ref sender) = self.video_sender {
            let mut stream = sender.lock();
            stream.write_all(nv12_bytes)?;
            self.bytes_sent
                .fetch_add(nv12_bytes.len() as u64, Ordering::Relaxed);
            self.frames_sent.fetch_add(1, Ordering::Relaxed);
            Ok(())
        } else {
            Err(OutputError::Channel("Video stream closed".into()))
        }
    }

    /// Writes raw interleaved 32-bit float stereo audio samples into the audio ingest socket.
    pub fn write_audio_samples(&self, samples: &[f32]) -> OutputResult<()> {
        if let Some(ref sender) = self.audio_sender {
            let byte_slice = bytemuck::cast_slice::<f32, u8>(samples);
            let mut stream = sender.lock();
            stream.write_all(byte_slice)?;
            self.bytes_sent
                .fetch_add(byte_slice.len() as u64, Ordering::Relaxed);
            Ok(())
        } else {
            Err(OutputError::Channel("Audio stream closed".into()))
        }
    }

    /// Checks if the FFmpeg child process is still actively running.
    pub fn is_alive(&mut self) -> bool {
        if let Some(ref mut child) = self.process {
            match child.try_wait() {
                Ok(None) => true,
                Ok(Some(status)) => {
                    warn!("FFmpeg pipeline exited with status: {}", status);
                    false
                }
                Err(e) => {
                    error!("Error polling FFmpeg process: {}", e);
                    false
                }
            }
        } else {
            false
        }
    }

    /// Stops and terminates the FFmpeg encoding pipeline cleanly.
    pub fn stop(&mut self) -> OutputResult<()> {
        self.is_running.store(false, Ordering::SeqCst);

        // Explicitly shutdown sockets to send FIN packet to FFmpeg immediately
        if let Some(ref sender) = self.video_sender {
            let stream = sender.lock();
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
        if let Some(ref sender) = self.audio_sender {
            let stream = sender.lock();
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }

        self.video_sender = None;
        self.audio_sender = None;

        if let Some(mut child) = self.process.take() {
            // Wait up to 5 seconds for clean container finalize
            let start = Instant::now();
            loop {
                match child.try_wait() {
                    Ok(Some(_)) => break,
                    Ok(None) => {
                        if start.elapsed().as_secs() > 5 {
                            let _ = child.kill();
                            let _ = child.wait();
                            break;
                        }
                        thread::sleep(std::time::Duration::from_millis(50));
                    }
                    Err(_) => {
                        let _ = child.kill();
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Returns the total bytes sent through the pipeline.
    pub fn total_bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    /// Returns the total video frames dispatched.
    pub fn total_frames_sent(&self) -> u64 {
        self.frames_sent.load(Ordering::Relaxed)
    }

    /// Elapsed pipeline runtime in seconds.
    pub fn elapsed_seconds(&self) -> f64 {
        self.start_time.elapsed().as_secs_f64()
    }

    /// Remuxes an MKV file to MP4 using lossless stream copy (`-c copy`).
    pub fn remux_mkv_to_mp4(mkv_path: &Path, mp4_path: &Path) -> OutputResult<()> {
        let ffmpeg_bin = EncoderProbe::find_ffmpeg().ok_or_else(|| {
            OutputError::FfmpegNotFound("FFmpeg binary required for remuxing".into())
        })?;

        let status = Command::new(&ffmpeg_bin)
            .args([
                "-y",
                "-i",
                mkv_path.to_str().unwrap_or(""),
                "-c",
                "copy",
                mp4_path.to_str().unwrap_or(""),
            ])
            .output()?;

        if !status.status.success() {
            let stderr = String::from_utf8_lossy(&status.stderr);
            return Err(OutputError::RemuxFailed(format!(
                "FFmpeg remux failed: {}",
                stderr
            )));
        }

        info!(
            "Successfully remuxed '{}' to '{}'",
            mkv_path.display(),
            mp4_path.display()
        );
        Ok(())
    }
}

impl Drop for FfmpegPipeline {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}
