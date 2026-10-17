//! M11 Acceptance Test: Overlay as live source and control API
//!
//! Verifies:
//! 1. An overlay on the program output is updated through both UI and HTTP/curl API calls.
//! 2. HTTP REST control API routes (/status, /scenes, /data/set, /data/get, /scene/switch, /overlay/trigger).
//! 3. Hotkey combinations dispatch correct studio actions.
//! 4. WebSocket upgrade handshake and push events for connected clients.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use zeno_compositor::ArxOverlaySource;
use zeno_control::{ControlServer, ControlState, HotkeyAction, HotkeyCombination, HotkeyManager};
use zeno_core::{LayerId, Property};
use zeno_data::DataStore;
use zeno_engine::eval::ResolvedContent;
use zeno_engine::{Composition, Layer, LayerContent, TextContent};

/// Helper to execute an HTTP request and return (status_code, body).
fn http_request(addr: SocketAddr, method: &str, path: &str, body: Option<&str>) -> (u16, String) {
    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();

    let body_bytes = body.unwrap_or("").as_bytes();
    let req = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        method,
        path,
        addr,
        body_bytes.len(),
        body.unwrap_or("")
    );

    stream.write_all(req.as_bytes()).unwrap();

    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();

    let mut lines = response.lines();
    let status_line = lines.next().unwrap_or("");
    let status_code: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|c| c.parse().ok())
        .unwrap_or(0);

    let body_str = if let Some(idx) = response.find("\r\n\r\n") {
        response[idx + 4..].to_string()
    } else {
        String::new()
    };

    (status_code, body_str)
}

