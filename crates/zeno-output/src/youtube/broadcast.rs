//! High-level YouTube Live broadcast manager: one-click Go Live, lifecycle transitions, and health polling.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use tracing::info;

use crate::encoder::{AudioEncoderSettings, VideoEncoderSettings};
use crate::error::{OutputError, OutputResult};
use crate::service::OutputManager;
use crate::youtube::auth::TokenStorage;
use crate::youtube::client::{CreateBroadcastParams, YouTubeApiClient};
use crate::youtube::preflight::PreflightValidator;

/// Active broadcast status and metrics for UI display.
#[derive(Debug, Clone)]
pub struct YouTubeLiveStatus {
    /// Active broadcast ID if live or testing
    pub broadcast_id: Option<String>,
    /// Active stream ID
    pub stream_id: Option<String>,
    /// RTMPS ingestion target address
    pub rtmps_url: Option<String>,
    /// Broadcast lifecycle state ("ready", "testing", "live", "complete")
    pub lifecycle_state: String,
    /// Concurrent viewer count
    pub viewer_count: u64,
    /// Stream health status ("good", "ok", "bad", "no_data")
    pub stream_health: String,
    /// Whether currently streaming to YouTube
    pub is_on_air: bool,
}

impl Default for YouTubeLiveStatus {
    fn default() -> Self {
        Self {
            broadcast_id: None,
            stream_id: None,
            rtmps_url: None,
            lifecycle_state: "ready".into(),
            viewer_count: 0,
            stream_health: "ready".into(),
            is_on_air: false,
        }
    }
}

/// One-click YouTube Live broadcast orchestrator.
pub struct YouTubeBroadcastManager {
    client: YouTubeApiClient,
    token_storage: TokenStorage,
    output_manager: Arc<OutputManager>,
    status: Arc<Mutex<YouTubeLiveStatus>>,
    monitor_running: Arc<AtomicBool>,
    monitor_handle: Option<JoinHandle<()>>,
    manual_stream_key: Arc<Mutex<Option<String>>>,
    concurrent_viewers: Arc<AtomicU64>,
}

