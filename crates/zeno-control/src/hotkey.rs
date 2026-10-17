//! Global and application hotkey management for studio switching and overlay control.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use tracing::info;

use crate::api::server::ControlState;

/// Modifier keys pressed during a shortcut.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub win: bool,
}

impl Modifiers {
    pub const NONE: Self = Self {
        ctrl: false,
        alt: false,
        shift: false,
        win: false,
    };
    pub const CTRL: Self = Self {
        ctrl: true,
        alt: false,
        shift: false,
        win: false,
    };
    pub const ALT: Self = Self {
        ctrl: false,
        alt: true,
        shift: false,
        win: false,
    };
    pub const SHIFT: Self = Self {
        ctrl: false,
        alt: false,
        shift: true,
        win: false,
    };
}

/// A specific hotkey combination (e.g. `Ctrl + 1`, `F5`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HotkeyCombination {
    pub key: String,
    pub modifiers: Modifiers,
}

impl HotkeyCombination {
    /// Constructs a hotkey combination with key name and modifiers.
    pub fn new(key: impl Into<String>, modifiers: Modifiers) -> Self {
        Self {
            key: key.into().to_uppercase(),
            modifiers,
        }
    }

    /// Convenience for a single key without modifiers (e.g. "F5").
    pub fn key_only(key: impl Into<String>) -> Self {
        Self::new(key, Modifiers::NONE)
    }

    /// Convenience for Ctrl + Key.
    pub fn ctrl(key: impl Into<String>) -> Self {
        Self::new(key, Modifiers::CTRL)
    }
}

/// Action performed when a hotkey is triggered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HotkeyAction {
    /// Instant switch to target scene.
    SwitchScene(String),
    /// Triggers an overlay animation (e.g. play in/out/toggle).
    TriggerOverlay { overlay_id: String, action: String },
    /// Starts RTMP stream.
    StartStream,
    /// Stops RTMP stream.
    StopStream,
    /// Starts local recording.
    StartRecord,
    /// Stops local recording.
    StopRecord,
    /// Custom command payload.
    Custom(String),
}

/// Hotkey manager registering and routing shortcut triggers to the control state.
#[derive(Debug, Default)]
pub struct HotkeyManager {
    bindings: HashMap<HotkeyCombination, HotkeyAction>,
}

impl HotkeyManager {
    /// Creates a new hotkey manager with default studio bindings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Binds a hotkey combination to a specific action.
    pub fn bind(&mut self, combo: HotkeyCombination, action: HotkeyAction) {
        self.bindings.insert(combo, action);
    }

    /// Unbinds a hotkey combination.
    pub fn unbind(&mut self, combo: &HotkeyCombination) -> Option<HotkeyAction> {
        self.bindings.remove(combo)
    }

    /// Looks up and dispatches an action for a pressed hotkey combination against the control state.
    pub fn dispatch(
        &self,
        combo: &HotkeyCombination,
        state: &ControlState,
    ) -> Option<HotkeyAction> {
        if let Some(action) = self.bindings.get(combo) {
            info!("Hotkey activated: {:?} -> {:?}", combo, action);
            match action {
                HotkeyAction::SwitchScene(scene) => {
                    *state.current_scene.write() = scene.clone();
                    state.broadcast_ws("scene_change", serde_json::json!({ "scene": scene }));
                }
                HotkeyAction::TriggerOverlay { overlay_id, action } => {
                    state
                        .overlay_triggers
                        .write()
                        .push((overlay_id.clone(), action.clone()));
                    state.broadcast_ws(
                        "overlay_trigger",
                        serde_json::json!({ "overlay_id": overlay_id, "action": action }),
                    );
                }
                HotkeyAction::StartStream => {
                    state.is_streaming.store(true, Ordering::Relaxed);
                    state.broadcast_ws("stream_status", serde_json::json!({ "streaming": true }));
                }
                HotkeyAction::StopStream => {
                    state.is_streaming.store(false, Ordering::Relaxed);
                    state.broadcast_ws("stream_status", serde_json::json!({ "streaming": false }));
                }
                HotkeyAction::StartRecord => {
                    state.is_recording.store(true, Ordering::Relaxed);
                    state.broadcast_ws("record_status", serde_json::json!({ "recording": true }));
                }
                HotkeyAction::StopRecord => {
                    state.is_recording.store(false, Ordering::Relaxed);
                    state.broadcast_ws("record_status", serde_json::json!({ "recording": false }));
                }
                HotkeyAction::Custom(_) => {}
            }
            Some(action.clone())
        } else {
            None
        }
    }
}
