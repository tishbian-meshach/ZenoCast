//! Section 4 Integration Test Tier: UI Wiring Regression Guard.
//!
//! Validates core application flows against real engine state:
//! 1. Add each source type and assert real Source exists in registry and produces non-empty frames.
//! 2. Switch scenes and assert program output changed.
//! 3. Start/stop local recording and assert a valid file exists on disk afterward.
//! 4. Start/stop streaming against a local test RTMP server and assert server received data.

pub mod app_render_flow;
pub mod recording_flow;
pub mod scene_switching;
pub mod source_flows;
pub mod streaming_flow;
