//! # zeno-control
//!
//! Control interfaces: HTTP and WebSocket remote-control API (`127.0.0.1:4466`),
//! Stream Deck / Bitfocus Companion integrations, and global hotkeys.

pub mod api;
pub mod hotkey;

pub use api::{compute_accept_key, encode_text_frame, ControlServer, ControlState};
pub use hotkey::{HotkeyAction, HotkeyCombination, HotkeyManager, Modifiers};
