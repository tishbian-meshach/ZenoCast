//! Milestone M7b Acceptance Tests: YouTube Live Integration.
//!
//! Verifies:
//! 1. One-click Go Live lifecycle with mock API (create -> bind -> rtmps handoff -> transition live -> complete)
//! 2. Revoked/expired token diagnostics
//! 3. Channel not activated (liveStreamingNotEnabled) diagnostics
//! 4. API Quota exceeded diagnostics
//! 5. Preflight validation & YouTube bitrate table lookups
//! 6. Manual stream key fallback bypass
//! 7. PKCE code verifier and challenge generation

use std::sync::Arc;
use std::time::Duration;

use zeno_output::encoder::{
    AudioEncoderSettings, EncoderKind, EncoderPreset, EncoderProfile, RateControl,
    VideoEncoderSettings,
};
use zeno_output::service::OutputManager;
use zeno_output::youtube::auth::{OAuthToken, PkcePair, TokenStorage};
use zeno_output::youtube::broadcast::YouTubeBroadcastManager;
use zeno_output::youtube::client::{
    BroadcastPrivacy, CreateBroadcastParams, LatencyPreference, YouTubeApiClient,
};
use zeno_output::youtube::preflight::{PreflightValidator, YouTubeBitrateTable};

#[test]
fn test_youtube_pkce_generation() {
    let pkce = PkcePair::generate();
    assert!(pkce.code_verifier.len() >= 43 && pkce.code_verifier.len() <= 128);
    assert!(!pkce.code_challenge.is_empty());
    assert_eq!(pkce.code_challenge_method, "S256");

    // Characters in verifier must be unreserved URL characters
    for b in pkce.code_verifier.bytes() {
        assert!(
            b.is_ascii_alphanumeric() || b == b'-' || b == b'.' || b == b'_' || b == b'~',
            "Invalid character in PKCE verifier"
        );
    }
}

#[test]
fn test_youtube_bitrate_table_and_preflight() {
    // 1080p60 lookup
    let rec_1080p60 = YouTubeBitrateTable::lookup(1920, 1080, 60.0);
    assert_eq!(rec_1080p60.min_kbps, 4500);
    assert_eq!(rec_1080p60.max_kbps, 9000);
    assert_eq!(rec_1080p60.default_kbps, 6000);

    // 4K60 lookup
    let rec_4k60 = YouTubeBitrateTable::lookup(3840, 2160, 60.0);
    assert_eq!(rec_4k60.min_kbps, 20000);
    assert_eq!(rec_4k60.max_kbps, 51000);

    // 720p30 lookup
    let rec_720p30 = YouTubeBitrateTable::lookup(1280, 720, 30.0);
    assert_eq!(rec_720p30.min_kbps, 1500);
    assert_eq!(rec_720p30.max_kbps, 4000);

    let v_settings = VideoEncoderSettings {
        encoder_kind: EncoderKind::X264,
        width: 1920,
        height: 1080,
        fps: 60.0,
        rate_control: RateControl::Cbr,
        bitrate_kbps: 6000,
        max_bitrate_kbps: Some(6000),
        keyframe_interval_seconds: 2.0,
        preset: EncoderPreset::VeryFast,
        profile: EncoderProfile::Main,
        b_frames: 0,
    };

    let a_settings = AudioEncoderSettings {
        sample_rate: 48000,
        channels: 2,
        bitrate_kbps: 160,
    };

    // Ready preflight
    let report_ok = PreflightValidator::validate(true, true, &v_settings, &a_settings);
    assert!(report_ok.can_go_live);
    assert!(report_ok.errors.is_empty());

    // Unauthenticated preflight
    let report_unauth = PreflightValidator::validate(false, true, &v_settings, &a_settings);
    assert!(!report_unauth.can_go_live);
    assert!(report_unauth.errors[0].contains("Not signed in to YouTube"));

    // Channel not enabled preflight
    let report_unenabled = PreflightValidator::validate(true, false, &v_settings, &a_settings);
    assert!(!report_unenabled.can_go_live);
    assert!(report_unenabled.errors[0].contains("not enabled for live streaming"));

    // Audio mismatch warning
    let bad_audio = AudioEncoderSettings {
        sample_rate: 44100,
        channels: 1,
        bitrate_kbps: 128,
    };
    let report_audio = PreflightValidator::validate(true, true, &v_settings, &bad_audio);
    assert!(report_audio.can_go_live); // Non-blocking warning
    assert!(report_audio
        .warnings
        .iter()
        .any(|w| w.contains("48 kHz stereo")));
}