impl YouTubeBroadcastManager {
    /// Creates a YouTube broadcast manager tied to the live output manager.
    pub fn new(
        client: YouTubeApiClient,
        token_storage: TokenStorage,
        output_manager: Arc<OutputManager>,
    ) -> Self {
        Self {
            client,
            token_storage,
            output_manager,
            status: Arc::new(Mutex::new(YouTubeLiveStatus::default())),
            monitor_running: Arc::new(AtomicBool::new(false)),
            monitor_handle: None,
            manual_stream_key: Arc::new(Mutex::new(None)),
            concurrent_viewers: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Configures a manual stream key fallback.
    pub fn set_manual_stream_key(&self, stream_key: Option<String>) {
        *self.manual_stream_key.lock() = stream_key;
    }

    /// One-click Go Live pipeline: preflight -> insert broadcast & stream -> bind -> stream RTMPS -> transition live.
    pub fn go_live(
        &mut self,
        params: CreateBroadcastParams,
        video_settings: &VideoEncoderSettings,
        audio_settings: &AudioEncoderSettings,
    ) -> OutputResult<YouTubeLiveStatus> {
        // Fallback: If operator specified a manual stream key, bypass OAuth API and stream directly to YouTube RTMP
        if let Some(ref key) = *self.manual_stream_key.lock() {
            let rtmp_url = format!("rtmp://a.rtmp.youtube.com/live2/{}", key);
            info!("Going live using manual YouTube stream key fallback");
            self.output_manager.start_streaming(&rtmp_url)?;

            let mut s = self.status.lock();
            s.rtmps_url = Some(rtmp_url);
            s.lifecycle_state = "live".into();
            s.is_on_air = true;
            return Ok(s.clone());
        }

        // 1. Preflight Validation
        let token = self.token_storage.load().ok_or_else(|| {
            OutputError::InvalidParameters(
                "Not signed in to YouTube. Please sign in via Settings > YouTube.".into(),
            )
        })?;

        let preflight = PreflightValidator::validate(true, true, video_settings, audio_settings);
        if !preflight.can_go_live {
            let error_details = preflight.errors.join("; ");
            return Err(OutputError::InvalidParameters(format!(
                "YouTube Go Live preflight check failed: {}",
                error_details
            )));
        }

        // 2. Create Broadcast Resource
        let broadcast = self.client.insert_broadcast(&token.access_token, &params)?;
        info!("Created YouTube broadcast: id='{}'", broadcast.id);

        // 3. Create Live Ingestion Stream
        let stream = self
            .client
            .insert_stream(&token.access_token, &params.title)?;
        info!(
            "Created YouTube stream: id='{}', ingest='{}'",
            stream.id, stream.ingestion_address
        );

        // 4. Bind Broadcast to Stream
        self.client
            .bind_broadcast(&token.access_token, &broadcast.id, &stream.id)?;
        info!(
            "Bound YouTube broadcast '{}' to stream '{}'",
            broadcast.id, stream.id
        );

        // 5. Hand off RTMPS Ingestion Target to OutputManager
        let rtmps_url = stream.full_rtmps_url();
        info!("Handoff ingestion address to encoder: '{}'", rtmps_url);
        self.output_manager.start_streaming(&rtmps_url)?;

        // 6. Transition to Live on Air
        // In live broadcast, transition through testing to live
        let _ = self
            .client
            .transition_broadcast(&token.access_token, &broadcast.id, "testing");
        let live_status =
            self.client
                .transition_broadcast(&token.access_token, &broadcast.id, "live")?;
        info!(
            "YouTube broadcast '{}' transitioned to '{}'",
            broadcast.id, live_status
        );

        // Update active status
        {
            let mut s = self.status.lock();
            s.broadcast_id = Some(broadcast.id.clone());
            s.stream_id = Some(stream.id.clone());
            s.rtmps_url = Some(rtmps_url);
            s.lifecycle_state = live_status;
            s.stream_health = "good".into();
            s.is_on_air = true;
        }

        // 7. Start Health and Viewer Poller (every 5 seconds, quota compliant)
        self.start_monitoring(broadcast.id, stream.id, token.access_token);

        Ok(self.status.lock().clone())
    }

    /// Starts background status and viewer polling respecting YouTube quota.
    fn start_monitoring(&mut self, broadcast_id: String, stream_id: String, access_token: String) {
        self.monitor_running.store(true, Ordering::SeqCst);
        let flag = Arc::clone(&self.monitor_running);
        let status = Arc::clone(&self.status);
        let viewers_atomic = Arc::clone(&self.concurrent_viewers);
        let client = self.client.clone();

        let handle = thread::spawn(move || {
            let mut last_poll = Instant::now()
                .checked_sub(Duration::from_secs(5))
                .unwrap_or_else(Instant::now);

            while flag.load(Ordering::Relaxed) {
                // Poll every 5 seconds per Section 10C specification
                if last_poll.elapsed() >= Duration::from_secs(5) {
                    if let Ok(v) = client.get_concurrent_viewers(&access_token, &broadcast_id) {
                        viewers_atomic.store(v, Ordering::Relaxed);
                        let mut s = status.lock();
                        s.viewer_count = v;
                    }

                    if let Ok(st) = client.get_stream_status(&access_token, &stream_id) {
                        let mut s = status.lock();
                        s.stream_health = st;
                    }

                    last_poll = Instant::now();
                }

                thread::sleep(Duration::from_millis(50));
            }
        });

        self.monitor_handle = Some(handle);
    }

    /// Stops broadcasting, halts stream output, and marks the YouTube broadcast as complete.
    pub fn end_broadcast(&mut self) -> OutputResult<()> {
        // Stop health poller
        self.monitor_running.store(false, Ordering::SeqCst);
        if let Some(handle) = self.monitor_handle.take() {
            let _ = handle.join();
        }

        // Stop encoder output
        self.output_manager.stop_streaming();

        // Mark broadcast as complete via API
        let broadcast_id = self.status.lock().broadcast_id.clone();
        if let Some(id) = broadcast_id {
            if let Some(token) = self.token_storage.load() {
                let _ = self
                    .client
                    .transition_broadcast(&token.access_token, &id, "complete");
                info!("Transitioned YouTube broadcast '{}' to complete", id);
            }
        }

        let mut s = self.status.lock();
        s.lifecycle_state = "complete".into();
        s.is_on_air = false;
        s.stream_health = "offline".into();
        info!("Cleanly ended YouTube live broadcast");
        Ok(())
    }

    /// Current broadcast status snapshot.
    pub fn status(&self) -> YouTubeLiveStatus {
        self.status.lock().clone()
    }
}

impl Drop for YouTubeBroadcastManager {
    fn drop(&mut self) {
        let _ = self.end_broadcast();
    }
}
