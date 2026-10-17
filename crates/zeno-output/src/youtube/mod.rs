//! YouTube Live integration: OAuth PKCE flow, API client, preflight verification, and broadcast management.

pub mod auth;
pub mod broadcast;
pub mod client;
pub mod preflight;

pub use auth::{LoopbackAuthListener, OAuthToken, PkcePair, TokenStorage};
pub use broadcast::{YouTubeBroadcastManager, YouTubeLiveStatus};
pub use client::{
    BroadcastPrivacy, CreateBroadcastParams, LatencyPreference, MockHttpHandler, YouTubeApiClient,
    YouTubeBroadcast, YouTubeStream,
};
pub use preflight::{PreflightReport, PreflightValidator, RecommendedBitrate, YouTubeBitrateTable};