#[test]
fn test_youtube_api_client_error_diagnostics() {
    // 1. Expired / Revoked Token
    let mock_expired = YouTubeApiClient::new_mock(|_method, _url, _body| {
        Ok((
            401,
            r#"{"error": {"code": 401, "message": "Request had invalid authentication credentials. Expected OAuth 2 access token, login cookie or other valid credential.", "status": "UNAUTHENTICATED"}}"#.into(),
        ))
    });

    let params = CreateBroadcastParams {
        title: "Test Stream".into(),
        description: "Test Description".into(),
        privacy: BroadcastPrivacy::Private,
        latency: LatencyPreference::Low,
        enable_dvr: true,
        made_for_kids: false,
        scheduled_start_time: None,
    };

    let res = mock_expired.insert_broadcast("expired-token", &params);
    assert!(res.is_err());
    let err_msg = format!("{}", res.err().unwrap());
    assert!(
        err_msg.contains("expired or revoked"),
        "Unexpected error: {}",
        err_msg
    );

    // 2. Channel Not Activated (liveStreamingNotEnabled)
    let mock_unactivated = YouTubeApiClient::new_mock(|_method, _url, _body| {
        Ok((
            403,
            r#"{"error": {"errors": [{"reason": "liveStreamingNotEnabled", "message": "The user is not enabled for live streaming."}], "code": 403, "message": "The user is not enabled for live streaming."}}"#.into(),
        ))
    });

    let res_channel = mock_unactivated.insert_broadcast("valid-token", &params);
    assert!(res_channel.is_err());
    let err_msg = format!("{}", res_channel.err().unwrap());
    assert!(
        err_msg.contains("Activation can take up to 24 hours"),
        "Unexpected error: {}",
        err_msg
    );

    // 3. Quota Exceeded
    let mock_quota = YouTubeApiClient::new_mock(|_method, _url, _body| {
        Ok((
            403,
            r#"{"error": {"errors": [{"reason": "quotaExceeded", "message": "The request cannot be completed because you have exceeded your quota."}], "code": 403, "message": "Quota exceeded"}}"#.into(),
        ))
    });

    let res_quota = mock_quota.insert_broadcast("valid-token", &params);
    assert!(res_quota.is_err());
    let err_msg = format!("{}", res_quota.err().unwrap());
    assert!(
        err_msg.contains("quota exceeded"),
        "Unexpected error: {}",
        err_msg
    );
}

#[test]
fn test_manual_stream_key_fallback() {
    let client = YouTubeApiClient::new();
    let token_storage = TokenStorage::new(); // empty, not signed in

    let v_settings = VideoEncoderSettings {
        encoder_kind: EncoderKind::X264,
        width: 1280,
        height: 720,
        fps: 30.0,
        rate_control: RateControl::Cbr,
        bitrate_kbps: 3000,
        max_bitrate_kbps: Some(3000),
        keyframe_interval_seconds: 2.0,
        preset: EncoderPreset::VeryFast,
        profile: EncoderProfile::Main,
        b_frames: 0,
    };
    let a_settings = AudioEncoderSettings {
        sample_rate: 48000,
        channels: 2,
        bitrate_kbps: 128,
    };

    let output_manager = Arc::new(OutputManager::new_dry_run(
        v_settings.clone(),
        a_settings.clone(),
    ));
    let mut broadcast_mgr = YouTubeBroadcastManager::new(client, token_storage, output_manager);

    // Set manual stream key
    broadcast_mgr.set_manual_stream_key(Some("xxxx-yyyy-zzzz-1234".into()));

    let params = CreateBroadcastParams {
        title: "Manual Key Stream".into(),
        description: "".into(),
        privacy: BroadcastPrivacy::Unlisted,
        latency: LatencyPreference::Normal,
        enable_dvr: false,
        made_for_kids: false,
        scheduled_start_time: None,
    };

    let status = broadcast_mgr
        .go_live(params, &v_settings, &a_settings)
        .expect("Manual key go_live should succeed");

    assert!(status.is_on_air);
    assert_eq!(status.lifecycle_state, "live");
    assert_eq!(
        status.rtmps_url,
        Some("rtmp://a.rtmp.youtube.com/live2/xxxx-yyyy-zzzz-1234".into())
    );

    // End broadcast
    broadcast_mgr.end_broadcast().expect("end_broadcast failed");
    let end_status = broadcast_mgr.status();
    assert!(!end_status.is_on_air);
    assert_eq!(end_status.lifecycle_state, "complete");
}

