//! YouTube Live Streaming API v3 HTTP client with Windows Schannel support and specific error diagnostics.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::encoder::create_hidden_command;
use crate::error::{OutputError, OutputResult};

/// Latency mode for YouTube broadcast stream ingestion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LatencyPreference {
    Normal,
    Low,
    UltraLow,
}

impl LatencyPreference {
    /// Returns YouTube API string value for latency preference.
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Low => "low",
            Self::UltraLow => "ultraLow",
        }
    }
}

/// Privacy status for a YouTube live broadcast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BroadcastPrivacy {
    Public,
    Unlisted,
    Private,
}

impl BroadcastPrivacy {
    /// Returns YouTube API string value for privacy status.
    pub fn as_api_str(&self) -> &'static str {
        match self {
            Self::Public => "public",
            Self::Unlisted => "unlisted",
            Self::Private => "private",
        }
    }
}

/// Parameters for creating a new YouTube Live broadcast.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBroadcastParams {
    pub title: String,
    pub description: String,
    pub privacy: BroadcastPrivacy,
    pub latency: LatencyPreference,
    pub enable_dvr: bool,
    pub made_for_kids: bool,
    pub scheduled_start_time: Option<String>,
}

/// Created YouTube broadcast resource summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeBroadcast {
    pub id: String,
    pub title: String,
    pub status: String,
    pub bound_stream_id: Option<String>,
}

/// Created YouTube stream resource summary with ingestion info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YouTubeStream {
    pub id: String,
    pub ingestion_address: String,
    pub stream_name: String,
    pub status: String,
}

impl YouTubeStream {
    /// Constructs the full RTMPS ingestion target URL for the encoder.
    pub fn full_rtmps_url(&self) -> String {
        format!(
            "{}/{}",
            self.ingestion_address.trim_end_matches('/'),
            self.stream_name
        )
    }
}

use std::sync::Arc;

/// Mock response handler type for offline testing.
pub type MockHttpHandler =
    Arc<dyn Fn(&str, &str, Option<&str>) -> Result<(u16, String), OutputError> + Send + Sync>;

/// Client for communicating with the YouTube Data API v3.
#[derive(Clone)]
pub struct YouTubeApiClient {
    api_base_url: String,
    mock_handler: Option<MockHttpHandler>,
}

impl Default for YouTubeApiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl YouTubeApiClient {
    /// Creates a production API client targeting Google's live endpoints.
    pub fn new() -> Self {
        Self {
            api_base_url: "https://www.googleapis.com/youtube/v3".to_string(),
            mock_handler: None,
        }
    }

    /// Creates a test client with a mock response handler for offline validation.
    pub fn new_mock(
        handler: impl Fn(&str, &str, Option<&str>) -> Result<(u16, String), OutputError>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self {
            api_base_url: "http://mock.youtube.local".to_string(),
            mock_handler: Some(Arc::new(handler)),
        }
    }

    /// Dispatches an authenticated HTTP request using curl with Windows Schannel or the mock handler.
    fn request(
        &self,
        method: &str,
        endpoint: &str,
        access_token: &str,
        body: Option<&str>,
    ) -> OutputResult<Value> {
        let url = if endpoint.starts_with("http") {
            endpoint.to_string()
        } else {
            format!(
                "{}/{}",
                self.api_base_url.trim_end_matches('/'),
                endpoint.trim_start_matches('/')
            )
        };

        let (status_code, response_body) = if let Some(ref mock) = self.mock_handler {
            mock(method, &url, body)?
        } else {
            let mut cmd = create_hidden_command("curl.exe");
            cmd.args(["-s", "-w", "\n%{http_code}", "-X", method, &url]);
            cmd.args(["-H", &format!("Authorization: Bearer {}", access_token)]);
            cmd.args(["-H", "Content-Type: application/json"]);

            if let Some(payload) = body {
                cmd.args(["-d", payload]);
            }

            let output = cmd.output()?;
            let full_out = String::from_utf8_lossy(&output.stdout);

            let mut lines = full_out.lines().collect::<Vec<_>>();
            let status_code_str = lines.pop().unwrap_or("0");
            let status_code = status_code_str.trim().parse::<u16>().unwrap_or(0);
            let body_str = lines.join("\n");

            (status_code, body_str)
        };

        // Specific Error Diagnostics per Section 10C
        if status_code == 401
            || response_body.contains("invalid_grant")
            || response_body.contains("token_expired")
        {
            return Err(OutputError::InvalidParameters(
                "YouTube authentication token is expired or revoked. Please sign in again via Settings > YouTube.".into(),
            ));
        }

        if response_body.contains("liveStreamingNotEnabled") {
            return Err(OutputError::InvalidParameters(
                "YouTube channel is not enabled for live streaming. Activation can take up to 24 hours. Visit https://www.youtube.com/live_dashboard to check status.".into(),
            ));
        }

        if response_body.contains("quotaExceeded") {
            return Err(OutputError::InvalidParameters(
                "YouTube API quota exceeded for the current 24-hour period. Please retry tomorrow or use a manual stream key.".into(),
            ));
        }

        if status_code >= 400 {
            return Err(OutputError::ProcessFailed(format!(
                "YouTube API error (HTTP {}): {}",
                status_code, response_body
            )));
        }

        serde_json::from_str(&response_body).map_err(|e| {
            OutputError::InvalidParameters(format!(
                "Failed to parse YouTube API JSON response: {}",
                e
            ))
        })
    }

