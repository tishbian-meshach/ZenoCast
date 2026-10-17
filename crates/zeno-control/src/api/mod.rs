//! Control API: HTTP/WebSocket server and frame protocol.

pub mod server;
pub mod websocket;

pub use server::{ControlServer, ControlState};
pub use websocket::{compute_accept_key, encode_text_frame};
