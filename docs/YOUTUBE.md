# Zenocast: YouTube Live Streaming Integration Guide

This document describes how to configure, authenticate, and broadcast live productions directly to YouTube from Zenocast using the YouTube Live Streaming API v3.

---

## 1. Google Cloud Project & API Configuration

To enable direct in-app sign-in and broadcast management:

1. Visit the [Google Cloud Console](https://console.cloud.google.com/).
2. Create or select a project (e.g., `zenocast-studio`).
3. Navigate to **APIs & Services > Library** and enable:
   - **YouTube Data API v3** (includes YouTube Live Streaming API v3).
4. Configure the **OAuth Consent Screen**:
   - User Type: **External** (or Internal for Google Workspace organizations).
   - Scopes: `https://www.googleapis.com/auth/youtube` (manage YouTube account and live broadcasts).
5. Create **OAuth 2.0 Client ID**:
   - Application Type: **Desktop app**.
   - Name: `Zenocast Desktop`.
   - Download the client configuration or copy `client_id` and `client_secret`.
6. Add the credentials to your Zenocast configuration or environment:
   ```json
   {
     "youtube_oauth": {
       "client_id": "YOUR_CLIENT_ID.apps.googleusercontent.com",
       "client_secret": "YOUR_CLIENT_SECRET"
     }
   }
   ```

---

## 2. Authentication Flow (PKCE + Loopback Redirect)

Zenocast implements the standard OAuth 2.0 Authorization Code Flow with PKCE (Proof Key for Code Exchange) per RFC 7636:

1. **Authorization Request**:
   - Generates high-entropy cryptographic code verifier and code challenge (S256).
   - Binds a temporary HTTP loopback listener on `http://127.0.0.1:0`.
   - Opens the default system web browser to Google's OAuth consent endpoint.
2. **Loopback Callback**:
   - Receives the authorization code and exchanges it with Google for access and refresh tokens.
   - Closes the temporary HTTP listener.
3. **Secure Token Storage**:
   - Tokens are stored securely in Windows Credential Manager using the `keyring` crate under service `zenocast` and account `youtube_oauth_token`.
   - Raw tokens are never written to unencrypted JSON settings or disk logs.
4. **Automatic Refresh**:
   - Access tokens expire after 1 hour. Zenocast automatically uses the refresh token to acquire a fresh access token without user intervention.
5. **Sign-out**:
   - Clicking "Sign Out" revokes the token and purges credentials from Windows Credential Manager.

---

## 3. Broadcast Lifecycle & YouTube Live Streaming API v3

The in-app "Go Live" pipeline performs the full API lifecycle:

```
[Create Broadcast] (liveBroadcasts.insert)
        │
        ▼
[Create Stream] (liveStreams.insert)
        │
        ▼
[Bind Broadcast to Stream] (liveBroadcasts.bind)
        │
        ▼
[Start Hardware Encoder] -> RTMPS Ingestion Address (cdn.ingestionInfo)
        │
        ▼
[Poll Stream Status] (liveStreams.list -> status.streamStatus == "active")
        │
        ▼
[Transition to Testing] (liveBroadcasts.transition?broadcastStatus=testing)
        │
        ▼
[Transition to Live] (liveBroadcasts.transition?broadcastStatus=live)
        │
    (ON AIR)
        │
[Transition to Complete] (liveBroadcasts.transition?broadcastStatus=complete)
```

> [!IMPORTANT]
> Never hardcode ingestion URLs. Zenocast dynamically reads `cdn.ingestionInfo.ingestionAddress` and `cdn.ingestionInfo.streamName` returned by the API call.

---

## 4. Recommended Encoder Settings for YouTube Live

| Resolution | Frame Rate | Video Bitrate (CBR) | Keyframe Interval | Audio Bitrate |
|---|---|---|---|---|
| **1080p** (1920x1080) | 60 fps | 4,500 – 9,000 kbps (6,000 default) | 2 seconds | 128 – 160 kbps |
| **1440p** (2560x1440) | 60 fps | 9,000 – 18,000 kbps (12,000 default) | 2 seconds | 160 – 192 kbps |
| **4K** (3840x2160) | 60 fps | 20,000 – 51,000 kbps (30,000 default) | 2 seconds | 192 – 320 kbps |

- **Rate Control**: CBR (Constant Bitrate) required by YouTube.
- **Audio**: 48,000 Hz stereo, AAC codec.
- **GOP / Keyframe**: Must be between 1 and 4 seconds (Zenocast defaults to 2.0s = 120 frames at 60 fps).

---

## 5. Quota Management & Error Handling

- **Polling Frequency**: Stream health and viewer counts are polled no faster than once every 5 seconds to stay within YouTube's daily API quota (10,000 units/day default).
- **Channel Activation Delay**: First-time live streaming requires 24 hours for YouTube account verification. If unverified, the API returns an error; Zenocast displays a clear message directing the operator to the YouTube Live verification page.
- **Manual Fallback**: Operators may also paste a stream key directly to stream to `rtmp://a.rtmp.youtube.com/live2` or `rtmps://...` without OAuth.