    /// Creates a new broadcast resource via `liveBroadcasts.insert`.
    pub fn insert_broadcast(
        &self,
        access_token: &str,
        params: &CreateBroadcastParams,
    ) -> OutputResult<YouTubeBroadcast> {
        let payload = serde_json::json!({
            "snippet": {
                "title": params.title,
                "description": params.description,
                "scheduledStartTime": params.scheduled_start_time.clone().unwrap_or_else(|| "2026-10-17T18:00:00Z".into())
            },
            "status": {
                "privacyStatus": params.privacy.as_api_str(),
                "selfDeclaredMadeForKids": params.made_for_kids
            },
            "contentDetails": {
                "latencyPreference": params.latency.as_api_str(),
                "enableDvr": params.enable_dvr
            }
        });

        let res = self.request(
            "POST",
            "liveBroadcasts?part=snippet,status,contentDetails",
            access_token,
            Some(&payload.to_string()),
        )?;

        let id = res["id"].as_str().unwrap_or("").to_string();
        let title = res["snippet"]["title"].as_str().unwrap_or("").to_string();
        let status = res["status"]["lifeCycleStatus"]
            .as_str()
            .unwrap_or("created")
            .to_string();

        Ok(YouTubeBroadcast {
            id,
            title,
            status,
            bound_stream_id: None,
        })
    }

    /// Creates a live ingestion stream resource via `liveStreams.insert`.
    pub fn insert_stream(&self, access_token: &str, title: &str) -> OutputResult<YouTubeStream> {
        let payload = serde_json::json!({
            "snippet": {
                "title": format!("{} Stream", title)
            },
            "cdn": {
                "frameRate": "60fps",
                "ingestionType": "rtmp",
                "resolution": "1080p"
            }
        });

        let res = self.request(
            "POST",
            "liveStreams?part=snippet,cdn,status",
            access_token,
            Some(&payload.to_string()),
        )?;

        let id = res["id"].as_str().unwrap_or("").to_string();
        let ingestion_address = res["cdn"]["ingestionInfo"]["ingestionAddress"]
            .as_str()
            .unwrap_or("rtmp://a.rtmp.youtube.com/live2")
            .to_string();
        let stream_name = res["cdn"]["ingestionInfo"]["streamName"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let status = res["status"]["streamStatus"]
            .as_str()
            .unwrap_or("ready")
            .to_string();

        Ok(YouTubeStream {
            id,
            ingestion_address,
            stream_name,
            status,
        })
    }

    /// Binds an existing broadcast to a live stream resource via `liveBroadcasts.bind`.
    pub fn bind_broadcast(
        &self,
        access_token: &str,
        broadcast_id: &str,
        stream_id: &str,
    ) -> OutputResult<YouTubeBroadcast> {
        let endpoint = format!(
            "liveBroadcasts/bind?id={}&part=id,contentDetails,status&streamId={}",
            broadcast_id, stream_id
        );

        let res = self.request("POST", &endpoint, access_token, None)?;
        let id = res["id"].as_str().unwrap_or("").to_string();
        let status = res["status"]["lifeCycleStatus"]
            .as_str()
            .unwrap_or("ready")
            .to_string();

        Ok(YouTubeBroadcast {
            id,
            title: "".to_string(),
            status,
            bound_stream_id: Some(stream_id.to_string()),
        })
    }

    /// Transitions a broadcast's lifecycle status (`testing`, `live`, or `complete`).
    pub fn transition_broadcast(
        &self,
        access_token: &str,
        broadcast_id: &str,
        target_status: &str,
    ) -> OutputResult<String> {
        let endpoint = format!(
            "liveBroadcasts/transition?broadcastStatus={}&id={}&part=status",
            target_status, broadcast_id
        );

        let res = self.request("POST", &endpoint, access_token, None)?;
        let new_status = res["status"]["lifeCycleStatus"]
            .as_str()
            .unwrap_or(target_status)
            .to_string();
        Ok(new_status)
    }

    /// Queries live stream status (`active`, `ready`, or `error`).
    pub fn get_stream_status(&self, access_token: &str, stream_id: &str) -> OutputResult<String> {
        let endpoint = format!("liveStreams?part=status&id={}", stream_id);
        let res = self.request("GET", &endpoint, access_token, None)?;

        let items = res["items"].as_array();
        if let Some(items) = items {
            if let Some(first) = items.first() {
                let status = first["status"]["streamStatus"]
                    .as_str()
                    .unwrap_or("ready")
                    .to_string();
                return Ok(status);
            }
        }

        Ok("ready".to_string())
    }

    /// Queries concurrent viewer statistics.
    pub fn get_concurrent_viewers(
        &self,
        access_token: &str,
        broadcast_id: &str,
    ) -> OutputResult<u64> {
        let endpoint = format!("liveBroadcasts?part=statistics&id={}", broadcast_id);
        let res = self.request("GET", &endpoint, access_token, None)?;

        if let Some(items) = res["items"].as_array() {
            if let Some(first) = items.first() {
                let viewers_str = first["statistics"]["concurrentViewers"]
                    .as_str()
                    .unwrap_or("0");
                let viewers = viewers_str.parse::<u64>().unwrap_or(0);
                return Ok(viewers);
            }
        }

        Ok(0)
    }
}
