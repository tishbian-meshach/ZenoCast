//! High-performance, zero-dependency HTTP and WebSocket remote control server (`127.0.0.1:4466`).
//!
//! Provides REST endpoints for live scoring, scene switching, overlay triggers,
//! streaming/recording state toggles, and live WebSocket push events for Stream Deck / Bitfocus Companion.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use parking_lot::RwLock;
use serde_json::json;
use tracing::{debug, info, warn};
use zeno_data::{DataStore, DataValue};

use crate::api::websocket::{compute_accept_key, encode_text_frame};

/// Shared state exposed through the control server.
#[derive(Clone)]
pub struct ControlState {
    pub store: Arc<DataStore>,
    pub current_scene: Arc<RwLock<String>>,
    pub available_scenes: Arc<RwLock<Vec<String>>>,
    pub is_streaming: Arc<AtomicBool>,
    pub is_recording: Arc<AtomicBool>,
    pub auth_token: Option<String>,
    pub overlay_triggers: Arc<RwLock<Vec<(String, String)>>>,
    pub ws_clients: Arc<RwLock<Vec<TcpStream>>>,
}

impl ControlState {
    /// Creates a new control state backed by the given data store.
    pub fn new(store: Arc<DataStore>) -> Self {
        Self {
            store,
            current_scene: Arc::new(RwLock::new("Live Match".to_string())),
            available_scenes: Arc::new(RwLock::new(vec![
                "Starting Soon".to_string(),
                "Live Match".to_string(),
                "Caster Desk".to_string(),
                "Break".to_string(),
                "Ending".to_string(),
            ])),
            is_streaming: Arc::new(AtomicBool::new(false)),
            is_recording: Arc::new(AtomicBool::new(false)),
            auth_token: None,
            overlay_triggers: Arc::new(RwLock::new(Vec::new())),
            ws_clients: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Configures optional bearer token authentication.
    pub fn with_auth_token(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    /// Broadcasts a JSON event to all connected WebSocket clients.
    pub fn broadcast_ws(&self, event_name: &str, data: serde_json::Value) {
        let msg = json!({
            "event": event_name,
            "data": data,
        })
        .to_string();

        let frame = encode_text_frame(&msg);
        let mut clients = self.ws_clients.write();
        clients.retain_mut(|stream| stream.write_all(&frame).is_ok());
    }
}

/// Standalone HTTP and WebSocket control server.
pub struct ControlServer {
    local_addr: SocketAddr,
    shutdown: Arc<AtomicBool>,
    state: ControlState,
    thread_handle: Option<JoinHandle<()>>,
}

impl ControlServer {
    /// Binds and starts the control server on the requested address (e.g. `127.0.0.1:4466` or `127.0.0.1:0`).
    pub fn bind(addr: &str, state: ControlState) -> Result<Self, std::io::Error> {
        let listener = TcpListener::bind(addr)?;
        listener.set_nonblocking(true)?;
        let local_addr = listener.local_addr()?;
        let shutdown = Arc::new(AtomicBool::new(false));

        let shutdown_clone = shutdown.clone();
        let state_clone = state.clone();

        let thread_handle = thread::spawn(move || {
            info!("Control server listening on {}", local_addr);
            while !shutdown_clone.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, client_addr)) => {
                        debug!("Control connection from {}", client_addr);
                        let s_state = state_clone.clone();
                        thread::spawn(move || {
                            handle_connection(stream, s_state);
                        });
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => {
                        warn!("Control accept error: {}", e);
                        thread::sleep(Duration::from_millis(50));
                    }
                }
            }
            info!("Control server stopped");
        });

        Ok(Self {
            local_addr,
            shutdown,
            state,
            thread_handle: Some(thread_handle),
        })
    }

    /// Returns the local bound socket address.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Returns a reference to the shared control state.
    pub fn state(&self) -> &ControlState {
        &self.state
    }

    /// Stops the server and joins the listener thread.
    pub fn stop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for ControlServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Handles a single incoming HTTP / WebSocket connection.
fn handle_connection(mut stream: TcpStream, state: ControlState) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    let mut buffer = [0u8; 8192];