#[test]
fn test_one_click_go_live_lifecycle_mocked() {
    let mock_client = YouTubeApiClient::new_mock(|method, url, _body| {
        if url.contains("liveBroadcasts?part=snippet") && method == "POST" {
            Ok((
                200,
                r#"{
                    "id": "broadcast_test_99",
                    "snippet": { "title": "Championship Grand Final" },
                    "status": { "lifeCycleStatus": "created" }
                }"#
                .into(),
            ))
        } else if url.contains("liveStreams?part=snippet") && method == "POST" {
            Ok((
                200,
                r#"{
                    "id": "stream_test_88",
                    "cdn": {
                        "ingestionInfo": {
                            "ingestionAddress": "rtmp://a.rtmp.youtube.com/live2",
                            "streamName": "secret-ingest-key"
                        }
                    },
                    "status": { "streamStatus": "ready" }
                }"#
                .into(),
            ))
        } else if url.contains("liveBroadcasts/bind") && method == "POST" {
            Ok((
                200,
                r#"{
                    "id": "broadcast_test_99",
                    "status": { "lifeCycleStatus": "ready" }
                }"#
                .into(),
            ))
        } else if url.contains("liveBroadcasts/transition") && method == "POST" {
            let target_status = if url.contains("broadcastStatus=live") {
                "live"
            } else if url.contains("broadcastStatus=complete") {
                "complete"
            } else {
                "testing"
            };
            Ok((
                200,
                format!(
                    r#"{{ "id": "broadcast_test_99", "status": {{ "lifeCycleStatus": "{}" }} }}"#,
                    target_status
                ),
            ))
        } else if url.contains("liveStreams?part=status") && method == "GET" {
            Ok((
                200,
                r#"{
                    "items": [
                        { "status": { "streamStatus": "active" } }
                    ]
                }"#
                .into(),
            ))
        } else if url.contains("liveBroadcasts?part=statistics") && method == "GET" {
            Ok((
                200,
                r#"{
                    "items": [
                        { "statistics": { "concurrentViewers": "1337" } }
                    ]
                }"#
                .into(),
            ))
        } else {
            Ok((200, "{}".into()))
        }
    });

    let token_storage = TokenStorage::new();
    token_storage
        .store(OAuthToken {
            access_token: "mock-valid-access-token".into(),
            refresh_token: Some("mock-refresh-token".into()),
            expires_at: 9999999999, // far future
        })
        .unwrap();

    let v_settings = VideoEncoderSettings {
        encoder_kind: EncoderKind::X264,
        width: 1920,
        height: 1080,
        fps: 60.0,
        rate_control: RateControl::Cbr,
        bitrate_kbps: 6000,
        max_bitrate_kbps: Some(6000),
        keyframe_interval_seconds: 2.0,
        preset: EncoderPreset::VeryFast,
        profile: EncoderProfile::Main,
        b_frames: 0,
    };
    let a_settings = AudioEncoderSettings {
        sample_rate: 48000,
        channels: 2,
        bitrate_kbps: 160,
    };

    let output_manager = Arc::new(OutputManager::new_dry_run(
        v_settings.clone(),
        a_settings.clone(),
    ));
    let mut broadcast_mgr =
        YouTubeBroadcastManager::new(mock_client, token_storage, output_manager);

    let params = CreateBroadcastParams {
        title: "Championship Grand Final".into(),
        description: "Live esports broadcast".into(),
        privacy: BroadcastPrivacy::Public,
        latency: LatencyPreference::UltraLow,
        enable_dvr: true,
        made_for_kids: false,
        scheduled_start_time: None,
    };

    // 1. One-click Go Live
    let live_status = broadcast_mgr
        .go_live(params, &v_settings, &a_settings)
        .expect("go_live should succeed");

    assert!(live_status.is_on_air);
    assert_eq!(live_status.lifecycle_state, "live");
    assert_eq!(live_status.broadcast_id, Some("broadcast_test_99".into()));
    assert_eq!(live_status.stream_id, Some("stream_test_88".into()));
    assert_eq!(
        live_status.rtmps_url,
        Some("rtmp://a.rtmp.youtube.com/live2/secret-ingest-key".into())
    );

    // Give the background health poller 100ms to poll
    std::thread::sleep(Duration::from_millis(100));

    let polled_status = broadcast_mgr.status();
    assert_eq!(polled_status.viewer_count, 1337);
    assert_eq!(polled_status.stream_health, "active");

    // 2. Cleanly End Broadcast
    broadcast_mgr
        .end_broadcast()
        .expect("end_broadcast should cleanly succeed");

    let final_status = broadcast_mgr.status();
    assert!(!final_status.is_on_air);
    assert_eq!(final_status.lifecycle_state, "complete");
    assert_eq!(final_status.stream_health, "offline");
}