/// Acceptance Check: An overlay on the program output is updated through both the UI and curl calls.
#[test]
fn test_overlay_source_updated_via_curl_and_ui() {
    let store = Arc::new(DataStore::new());
    store.set("scoreboard.team1.name", "Sentinels");
    store.set("scoreboard.team1.score", 0);

    let control_state = ControlState::new(store.clone());
    let server = ControlServer::bind("127.0.0.1:0", control_state).unwrap();
    let addr = server.local_addr();

    // 1. Setup ArxOverlaySource in the compositor sharing the same engine & data store
    let mut comp = Composition::new(1920, 1080, 60.0, 10.0);
    let layer_id = LayerId::new();
    let text_layer = Layer::new(
        layer_id,
        "ScoreTextLayer",
        LayerContent::Text(TextContent {
            text: Property::constant(
                "{scoreboard.team1.name}: {scoreboard.team1.score}".to_string(),
            ),
            font_family: "Inter".to_string(),
            font_size: Property::constant(48.0),
            color: Property::constant(zeno_core::Color::WHITE),
            ..Default::default()
        }),
    );
    comp.layers.push(text_layer);

    let mut overlay_source = ArxOverlaySource::new("MatchScoreOverlay", comp, store.clone());

    // Initial frame evaluation
    let initial_frame = overlay_source.evaluate_frame();
    if let ResolvedContent::Text(ref tc) = initial_frame.layers[0].content {
        assert_eq!(tc.text, "Sentinels: 0");
    } else {
        panic!("Expected ResolvedContent::Text");
    }

    // 2. Simulate cURL call: POST /api/v1/data/set {"path":"scoreboard.team1.score", "value": 7}
    let (status, body) = http_request(
        addr,
        "POST",
        "/api/v1/data/set",
        Some(r#"{"path": "scoreboard.team1.score", "value": 7}"#),
    );
    assert_eq!(status, 200);
    assert!(body.contains(r#""status":"ok""#));

    // Next frame evaluation on the program output reflects the curl update!
    overlay_source.tick(1.0 / 60.0);
    let curl_updated_frame = overlay_source.evaluate_frame();
    if let ResolvedContent::Text(ref tc) = curl_updated_frame.layers[0].content {
        assert_eq!(
            tc.text, "Sentinels: 7",
            "Overlay output must reflect update from curl call"
        );
    } else {
        panic!("Expected ResolvedContent::Text");
    }

    // 3. Simulate UI direct update: store.set("scoreboard.team1.name", "Fnatic") & score = 12
    store.set("scoreboard.team1.name", "Fnatic");
    store.set("scoreboard.team1.score", 12);

    // Next frame evaluation on the program output reflects the UI update!
    overlay_source.tick(1.0 / 60.0);
    let ui_updated_frame = overlay_source.evaluate_frame();
    if let ResolvedContent::Text(ref tc) = ui_updated_frame.layers[0].content {
        assert_eq!(
            tc.text, "Fnatic: 12",
            "Overlay output must reflect update from UI"
        );
    } else {
        panic!("Expected ResolvedContent::Text");
    }
}

/// Tests REST API endpoints: status, scenes, switch, overlay trigger, and streaming controls.
#[test]
fn test_rest_api_endpoints() {
    let store = Arc::new(DataStore::new());
    let control_state = ControlState::new(store);
    let server = ControlServer::bind("127.0.0.1:0", control_state).unwrap();
    let addr = server.local_addr();

    // 1. GET /api/v1/status
    let (status, body) = http_request(addr, "GET", "/api/v1/status", None);
    assert_eq!(status, 200);
    assert!(body.contains(r#""status":"ok""#));
    assert!(body.contains(r#""version":"0.1.0""#));

    // 2. GET /api/v1/scenes
    let (status, body) = http_request(addr, "GET", "/api/v1/scenes", None);
    assert_eq!(status, 200);
    assert!(body.contains("Live Match"));
    assert!(body.contains("Caster Desk"));

    // 3. POST /api/v1/scene/switch
    let (status, body) = http_request(
        addr,
        "POST",
        "/api/v1/scene/switch",
        Some(r#"{"scene_name": "Caster Desk"}"#),
    );
    assert_eq!(status, 200);
    assert!(body.contains(r#""scene":"Caster Desk""#));
    assert_eq!(*server.state().current_scene.read(), "Caster Desk");

    // 4. POST /api/v1/overlay/trigger
    let (status, body) = http_request(
        addr,
        "POST",
        "/api/v1/overlay/trigger",
        Some(r#"{"overlay_id": "scoreboard_overlay", "action": "play_in"}"#),
    );
    assert_eq!(status, 200);
    assert!(body.contains(r#""action":"play_in""#));
    assert_eq!(server.state().overlay_triggers.read().len(), 1);

    // 5. POST /stream/start and /stream/stop
    let (status, _) = http_request(addr, "POST", "/stream/start", None);
    assert_eq!(status, 200);
    assert!(server
        .state()
        .is_streaming
        .load(std::sync::atomic::Ordering::Relaxed));

    let (status, _) = http_request(addr, "POST", "/stream/stop", None);
    assert_eq!(status, 200);
    assert!(!server
        .state()
        .is_streaming
        .load(std::sync::atomic::Ordering::Relaxed));
}

/// Tests hotkey combination mapping and action dispatching.
#[test]
fn test_hotkey_manager_dispatch() {
    let store = Arc::new(DataStore::new());
    let control_state = ControlState::new(store);
    let mut hotkeys = HotkeyManager::new();

    // Map Ctrl+1 to SwitchScene("Live Match") and Ctrl+2 to SwitchScene("Ending")
    let combo1 = HotkeyCombination::ctrl("1");
    let combo2 = HotkeyCombination::ctrl("2");
    hotkeys.bind(
        combo1.clone(),
        HotkeyAction::SwitchScene("Live Match".to_string()),
    );
    hotkeys.bind(
        combo2.clone(),
        HotkeyAction::SwitchScene("Ending".to_string()),
    );

    // Map F5 to ToggleOverlay
    let combo_f5 = HotkeyCombination::key_only("F5");
    hotkeys.bind(
        combo_f5.clone(),
        HotkeyAction::TriggerOverlay {
            overlay_id: "lower_third".to_string(),
            action: "toggle".to_string(),
        },
    );

    // Initial scene
    *control_state.current_scene.write() = "Starting Soon".to_string();

    // Dispatch Ctrl+1
    let action1 = hotkeys.dispatch(&combo1, &control_state);
    assert_eq!(
        action1,
        Some(HotkeyAction::SwitchScene("Live Match".to_string()))
    );
    assert_eq!(*control_state.current_scene.read(), "Live Match");

    // Dispatch Ctrl+2
    let action2 = hotkeys.dispatch(&combo2, &control_state);
    assert_eq!(
        action2,
        Some(HotkeyAction::SwitchScene("Ending".to_string()))
    );
    assert_eq!(*control_state.current_scene.read(), "Ending");

    // Dispatch F5
    let action_f5 = hotkeys.dispatch(&combo_f5, &control_state);
    assert!(matches!(
        action_f5,
        Some(HotkeyAction::TriggerOverlay { .. })
    ));
    assert_eq!(control_state.overlay_triggers.read().len(), 1);
}

/// Tests WebSocket RFC 6455 upgrade handshake and real-time push frame encoding.
#[test]
fn test_websocket_upgrade_handshake_and_push() {
    let store = Arc::new(DataStore::new());
    let control_state = ControlState::new(store);
    let server = ControlServer::bind("127.0.0.1:0", control_state).unwrap();
    let addr = server.local_addr();

    // Connect raw TCP stream and send WebSocket upgrade handshake
    let mut stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();

    let handshake_req = format!(
        "GET /ws HTTP/1.1\r\nHost: {}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Key: dGhlIHNhbXBsZSBub25jZQ==\r\nSec-WebSocket-Version: 13\r\n\r\n",
        addr
    );
    stream.write_all(handshake_req.as_bytes()).unwrap();

    let mut response_buf = [0u8; 1024];
    let n = stream.read(&mut response_buf).unwrap();
    let response_str = String::from_utf8_lossy(&response_buf[..n]);

    assert!(
        response_str.starts_with("HTTP/1.1 101 Switching Protocols"),
        "WebSocket upgrade must respond with 101 Switching Protocols"
    );
    assert!(response_str.contains("Sec-WebSocket-Accept: s3pPLMBiTxaQ9kYGzzhZRbK+xOo="));

    // Send a data update via server, and verify WebSocket client receives the frame
    server.state().broadcast_ws(
        "score_update",
        serde_json::json!({ "team1": "Sentinels", "score": 13 }),
    );

    let mut frame_buf = [0u8; 1024];
    let frame_len = stream.read(&mut frame_buf).unwrap();
    assert!(frame_len > 2, "Must receive WebSocket text frame");
    assert_eq!(frame_buf[0], 0x81, "Opcode 0x1 text frame with FIN=1");
}