    let bytes_read = match stream.read(&mut buffer) {
        Ok(n) if n > 0 => n,
        _ => return,
    };

    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);
    let mut lines = request_str.lines();
    let request_line = match lines.next() {
        Some(l) => l,
        None => return,
    };

    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("");
    let uri = parts.next().unwrap_or("");

    // Collect headers
    let mut ws_key = None;
    let mut auth_header = None;

    for line in lines.by_ref() {
        if line.is_empty() || line == "\r" {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            let k_trim = k.trim().to_lowercase();
            let v_trim = v.trim();
            if k_trim == "sec-websocket-key" {
                ws_key = Some(v_trim.to_string());
            } else if k_trim == "authorization" {
                auth_header = Some(v_trim.to_string());
            }
        }
    }

    // Check optional bearer token authentication
    if let Some(expected_token) = &state.auth_token {
        let mut authenticated = false;
        if let Some(auth_val) = auth_header {
            if auth_val.strip_prefix("Bearer ").map(|t| t.trim()) == Some(expected_token) {
                authenticated = true;
            }
        }
        if !authenticated && uri.contains(&format!("token={}", expected_token)) {
            authenticated = true;
        }

        if !authenticated {
            let resp = "HTTP/1.1 401 Unauthorized\r\nContent-Type: application/json\r\nContent-Length: 35\r\n\r\n{\"error\":\"Unauthorized: invalid token\"}";
            let _ = stream.write_all(resp.as_bytes());
            return;
        }
    }

    // Handle WebSocket upgrade
    if let Some(key) = ws_key {
        let accept = compute_accept_key(&key);
        let handshake_response = format!(
            "HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: {}\r\n\r\n",
            accept
        );
        if stream.write_all(handshake_response.as_bytes()).is_ok() {
            let _ = stream.set_read_timeout(None);
            let _ = stream.set_nonblocking(true);
            state.ws_clients.write().push(stream);
        }
        return;
    }

    // Extract body
    let body_str = if let Some(body_start) = request_str.find("\r\n\r\n") {
        &request_str[body_start + 4..]
    } else {
        ""
    };

    // Clean URI path (strip query string)
    let path = uri.split('?').next().unwrap_or(uri);

    // Route matching
    match (method, path) {
        ("GET", "/status") | ("GET", "/api/v1/status") => {
            let status_json = json!({
                "status": "ok",
                "version": "0.1.0",
                "current_scene": *state.current_scene.read(),
                "is_streaming": state.is_streaming.load(Ordering::Relaxed),
                "is_recording": state.is_recording.load(Ordering::Relaxed),
                "store_keys": state.store.len(),
            });
            send_json_response(&mut stream, 200, &status_json);
        }

        ("GET", "/scenes") | ("GET", "/api/v1/scenes") => {
            let scenes_json = json!({
                "scenes": *state.available_scenes.read(),
                "active_scene": *state.current_scene.read(),
            });
            send_json_response(&mut stream, 200, &scenes_json);
        }

        ("POST", "/scene/switch") | ("POST", "/api/v1/scene/switch") => {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body_str) {
                if let Some(scene_name) = parsed.get("scene_name").and_then(|s| s.as_str()) {
                    *state.current_scene.write() = scene_name.to_string();
                    state.broadcast_ws("scene_change", json!({ "scene": scene_name }));
                    send_json_response(
                        &mut stream,
                        200,
                        &json!({ "status": "ok", "scene": scene_name }),
                    );
                    return;
                }
            }
            send_json_response(
                &mut stream,
                400,
                &json!({ "error": "Missing 'scene_name' parameter" }),
            );
        }

        ("POST", "/data/set") | ("POST", "/api/v1/data/set") => {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body_str) {
                if let (Some(data_path), Some(raw_val)) = (
                    parsed.get("path").and_then(|p| p.as_str()),
                    parsed.get("value"),
                ) {
                    let dval = DataValue::from_json(raw_val);
                    state.store.set(data_path, dval.clone());
                    state.broadcast_ws(
                        "data_change",
                        json!({ "path": data_path, "value": raw_val }),
                    );
                    send_json_response(
                        &mut stream,
                        200,
                        &json!({ "status": "ok", "path": data_path, "value": raw_val }),
                    );
                    return;
                }
            }
            send_json_response(
                &mut stream,
                400,
                &json!({ "error": "Invalid body. Expected {'path', 'value'}" }),
            );
        }

        ("GET", "/data/get") | ("GET", "/api/v1/data/get") => {
            let mut requested_path = None;
            if let Some(query) = uri.split('?').nth(1) {
                for pair in query.split('&') {
                    if let Some((k, v)) = pair.split_once('=') {
                        if k == "path" {
                            requested_path = Some(v);
                        }
                    }
                }
            }

            if let Some(p) = requested_path {
                if let Some(val) = state.store.get(p) {
                    send_json_response(
                        &mut stream,
                        200,
                        &json!({ "status": "ok", "path": p, "value": val.to_json() }),
                    );
                } else {
                    send_json_response(
                        &mut stream,
                        404,
                        &json!({ "error": "Key not found", "path": p }),
                    );
                }
            } else {
                send_json_response(
                    &mut stream,
                    400,
                    &json!({ "error": "Missing 'path' query parameter" }),
                );
            }
        }

        ("POST", "/overlay/trigger") | ("POST", "/api/v1/overlay/trigger") => {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(body_str) {
                if let (Some(id), Some(action)) = (
                    parsed.get("overlay_id").and_then(|s| s.as_str()),
                    parsed.get("action").and_then(|s| s.as_str()),
                ) {
                    state
                        .overlay_triggers
                        .write()
                        .push((id.to_string(), action.to_string()));
                    state.broadcast_ws(
                        "overlay_trigger",
                        json!({ "overlay_id": id, "action": action }),
                    );
                    send_json_response(
                        &mut stream,
                        200,
                        &json!({ "status": "ok", "overlay_id": id, "action": action }),
                    );
                    return;
                }
            }
            send_json_response(
                &mut stream,
                400,
                &json!({ "error": "Invalid body. Expected {'overlay_id', 'action'}" }),
            );
        }

        ("POST", "/stream/start") => {
            state.is_streaming.store(true, Ordering::Relaxed);
            state.broadcast_ws("stream_status", json!({ "streaming": true }));
            send_json_response(
                &mut stream,
                200,
                &json!({ "status": "ok", "streaming": true }),
            );
        }
        ("POST", "/stream/stop") => {
            state.is_streaming.store(false, Ordering::Relaxed);
            state.broadcast_ws("stream_status", json!({ "streaming": false }));
            send_json_response(
                &mut stream,
                200,
                &json!({ "status": "ok", "streaming": false }),
            );
        }

        ("POST", "/record/start") => {
            state.is_recording.store(true, Ordering::Relaxed);
            state.broadcast_ws("record_status", json!({ "recording": true }));
            send_json_response(
                &mut stream,
                200,
                &json!({ "status": "ok", "recording": true }),
            );
        }
        ("POST", "/record/stop") => {
            state.is_recording.store(false, Ordering::Relaxed);
            state.broadcast_ws("record_status", json!({ "recording": false }));
            send_json_response(
                &mut stream,
                200,
                &json!({ "status": "ok", "recording": false }),
            );
        }

        _ => {
            send_json_response(
                &mut stream,
                404,
                &json!({ "error": "Not Found", "uri": uri }),
            );
        }
    }
}

/// Helper function writing a standard HTTP JSON response.
fn send_json_response(stream: &mut TcpStream, status_code: u16, body: &serde_json::Value) {
    let body_bytes = body.to_string().into_bytes();
    let status_line = match status_code {
        200 => "HTTP/1.1 200 OK",
        400 => "HTTP/1.1 400 Bad Request",
        401 => "HTTP/1.1 401 Unauthorized",
        404 => "HTTP/1.1 404 Not Found",
        _ => "HTTP/1.1 500 Internal Server Error",
    };

    let response = format!(
        "{}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status_line,
        body_bytes.len()
    );

    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(&body_bytes);
}
