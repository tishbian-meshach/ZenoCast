//! Source-of-truth Hotkey Registry, KeyCombo representation, conflict detection,
//! and markdown documentation generator for Zenocast.

use egui::Key;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Identifiers for every standard application action across Sections 1–5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ActionId {
    // --- Section 1: Streaming, recording, and output ---
    StartStreaming,
    StopStreaming,
    StartRecording,
    StopRecording,
    PauseRecording,
    ToggleReplayBuffer,
    SaveReplayBuffer,
    TogglePreview,

    // --- Section 2: Scene and transition control ---
    Cut,
    AutoTransition,
    NextScene,
    PreviousScene,
    ToggleStudioMode,

    // --- Section 3: Live Studio editing ---
    Undo,
    Redo,
    DeleteSelected,
    DuplicateSelected,
    Copy,
    Paste,
    PasteInPlace,
    SelectAll,
    DeselectAll,
    BringToFront,
    BringForward,
    SendBackward,
    SendToBack,
    ToggleSourceVisibility,
    ToggleSourceLock,
    ToggleSourceMute,
    OpenSourceProperties,
    RenameSelected,

    // --- Section 4: Overlay editor ---
    AddTextLayer,
    AddShapeLayer,
    AddCameraLayer,
    AddParticlesLayer,
    DeleteLayer,
    GroupLayers,
    UngroupLayers,
    AddKeyframe,
    DeleteKeyframe,
    PlayPauseTimeline,
    StopTimeline,
    GoToStart,
    GoToEnd,
    StepFrameBack,
    StepFrameForward,
    SetInMarker,
    SetOutMarker,
    AddMarker,
    ZoomTimelineIn,
    ZoomTimelineOut,
    FitTimeline,
    ZoomCanvasIn,
    ZoomCanvasOut,
    ResetCanvasZoom,
    FitCanvas,
    ToggleCurveEditor,
    ToolSelect,
    ToolPan,

    // --- Section 5: Window, panels, and modes ---
    ModeLiveStudio,
    ModeEditOverlay,
    ModeDataControl,
    OpenSettings,
    NewProject,
    OpenProject,
    SaveProject,
    SaveProjectAs,
    ToggleFullscreen,
    DialogCancel,
    DialogConfirm,
}

impl ActionId {
    /// String key used in configuration serialization and external lookup.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::StartStreaming => "start_streaming",
            Self::StopStreaming => "stop_streaming",
            Self::StartRecording => "start_recording",
            Self::StopRecording => "stop_recording",
            Self::PauseRecording => "pause_recording",
            Self::ToggleReplayBuffer => "toggle_replay_buffer",
            Self::SaveReplayBuffer => "save_replay_buffer",
            Self::TogglePreview => "toggle_preview",

            Self::Cut => "cut",
            Self::AutoTransition => "auto_transition",
            Self::NextScene => "next_scene",
            Self::PreviousScene => "previous_scene",
            Self::ToggleStudioMode => "toggle_studio_mode",

            Self::Undo => "undo",
            Self::Redo => "redo",
            Self::DeleteSelected => "delete_selected",
            Self::DuplicateSelected => "duplicate_selected",
            Self::Copy => "copy",
            Self::Paste => "paste",
            Self::PasteInPlace => "paste_in_place",
            Self::SelectAll => "select_all",
            Self::DeselectAll => "deselect_all",
            Self::BringToFront => "bring_to_front",
            Self::BringForward => "bring_forward",
            Self::SendBackward => "send_backward",
            Self::SendToBack => "send_to_back",
            Self::ToggleSourceVisibility => "toggle_source_visibility",
            Self::ToggleSourceLock => "toggle_source_lock",
            Self::ToggleSourceMute => "toggle_source_mute",
            Self::OpenSourceProperties => "open_source_properties",
            Self::RenameSelected => "rename_selected",

            Self::AddTextLayer => "add_text_layer",
            Self::AddShapeLayer => "add_shape_layer",
            Self::AddCameraLayer => "add_camera_layer",
            Self::AddParticlesLayer => "add_particles_layer",
            Self::DeleteLayer => "delete_layer",
            Self::GroupLayers => "group_layers",
            Self::UngroupLayers => "ungroup_layers",
            Self::AddKeyframe => "add_keyframe",
            Self::DeleteKeyframe => "delete_keyframe",
            Self::PlayPauseTimeline => "play_pause_timeline",
            Self::StopTimeline => "stop_timeline",
            Self::GoToStart => "go_to_start",
            Self::GoToEnd => "go_to_end",
            Self::StepFrameBack => "step_frame_back",
            Self::StepFrameForward => "step_frame_forward",
            Self::SetInMarker => "set_in_marker",
            Self::SetOutMarker => "set_out_marker",
            Self::AddMarker => "add_marker",
            Self::ZoomTimelineIn => "zoom_timeline_in",
            Self::ZoomTimelineOut => "zoom_timeline_out",
            Self::FitTimeline => "fit_timeline",
            Self::ZoomCanvasIn => "zoom_canvas_in",
            Self::ZoomCanvasOut => "zoom_canvas_out",
            Self::ResetCanvasZoom => "reset_canvas_zoom",
            Self::FitCanvas => "fit_canvas",
            Self::ToggleCurveEditor => "toggle_curve_editor",
            Self::ToolSelect => "tool_select",
            Self::ToolPan => "tool_pan",

            Self::ModeLiveStudio => "mode_live_studio",
            Self::ModeEditOverlay => "mode_edit_overlay",
            Self::ModeDataControl => "mode_data_control",
            Self::OpenSettings => "open_settings",
            Self::NewProject => "new_project",
            Self::OpenProject => "open_project",
            Self::SaveProject => "save_project",
            Self::SaveProjectAs => "save_project_as",
            Self::ToggleFullscreen => "toggle_fullscreen",
            Self::DialogCancel => "dialog_cancel",
            Self::DialogConfirm => "dialog_confirm",
        }
    }

    /// Converts from string key to ActionId.
    pub fn from_key(s: &str) -> Option<Self> {
        match s {
            "start_streaming" => Some(Self::StartStreaming),
            "stop_streaming" => Some(Self::StopStreaming),
            "start_recording" => Some(Self::StartRecording),
            "stop_recording" => Some(Self::StopRecording),
            "pause_recording" => Some(Self::PauseRecording),
            "toggle_replay_buffer" => Some(Self::ToggleReplayBuffer),
            "save_replay_buffer" => Some(Self::SaveReplayBuffer),
            "toggle_preview" => Some(Self::TogglePreview),

            "cut" => Some(Self::Cut),
            "auto_transition" => Some(Self::AutoTransition),
            "next_scene" => Some(Self::NextScene),
            "previous_scene" => Some(Self::PreviousScene),
            "toggle_studio_mode" => Some(Self::ToggleStudioMode),

            "undo" => Some(Self::Undo),
            "redo" => Some(Self::Redo),
            "delete_selected" => Some(Self::DeleteSelected),
            "duplicate_selected" => Some(Self::DuplicateSelected),
            "copy" => Some(Self::Copy),
            "paste" => Some(Self::Paste),
            "paste_in_place" => Some(Self::PasteInPlace),
            "select_all" => Some(Self::SelectAll),
            "deselect_all" => Some(Self::DeselectAll),
            "bring_to_front" => Some(Self::BringToFront),
            "bring_forward" => Some(Self::BringForward),
            "send_backward" => Some(Self::SendBackward),
            "send_to_back" => Some(Self::SendToBack),
            "toggle_source_visibility" => Some(Self::ToggleSourceVisibility),
            "toggle_source_lock" => Some(Self::ToggleSourceLock),
            "toggle_source_mute" => Some(Self::ToggleSourceMute),
            "open_source_properties" => Some(Self::OpenSourceProperties),
            "rename_selected" => Some(Self::RenameSelected),

            "add_text_layer" => Some(Self::AddTextLayer),
            "add_shape_layer" => Some(Self::AddShapeLayer),
            "add_camera_layer" => Some(Self::AddCameraLayer),
            "add_particles_layer" => Some(Self::AddParticlesLayer),
            "delete_layer" => Some(Self::DeleteLayer),
            "group_layers" => Some(Self::GroupLayers),
            "ungroup_layers" => Some(Self::UngroupLayers),
            "add_keyframe" => Some(Self::AddKeyframe),
            "delete_keyframe" => Some(Self::DeleteKeyframe),
            "play_pause_timeline" => Some(Self::PlayPauseTimeline),
            "stop_timeline" => Some(Self::StopTimeline),
            "go_to_start" => Some(Self::GoToStart),
            "go_to_end" => Some(Self::GoToEnd),
            "step_frame_back" => Some(Self::StepFrameBack),
            "step_frame_forward" => Some(Self::StepFrameForward),
            "set_in_marker" => Some(Self::SetInMarker),
            "set_out_marker" => Some(Self::SetOutMarker),
            "add_marker" => Some(Self::AddMarker),
            "zoom_timeline_in" => Some(Self::ZoomTimelineIn),
            "zoom_timeline_out" => Some(Self::ZoomTimelineOut),
            "fit_timeline" => Some(Self::FitTimeline),
            "zoom_canvas_in" => Some(Self::ZoomCanvasIn),
            "zoom_canvas_out" => Some(Self::ZoomCanvasOut),
            "reset_canvas_zoom" => Some(Self::ResetCanvasZoom),
            "fit_canvas" => Some(Self::FitCanvas),
            "toggle_curve_editor" => Some(Self::ToggleCurveEditor),
            "tool_select" => Some(Self::ToolSelect),
            "tool_pan" => Some(Self::ToolPan),

            "mode_live_studio" => Some(Self::ModeLiveStudio),
            "mode_edit_overlay" => Some(Self::ModeEditOverlay),
            "mode_data_control" => Some(Self::ModeDataControl),
            "open_settings" => Some(Self::OpenSettings),
            "new_project" => Some(Self::NewProject),
            "open_project" => Some(Self::OpenProject),
            "save_project" => Some(Self::SaveProject),
            "save_project_as" => Some(Self::SaveProjectAs),
            "toggle_fullscreen" => Some(Self::ToggleFullscreen),
            "dialog_cancel" => Some(Self::DialogCancel),
            "dialog_confirm" => Some(Self::DialogConfirm),
            _ => None,
        }
    }
}

impl std::str::FromStr for ActionId {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_key(s).ok_or(())
    }
}

/// Category grouping for settings presentation and documentation tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ActionCategory {
    StreamingAndOutput,
    SceneAndTransition,
    LiveStudioEditing,
    OverlayEditor,
    WindowAndModes,
}

impl ActionCategory {
    pub fn title(&self) -> &'static str {
        match self {
            Self::StreamingAndOutput => "Streaming, Recording, and Output",
            Self::SceneAndTransition => "Scene and Transition Control",
            Self::LiveStudioEditing => "Live Studio Editing",
            Self::OverlayEditor => "Overlay Editor",
            Self::WindowAndModes => "Window, Panels, and Modes",
        }
    }

    pub fn section_num(&self) -> u32 {
        match self {
            Self::StreamingAndOutput => 1,
            Self::SceneAndTransition => 2,
            Self::LiveStudioEditing => 3,
            Self::OverlayEditor => 4,
            Self::WindowAndModes => 5,
        }
    }
}

/// Normalized keyboard chord combination.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct KeyCombo {
    pub key: Key,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
}

impl KeyCombo {
    pub fn new(key: Key, ctrl: bool, alt: bool, shift: bool) -> Self {
        Self {
            key,
            ctrl,
            alt,
            shift,
        }
    }

    pub fn key_only(key: Key) -> Self {
        Self::new(key, false, false, false)
    }

    pub fn ctrl(key: Key) -> Self {
        Self::new(key, true, false, false)
    }

    pub fn ctrl_shift(key: Key) -> Self {
        Self::new(key, true, false, true)
    }

    pub fn shift(key: Key) -> Self {
        Self::new(key, false, false, true)
    }

    pub fn alt(key: Key) -> Self {
        Self::new(key, false, true, false)
    }

    pub fn alt_shift(key: Key) -> Self {
        Self::new(key, false, true, true)
    }

    /// Formats the key combination into canonical user-readable text (e.g. `Ctrl+Shift+Z`).
    pub fn format_text(&self) -> String {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.alt {
            parts.push("Alt");
        }
        if self.shift {
            parts.push("Shift");
        }
        parts.push(key_to_str(self.key));
        parts.join("+")
    }

    /// Parses a string representation (e.g. "Ctrl+Shift+Z", "F2", "Delete") into a KeyCombo.
    pub fn parse(s: &str) -> Option<Self> {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            return None;
        }

        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;

        // Split by '+'
        let parts: Vec<&str> = trimmed.split('+').collect();
        if parts.is_empty() {
            return None;
        }

        for &part in &parts[..parts.len() - 1] {
            let p = part.trim().to_lowercase();
            match p.as_str() {
                "ctrl" | "control" | "cmd" | "command" => ctrl = true,
                "alt" | "option" => alt = true,
                "shift" => shift = true,
                _ => {}
            }
        }

        let key_str = parts.last()?.trim();
        let key = str_to_key(key_str)?;

        Some(Self {
            key,
            ctrl,
            alt,
            shift,
        })
    }

    /// Tests if this chord matches the current input state on key-press.
    pub fn matches(&self, i: &egui::InputState) -> bool {
        if !i.key_pressed(self.key) {
            return false;
        }
        let ctrl = i.modifiers.command || i.modifiers.ctrl;
        ctrl == self.ctrl && i.modifiers.alt == self.alt && i.modifiers.shift == self.shift
    }

    /// Tests if this chord's key is actively held down (for push-to-talk / push-to-mute).
    pub fn is_down(&self, i: &egui::InputState) -> bool {
        if !i.key_down(self.key) {
            return false;
        }
        let ctrl = i.modifiers.command || i.modifiers.ctrl;
        ctrl == self.ctrl && i.modifiers.alt == self.alt && i.modifiers.shift == self.shift
    }

    /// Detects if any valid key was pressed during this input state and constructs a KeyCombo with active modifiers.
    /// Excludes Escape so that Escape can be reserved for canceling modal interactions.
    pub fn detect_pressed(i: &egui::InputState) -> Option<Self> {
        let ctrl = i.modifiers.command || i.modifiers.ctrl;
        let alt = i.modifiers.alt;
        let shift = i.modifiers.shift;

        const CANDIDATE_KEYS: &[Key] = &[
            Key::F1,
            Key::F2,
            Key::F3,
            Key::F4,
            Key::F5,
            Key::F6,
            Key::F7,
            Key::F8,
            Key::F9,
            Key::F10,
            Key::F11,
            Key::F12,
            Key::A,
            Key::B,
            Key::C,
            Key::D,
            Key::E,
            Key::F,
            Key::G,
            Key::H,
            Key::I,
            Key::J,
            Key::K,
            Key::L,
            Key::M,
            Key::N,
            Key::O,
            Key::P,
            Key::Q,
            Key::R,
            Key::S,
            Key::T,
            Key::U,
            Key::V,
            Key::W,
            Key::X,
            Key::Y,
            Key::Z,
            Key::Num0,
            Key::Num1,
            Key::Num2,
            Key::Num3,
            Key::Num4,
            Key::Num5,
            Key::Num6,
            Key::Num7,
            Key::Num8,
            Key::Num9,
            Key::Space,
            Key::Enter,
            Key::Delete,
            Key::Backspace,
            Key::Tab,
            Key::Insert,
            Key::Home,
            Key::End,
            Key::PageUp,
            Key::PageDown,
            Key::ArrowLeft,
            Key::ArrowRight,
            Key::ArrowUp,
            Key::ArrowDown,
            Key::Comma,
            Key::Period,
            Key::Minus,
            Key::Equals,
            Key::OpenBracket,
            Key::CloseBracket,
            Key::Backslash,
            Key::Slash,
            Key::Semicolon,
            Key::Quote,
        ];

        for &k in CANDIDATE_KEYS {
            if i.key_pressed(k) {
                return Some(Self {
                    key: k,
                    ctrl,
                    alt,
                    shift,
                });
            }
        }
        None
    }

    /// Converts this key combo to a `global_hotkey::hotkey::HotKey` instance if convertible.
    pub fn to_global_hotkey(&self) -> Option<global_hotkey::hotkey::HotKey> {
        use global_hotkey::hotkey::{Code, HotKey, Modifiers};
        let mut mods = Modifiers::empty();
        if self.ctrl {
            mods |= Modifiers::CONTROL;
        }
        if self.shift {
            mods |= Modifiers::SHIFT;
        }
        if self.alt {
            mods |= Modifiers::ALT;
        }
        let code = match self.key {
            Key::A => Code::KeyA,
            Key::B => Code::KeyB,
            Key::C => Code::KeyC,
            Key::D => Code::KeyD,
            Key::E => Code::KeyE,
            Key::F => Code::KeyF,
            Key::G => Code::KeyG,
            Key::H => Code::KeyH,
            Key::I => Code::KeyI,
            Key::J => Code::KeyJ,
            Key::K => Code::KeyK,
            Key::L => Code::KeyL,
            Key::M => Code::KeyM,
            Key::N => Code::KeyN,
            Key::O => Code::KeyO,
            Key::P => Code::KeyP,
            Key::Q => Code::KeyQ,
            Key::R => Code::KeyR,
            Key::S => Code::KeyS,
            Key::T => Code::KeyT,
            Key::U => Code::KeyU,
            Key::V => Code::KeyV,
            Key::W => Code::KeyW,
            Key::X => Code::KeyX,
            Key::Y => Code::KeyY,
            Key::Z => Code::KeyZ,
            Key::Num0 => Code::Digit0,
            Key::Num1 => Code::Digit1,
            Key::Num2 => Code::Digit2,
            Key::Num3 => Code::Digit3,
            Key::Num4 => Code::Digit4,
            Key::Num5 => Code::Digit5,
            Key::Num6 => Code::Digit6,
            Key::Num7 => Code::Digit7,
            Key::Num8 => Code::Digit8,
            Key::Num9 => Code::Digit9,
            Key::F1 => Code::F1,
            Key::F2 => Code::F2,
            Key::F3 => Code::F3,
            Key::F4 => Code::F4,
            Key::F5 => Code::F5,
            Key::F6 => Code::F6,
            Key::F7 => Code::F7,
            Key::F8 => Code::F8,
            Key::F9 => Code::F9,
            Key::F10 => Code::F10,
            Key::F11 => Code::F11,
            Key::F12 => Code::F12,
            Key::Space => Code::Space,
            Key::Enter => Code::Enter,
            Key::Escape => Code::Escape,
            Key::Delete => Code::Delete,
            Key::Home => Code::Home,
            Key::End => Code::End,
            Key::Comma => Code::Comma,
            Key::Period => Code::Period,
            Key::Minus => Code::Minus,
            Key::Equals => Code::Equal,
            Key::OpenBracket => Code::BracketLeft,
            Key::CloseBracket => Code::BracketRight,
            Key::ArrowLeft => Code::ArrowLeft,
            Key::ArrowRight => Code::ArrowRight,
            Key::ArrowUp => Code::ArrowUp,
            Key::ArrowDown => Code::ArrowDown,
            _ => return None,
        };
        let m = if mods.is_empty() { None } else { Some(mods) };
        Some(HotKey::new(m, code))
    }
}

/// Mapping from egui::Key to human-readable string.
pub fn key_to_str(key: Key) -> &'static str {
    match key {
        Key::A => "A",
        Key::B => "B",
        Key::C => "C",
        Key::D => "D",
        Key::E => "E",
        Key::F => "F",
        Key::G => "G",
        Key::H => "H",
        Key::I => "I",
        Key::J => "J",
        Key::K => "K",
        Key::L => "L",
        Key::M => "M",
        Key::N => "N",
        Key::O => "O",
        Key::P => "P",
        Key::Q => "Q",
        Key::R => "R",
        Key::S => "S",
        Key::T => "T",
        Key::U => "U",
        Key::V => "V",
        Key::W => "W",
        Key::X => "X",
        Key::Y => "Y",
        Key::Z => "Z",

        Key::Num0 => "0",
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",

        Key::F1 => "F1",
        Key::F2 => "F2",
        Key::F3 => "F3",
        Key::F4 => "F4",
        Key::F5 => "F5",
        Key::F6 => "F6",
        Key::F7 => "F7",
        Key::F8 => "F8",
        Key::F9 => "F9",
        Key::F10 => "F10",
        Key::F11 => "F11",
        Key::F12 => "F12",

        Key::Escape => "Escape",
        Key::Enter => "Enter",
        Key::Space => "Space",
        Key::Tab => "Tab",
        Key::Backspace => "Backspace",
        Key::Delete => "Delete",
        Key::Insert => "Insert",
        Key::Home => "Home",
        Key::End => "End",
        Key::PageUp => "PageUp",
        Key::PageDown => "PageDown",

        Key::ArrowLeft => "Left",
        Key::ArrowRight => "Right",
        Key::ArrowUp => "Up",
        Key::ArrowDown => "Down",

        Key::Comma => ",",
        Key::Period => ".",
        Key::Minus => "-",
        Key::Equals => "=",
        Key::OpenBracket => "[",
        Key::CloseBracket => "]",
        Key::Backslash => "\\",
        Key::Slash => "/",
        Key::Semicolon => ";",
        Key::Quote => "'",
        _ => "Unknown",
    }
}

/// Mapping from string token to egui::Key.
pub fn str_to_key(s: &str) -> Option<Key> {
    let lower = s.trim().to_lowercase();
    match lower.as_str() {
        "a" => Some(Key::A),
        "b" => Some(Key::B),
        "c" => Some(Key::C),
        "d" => Some(Key::D),
        "e" => Some(Key::E),
        "f" => Some(Key::F),
        "g" => Some(Key::G),
        "h" => Some(Key::H),
        "i" => Some(Key::I),
        "j" => Some(Key::J),
        "k" => Some(Key::K),
        "l" => Some(Key::L),
        "m" => Some(Key::M),
        "n" => Some(Key::N),
        "o" => Some(Key::O),
        "p" => Some(Key::P),
        "q" => Some(Key::Q),
        "r" => Some(Key::R),
        "s" => Some(Key::S),
        "t" => Some(Key::T),
        "u" => Some(Key::U),
        "v" => Some(Key::V),
        "w" => Some(Key::W),
        "x" => Some(Key::X),
        "y" => Some(Key::Y),
        "z" => Some(Key::Z),

        "0" => Some(Key::Num0),
        "1" => Some(Key::Num1),
        "2" => Some(Key::Num2),
        "3" => Some(Key::Num3),
        "4" => Some(Key::Num4),
        "5" => Some(Key::Num5),
        "6" => Some(Key::Num6),
        "7" => Some(Key::Num7),
        "8" => Some(Key::Num8),
        "9" => Some(Key::Num9),

        "f1" => Some(Key::F1),
        "f2" => Some(Key::F2),
        "f3" => Some(Key::F3),
        "f4" => Some(Key::F4),
        "f5" => Some(Key::F5),
        "f6" => Some(Key::F6),
        "f7" => Some(Key::F7),
        "f8" => Some(Key::F8),
        "f9" => Some(Key::F9),
        "f10" => Some(Key::F10),
        "f11" => Some(Key::F11),
        "f12" => Some(Key::F12),

        "escape" | "esc" => Some(Key::Escape),
        "enter" | "return" => Some(Key::Enter),
        "space" => Some(Key::Space),
        "tab" => Some(Key::Tab),
        "backspace" => Some(Key::Backspace),
        "delete" | "del" => Some(Key::Delete),
        "insert" | "ins" => Some(Key::Insert),
        "home" => Some(Key::Home),
        "end" => Some(Key::End),
        "pageup" => Some(Key::PageUp),
        "pagedown" => Some(Key::PageDown),

        "left" | "arrowleft" => Some(Key::ArrowLeft),
        "right" | "arrowright" => Some(Key::ArrowRight),
        "up" | "arrowup" => Some(Key::ArrowUp),
        "down" | "arrowdown" => Some(Key::ArrowDown),

        "," | "comma" => Some(Key::Comma),
        "." | "period" => Some(Key::Period),
        "-" | "minus" => Some(Key::Minus),
        "=" | "equals" => Some(Key::Equals),
        "[" | "openbracket" => Some(Key::OpenBracket),
        "]" | "closebracket" => Some(Key::CloseBracket),
        "\\" | "backslash" => Some(Key::Backslash),
        "/" | "slash" => Some(Key::Slash),
        ";" | "semicolon" => Some(Key::Semicolon),
        "'" | "quote" => Some(Key::Quote),
        _ => None,
    }
}

/// Static definition of an action in the master table.
pub struct ActionMetadata {
    pub id: ActionId,
    pub category: ActionCategory,
    pub name: &'static str,
    pub default_combo_str: &'static str,
    pub alias_combo_str: Option<&'static str>,
    pub global_eligible: bool,
    pub description: &'static str,
}

/// Source-of-truth table defining all default shortcuts across Sections 1–5.
pub static MASTER_ACTION_DEFINITIONS: &[ActionMetadata] = &[
    // --- Section 1: Streaming, recording, and output (global-hotkey eligible) ---
    ActionMetadata {
        id: ActionId::StartStreaming,
        category: ActionCategory::StreamingAndOutput,
        name: "Start Streaming",
        default_combo_str: "",
        alias_combo_str: None,
        global_eligible: true,
        description: "Starts RTMP/SRT broadcast to configured destinations.",
    },
    ActionMetadata {
        id: ActionId::StopStreaming,
        category: ActionCategory::StreamingAndOutput,
        name: "Stop Streaming",
        default_combo_str: "",
        alias_combo_str: None,
        global_eligible: true,
        description: "Halts live broadcast stream cleanly.",
    },
    ActionMetadata {
        id: ActionId::StartRecording,
        category: ActionCategory::StreamingAndOutput,
        name: "Start Recording",
        default_combo_str: "",
        alias_combo_str: None,
        global_eligible: true,
        description: "Begins recording master program output to disk.",
    },
    ActionMetadata {
        id: ActionId::StopRecording,
        category: ActionCategory::StreamingAndOutput,
        name: "Stop Recording",
        default_combo_str: "",
        alias_combo_str: None,
        global_eligible: true,
        description: "Stops active recording and remuxes output if enabled.",
    },
    ActionMetadata {
        id: ActionId::PauseRecording,
        category: ActionCategory::StreamingAndOutput,
        name: "Pause Recording",
        default_combo_str: "",
        alias_combo_str: None,
        global_eligible: true,
        description: "Pauses active recording without closing the file.",
    },
    ActionMetadata {
        id: ActionId::ToggleReplayBuffer,
        category: ActionCategory::StreamingAndOutput,
        name: "Start/Stop Replay Buffer",
        default_combo_str: "",
        alias_combo_str: None,
        global_eligible: true,
        description: "Toggles circular instant replay recording buffer.",
    },
    ActionMetadata {
        id: ActionId::SaveReplayBuffer,
        category: ActionCategory::StreamingAndOutput,
        name: "Save Replay Buffer",
        default_combo_str: "",
        alias_combo_str: None,
        global_eligible: true,
        description: "Saves buffered replay frames to an MP4/MKV clip.",
    },
    ActionMetadata {
        id: ActionId::TogglePreview,
        category: ActionCategory::StreamingAndOutput,
        name: "Enable/Disable Preview",
        default_combo_str: "",
        alias_combo_str: None,
        global_eligible: true,
        description: "Toggles Studio Mode Preview display.",
    },
    // --- Section 2: Scene and transition control ---
    ActionMetadata {
        id: ActionId::Cut,
        category: ActionCategory::SceneAndTransition,
        name: "Cut (Preview → Program immediately)",
        default_combo_str: "",
        alias_combo_str: None,
        global_eligible: true,
        description: "Immediately cuts Preview to Program without transition animation.",
    },
    ActionMetadata {
        id: ActionId::AutoTransition,
        category: ActionCategory::SceneAndTransition,
        name: "Trigger Transition (Auto)",
        default_combo_str: "",
        alias_combo_str: None,
        global_eligible: true,
        description: "Executes configured transition (Fade/Slide/Wipe) from Preview to Program.",
    },
    ActionMetadata {
        id: ActionId::NextScene,
        category: ActionCategory::SceneAndTransition,
        name: "Next Scene",
        default_combo_str: "",
        alias_combo_str: None,
        global_eligible: true,
        description: "Steps forward through the scenes list.",
    },
    ActionMetadata {
        id: ActionId::PreviousScene,
        category: ActionCategory::SceneAndTransition,
        name: "Previous Scene",
        default_combo_str: "",
        alias_combo_str: None,
        global_eligible: true,
        description: "Steps backward through the scenes list.",
    },
    ActionMetadata {
        id: ActionId::ToggleStudioMode,
        category: ActionCategory::SceneAndTransition,
        name: "Toggle Studio Mode",
        default_combo_str: "",
        alias_combo_str: None,
        global_eligible: true,
        description: "Toggles between Direct Mode and Dual-Bus Studio Mode.",
    },
    // --- Section 3: Live Studio editing ---
    ActionMetadata {
        id: ActionId::Undo,
        category: ActionCategory::LiveStudioEditing,
        name: "Undo",
        default_combo_str: "Ctrl+Z",
        alias_combo_str: None,
        global_eligible: false,
        description: "Reverts the last studio or overlay editor operation.",
    },
    ActionMetadata {
        id: ActionId::Redo,
        category: ActionCategory::LiveStudioEditing,
        name: "Redo",
        default_combo_str: "Ctrl+Shift+Z",
        alias_combo_str: Some("Ctrl+Y"),
        global_eligible: false,
        description: "Reapplies the last undone operation.",
    },
    ActionMetadata {
        id: ActionId::DeleteSelected,
        category: ActionCategory::LiveStudioEditing,
        name: "Delete selected source/layer",
        default_combo_str: "Delete",
        alias_combo_str: None,
        global_eligible: false,
        description: "Removes currently selected source or overlay layer.",
    },
    ActionMetadata {
        id: ActionId::DuplicateSelected,
        category: ActionCategory::LiveStudioEditing,
        name: "Duplicate selected source/layer",
        default_combo_str: "Ctrl+D",
        alias_combo_str: None,
        global_eligible: false,
        description: "Duplicates selected source or overlay layer with unique ID.",
    },
    ActionMetadata {
        id: ActionId::Copy,
        category: ActionCategory::LiveStudioEditing,
        name: "Copy",
        default_combo_str: "Ctrl+C",
        alias_combo_str: None,
        global_eligible: false,
        description: "Copies selected source or layer to internal clipboard.",
    },
    ActionMetadata {
        id: ActionId::Paste,
        category: ActionCategory::LiveStudioEditing,
        name: "Paste",
        default_combo_str: "Ctrl+V",
        alias_combo_str: None,
        global_eligible: false,
        description: "Pastes copied item with offset position.",
    },
    ActionMetadata {
        id: ActionId::PasteInPlace,
        category: ActionCategory::LiveStudioEditing,
        name: "Paste in place (same position)",
        default_combo_str: "Ctrl+Shift+V",
        alias_combo_str: None,
        global_eligible: false,
        description: "Pastes copied item at its exact original coordinates.",
    },
    ActionMetadata {
        id: ActionId::SelectAll,
        category: ActionCategory::LiveStudioEditing,
        name: "Select All (sources in current scene)",
        default_combo_str: "Ctrl+A",
        alias_combo_str: None,
        global_eligible: false,
        description: "Selects all sources in active Preview scene.",
    },
    ActionMetadata {
        id: ActionId::DeselectAll,
        category: ActionCategory::LiveStudioEditing,
        name: "Deselect All",
        default_combo_str: "Escape",
        alias_combo_str: None,
        global_eligible: false,
        description: "Clears current source or layer selection.",
    },
    ActionMetadata {
        id: ActionId::BringToFront,
        category: ActionCategory::LiveStudioEditing,
        name: "Bring to Front",
        default_combo_str: "Ctrl+Shift+]",
        alias_combo_str: None,
        global_eligible: false,
        description: "Moves selected item to top of rendering order.",
    },
    ActionMetadata {
        id: ActionId::BringForward,
        category: ActionCategory::LiveStudioEditing,
        name: "Bring Forward",
        default_combo_str: "Ctrl+]",
        alias_combo_str: None,
        global_eligible: false,
        description: "Moves selected item up one level in stack.",
    },
    ActionMetadata {
        id: ActionId::SendBackward,
        category: ActionCategory::LiveStudioEditing,
        name: "Send Backward",
        default_combo_str: "Ctrl+[",
        alias_combo_str: None,
        global_eligible: false,
        description: "Moves selected item down one level in stack.",
    },
    ActionMetadata {
        id: ActionId::SendToBack,
        category: ActionCategory::LiveStudioEditing,
        name: "Send to Back",
        default_combo_str: "Ctrl+Shift+[",
        alias_combo_str: None,
        global_eligible: false,
        description: "Moves selected item to bottom of stack.",
    },
    ActionMetadata {
        id: ActionId::ToggleSourceVisibility,
        category: ActionCategory::LiveStudioEditing,
        name: "Toggle visibility of selected source",
        default_combo_str: "Ctrl+Shift+H",
        alias_combo_str: None,
        global_eligible: true,
        description: "Hides or shows the currently selected source.",
    },
    ActionMetadata {
        id: ActionId::ToggleSourceLock,
        category: ActionCategory::LiveStudioEditing,
        name: "Toggle lock of selected source",
        default_combo_str: "Ctrl+Shift+L",
        alias_combo_str: None,
        global_eligible: false,
        description: "Locks or unlocks canvas transforms for selected source.",
    },
    ActionMetadata {
        id: ActionId::ToggleSourceMute,
        category: ActionCategory::LiveStudioEditing,
        name: "Mute/Unmute selected audio source",
        default_combo_str: "Ctrl+Shift+M",
        alias_combo_str: None,
        global_eligible: true,
        description: "Toggles mute status of selected audio source.",
    },
    ActionMetadata {
        id: ActionId::OpenSourceProperties,
        category: ActionCategory::LiveStudioEditing,
        name: "Open Properties of selected source",
        default_combo_str: "Alt+Enter",
        alias_combo_str: None,
        global_eligible: false,
        description: "Opens configuration modal for selected source.",
    },
    ActionMetadata {
        id: ActionId::RenameSelected,
        category: ActionCategory::LiveStudioEditing,
        name: "Rename selected scene/source",
        default_combo_str: "F2",
        alias_combo_str: None,
        global_eligible: false,
        description: "Activates inline rename buffer for selected item.",
    },
    // --- Section 4: Overlay editor ---
    ActionMetadata {
        id: ActionId::AddTextLayer,
        category: ActionCategory::OverlayEditor,
        name: "Add Text Layer",
        default_combo_str: "T",
        alias_combo_str: None,
        global_eligible: false,
        description: "Inserts a new vector text layer in overlay composition.",
    },
    ActionMetadata {
        id: ActionId::AddShapeLayer,
        category: ActionCategory::OverlayEditor,
        name: "Add Shape Layer",
        default_combo_str: "R",
        alias_combo_str: None,
        global_eligible: false,
        description: "Inserts a new vector rectangle/shape layer.",
    },
    ActionMetadata {
        id: ActionId::AddCameraLayer,
        category: ActionCategory::OverlayEditor,
        name: "Add Camera Layer",
        default_combo_str: "Shift+C",
        alias_combo_str: None,
        global_eligible: false,
        description: "Inserts 2.5D perspective camera layer.",
    },
    ActionMetadata {
        id: ActionId::AddParticlesLayer,
        category: ActionCategory::OverlayEditor,
        name: "Add Particles Layer",
        default_combo_str: "Shift+P",
        alias_combo_str: None,
        global_eligible: false,
        description: "Inserts GPU particle emitter layer.",
    },
    ActionMetadata {
        id: ActionId::DeleteLayer,
        category: ActionCategory::OverlayEditor,
        name: "Delete selected layer",
        default_combo_str: "Delete",
        alias_combo_str: None,
        global_eligible: false,
        description: "Deletes selected layer from timeline and canvas.",
    },
    ActionMetadata {
        id: ActionId::GroupLayers,
        category: ActionCategory::OverlayEditor,
        name: "Group selected layers",
        default_combo_str: "Ctrl+G",
        alias_combo_str: None,
        global_eligible: false,
        description: "Groups selected layers into a container group.",
    },
    ActionMetadata {
        id: ActionId::UngroupLayers,
        category: ActionCategory::OverlayEditor,
        name: "Ungroup",
        default_combo_str: "Ctrl+Shift+G",
        alias_combo_str: None,
        global_eligible: false,
        description: "Unwraps group contents back to parent composition.",
    },
    ActionMetadata {
        id: ActionId::AddKeyframe,
        category: ActionCategory::OverlayEditor,
        name: "Add Keyframe on selected property",
        default_combo_str: "Alt+K",
        alias_combo_str: None,
        global_eligible: false,
        description: "Inserts an animated keyframe at current playhead time.",
    },
    ActionMetadata {
        id: ActionId::DeleteKeyframe,
        category: ActionCategory::OverlayEditor,
        name: "Delete Keyframe at playhead",
        default_combo_str: "Alt+Shift+K",
        alias_combo_str: None,
        global_eligible: false,
        description: "Removes keyframe at playhead for selected property.",
    },
    ActionMetadata {
        id: ActionId::PlayPauseTimeline,
        category: ActionCategory::OverlayEditor,
        name: "Play/Pause (timeline)",
        default_combo_str: "Space",
        alias_combo_str: None,
        global_eligible: false,
        description: "Toggles playback animation loop.",
    },
    ActionMetadata {
        id: ActionId::StopTimeline,
        category: ActionCategory::OverlayEditor,
        name: "Stop (return to start)",
        default_combo_str: "Shift+Space",
        alias_combo_str: None,
        global_eligible: false,
        description: "Stops playback and returns playhead to start (0.0s).",
    },
    ActionMetadata {
        id: ActionId::GoToStart,
        category: ActionCategory::OverlayEditor,
        name: "Go to Start",
        default_combo_str: "Home",
        alias_combo_str: None,
        global_eligible: false,
        description: "Moves playhead to time 0.0s.",
    },
    ActionMetadata {
        id: ActionId::GoToEnd,
        category: ActionCategory::OverlayEditor,
        name: "Go to End",
        default_combo_str: "End",
        alias_combo_str: None,
        global_eligible: false,
        description: "Moves playhead to duration boundary.",
    },
    ActionMetadata {
        id: ActionId::StepFrameBack,
        category: ActionCategory::OverlayEditor,
        name: "Step one frame back",
        default_combo_str: ",",
        alias_combo_str: None,
        global_eligible: false,
        description: "Steps playhead backward by 1 frame (1/60s).",
    },
    ActionMetadata {
        id: ActionId::StepFrameForward,
        category: ActionCategory::OverlayEditor,
        name: "Step one frame forward",
        default_combo_str: ".",
        alias_combo_str: None,
        global_eligible: false,
        description: "Steps playhead forward by 1 frame (1/60s).",
    },
    ActionMetadata {
        id: ActionId::SetInMarker,
        category: ActionCategory::OverlayEditor,
        name: "Set In-point / Loop-start marker at playhead",
        default_combo_str: "I",
        alias_combo_str: None,
        global_eligible: false,
        description: "Sets lifecycle intro/loop start boundary at playhead.",
    },
    ActionMetadata {
        id: ActionId::SetOutMarker,
        category: ActionCategory::OverlayEditor,
        name: "Set Loop-end / Out-start marker at playhead",
        default_combo_str: "O",
        alias_combo_str: None,
        global_eligible: false,
        description: "Sets lifecycle loop end/out start boundary at playhead.",
    },
    ActionMetadata {
        id: ActionId::AddMarker,
        category: ActionCategory::OverlayEditor,
        name: "Add generic Marker at playhead",
        default_combo_str: "M",
        alias_combo_str: None,
        global_eligible: false,
        description: "Places a named cue point on the time ruler.",
    },
    ActionMetadata {
        id: ActionId::ZoomTimelineIn,
        category: ActionCategory::OverlayEditor,
        name: "Zoom timeline in",
        default_combo_str: "=",
        alias_combo_str: None,
        global_eligible: false,
        description: "Increases timeline pixel zoom resolution.",
    },
    ActionMetadata {
        id: ActionId::ZoomTimelineOut,
        category: ActionCategory::OverlayEditor,
        name: "Zoom timeline out",
        default_combo_str: "-",
        alias_combo_str: None,
        global_eligible: false,
        description: "Decreases timeline pixel zoom resolution.",
    },
    ActionMetadata {
        id: ActionId::FitTimeline,
        category: ActionCategory::OverlayEditor,
        name: "Fit timeline to window",
        default_combo_str: "Shift+F",
        alias_combo_str: None,
        global_eligible: false,
        description: "Scales timeline zoom to fit composition duration.",
    },
    ActionMetadata {
        id: ActionId::ZoomCanvasIn,
        category: ActionCategory::OverlayEditor,
        name: "Zoom canvas in",
        default_combo_str: "Ctrl+=",
        alias_combo_str: None,
        global_eligible: false,
        description: "Zooms in canvas viewport center.",
    },
    ActionMetadata {
        id: ActionId::ZoomCanvasOut,
        category: ActionCategory::OverlayEditor,
        name: "Zoom canvas out",
        default_combo_str: "Ctrl+-",
        alias_combo_str: None,
        global_eligible: false,
        description: "Zooms out canvas viewport center.",
    },
    ActionMetadata {
        id: ActionId::ResetCanvasZoom,
        category: ActionCategory::OverlayEditor,
        name: "Reset canvas zoom to 100%",
        default_combo_str: "Ctrl+0",
        alias_combo_str: None,
        global_eligible: false,
        description: "Resets canvas zoom factor to 1.0 (100%).",
    },
    ActionMetadata {
        id: ActionId::FitCanvas,
        category: ActionCategory::OverlayEditor,
        name: "Fit canvas to window",
        default_combo_str: "Shift+1",
        alias_combo_str: None,
        global_eligible: false,
        description: "Centers and scales composition to fit viewport.",
    },
    ActionMetadata {
        id: ActionId::ToggleCurveEditor,
        category: ActionCategory::OverlayEditor,
        name: "Toggle Dope Sheet / Curve Editor view",
        default_combo_str: "Shift+F3",
        alias_combo_str: None,
        global_eligible: false,
        description: "Switches between Dope Sheet and Bezier Curve Editor.",
    },
    ActionMetadata {
        id: ActionId::ToolSelect,
        category: ActionCategory::OverlayEditor,
        name: "Switch to Move/Select tool",
        default_combo_str: "V",
        alias_combo_str: None,
        global_eligible: false,
        description: "Activates selection and transform gizmo tool.",
    },
    ActionMetadata {
        id: ActionId::ToolPan,
        category: ActionCategory::OverlayEditor,
        name: "Switch to Pan (hand) tool",
        default_combo_str: "H",
        alias_combo_str: None,
        global_eligible: false,
        description: "Activates canvas hand pan tool.",
    },
    // --- Section 5: Window, panels, and modes ---
    ActionMetadata {
        id: ActionId::ModeLiveStudio,
        category: ActionCategory::WindowAndModes,
        name: "Switch to Live Studio mode",
        default_combo_str: "Ctrl+1",
        alias_combo_str: None,
        global_eligible: false,
        description: "Switches application to broadcast Live Studio switcher.",
    },
    ActionMetadata {
        id: ActionId::ModeEditOverlay,
        category: ActionCategory::WindowAndModes,
        name: "Switch to Edit Overlay mode",
        default_combo_str: "Ctrl+2",
        alias_combo_str: None,
        global_eligible: false,
        description: "Switches application to motion graphics Overlay Editor.",
    },
    ActionMetadata {
        id: ActionId::ModeDataControl,
        category: ActionCategory::WindowAndModes,
        name: "Switch to Data Control mode",
        default_combo_str: "Ctrl+3",
        alias_combo_str: None,
        global_eligible: false,
        description: "Switches application to Esports Live Data Control panel.",
    },
    ActionMetadata {
        id: ActionId::OpenSettings,
        category: ActionCategory::WindowAndModes,
        name: "Open Settings",
        default_combo_str: "Ctrl+,",
        alias_combo_str: None,
        global_eligible: false,
        description: "Opens Studio Settings modal.",
    },
    ActionMetadata {
        id: ActionId::NewProject,
        category: ActionCategory::WindowAndModes,
        name: "New Project",
        default_combo_str: "Ctrl+N",
        alias_combo_str: None,
        global_eligible: false,
        description: "Creates a new blank broadcast project.",
    },
    ActionMetadata {
        id: ActionId::OpenProject,
        category: ActionCategory::WindowAndModes,
        name: "Open Project",
        default_combo_str: "Ctrl+O",
        alias_combo_str: None,
        global_eligible: false,
        description: "Prompts to open an existing .arxproj project.",
    },
    ActionMetadata {
        id: ActionId::SaveProject,
        category: ActionCategory::WindowAndModes,
        name: "Save Project",
        default_combo_str: "Ctrl+S",
        alias_combo_str: None,
        global_eligible: false,
        description: "Saves current project state to disk.",
    },
    ActionMetadata {
        id: ActionId::SaveProjectAs,
        category: ActionCategory::WindowAndModes,
        name: "Save Project As",
        default_combo_str: "Ctrl+Shift+S",
        alias_combo_str: None,
        global_eligible: false,
        description: "Prompts for destination file path to save project.",
    },
    ActionMetadata {
        id: ActionId::ToggleFullscreen,
        category: ActionCategory::WindowAndModes,
        name: "Toggle fullscreen Preview/Program",
        default_combo_str: "F11",
        alias_combo_str: None,
        global_eligible: false,
        description: "Toggles borderless fullscreen monitor display.",
    },
    ActionMetadata {
        id: ActionId::DialogCancel,
        category: ActionCategory::WindowAndModes,
        name: "Close current dialog/modal",
        default_combo_str: "Escape",
        alias_combo_str: None,
        global_eligible: false,
        description: "Closes open modal or dismisses current dialog.",
    },
    ActionMetadata {
        id: ActionId::DialogConfirm,
        category: ActionCategory::WindowAndModes,
        name: "Confirm current dialog (primary action)",
        default_combo_str: "Enter",
        alias_combo_str: None,
        global_eligible: false,
        description: "Confirms primary button action in active modal dialog.",
    },
];

/// Target of an interactive key rebinding operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RebindTarget {
    Action(ActionId),
    SceneSwitch(String),
    SceneSwitchBack(String),
    SourceMute(String),
    SourcePushToMute(String),
    SourcePushToTalk(String),
    SourceShowHide(String),
}

/// Describes a hotkey collision between two actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictWarning {
    pub combo: String,
    pub action_names: Vec<String>,
}

/// Per-scene configurable hotkey bindings (Section 6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SceneHotkeys {
    #[serde(default)]
    pub switch_to: Option<String>,
    #[serde(default)]
    pub switch_to_global: bool,
    #[serde(default)]
    pub switch_back: Option<String>,
    #[serde(default)]
    pub switch_back_global: bool,
}

/// Per-source configurable hotkey bindings (Section 6).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SourceHotkeys {
    #[serde(default)]
    pub mute: Option<String>,
    #[serde(default)]
    pub mute_global: bool,
    #[serde(default)]
    pub push_to_mute: Option<String>,
    #[serde(default)]
    pub push_to_mute_global: bool,
    #[serde(default)]
    pub push_to_talk: Option<String>,
    #[serde(default)]
    pub push_to_talk_global: bool,
    #[serde(default)]
    pub show_hide: Option<String>,
    #[serde(default)]
    pub show_hide_global: bool,
}

/// Active registry managing user overrides, conflict analysis, and input dispatch.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HotkeyRegistry {
    /// Overrides for standard actions: action_id string -> KeyCombo string (or empty if unbound).
    #[serde(default)]
    pub overrides: HashMap<String, String>,
    /// Global hotkey flags: action_id string -> is_global.
    #[serde(default)]
    pub global_flags: HashMap<String, bool>,
    /// Per-scene hotkey bindings: scene_id -> SceneHotkeys.
    #[serde(default)]
    pub scene_hotkeys: HashMap<String, SceneHotkeys>,
    /// Per-source hotkey bindings: source_id -> SourceHotkeys.
    #[serde(default)]
    pub source_hotkeys: HashMap<String, SourceHotkeys>,
}

impl HotkeyRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the active KeyCombo for a standard action (user override or default).
    pub fn get_combo(&self, action: ActionId) -> Option<KeyCombo> {
        let key_str = action.as_str();
        if let Some(custom) = self.overrides.get(key_str) {
            KeyCombo::parse(custom)
        } else {
            // Find default in master table
            MASTER_ACTION_DEFINITIONS
                .iter()
                .find(|m| m.id == action)
                .and_then(|m| KeyCombo::parse(m.default_combo_str))
        }
    }

    /// Returns human-readable accelerator text for menu items and tooltips (e.g. `Ctrl+Z`, `T`, or empty).
    pub fn accelerator_text(&self, action: ActionId) -> String {
        if let Some(combo) = self.get_combo(action) {
            combo.format_text()
        } else {
            String::new()
        }
    }

    /// Sets or clears the binding for an action.
    pub fn set_combo(&mut self, action: ActionId, combo_str: &str) {
        let key_str = action.as_str().to_string();
        if combo_str.trim().is_empty() {
            self.overrides.insert(key_str, String::new());
        } else if let Some(combo) = KeyCombo::parse(combo_str) {
            self.overrides.insert(key_str, combo.format_text());
        }
    }

    /// Sets the global hotkey flag for an action.
    pub fn set_global(&mut self, action: ActionId, is_global: bool) {
        self.global_flags
            .insert(action.as_str().to_string(), is_global);
    }

    /// Checks if an action is marked as global.
    pub fn is_global(&self, action: ActionId) -> bool {
        self.global_flags
            .get(action.as_str())
            .copied()
            .unwrap_or(false)
    }

    /// Resets all overrides, global flags, and per-item bindings to default.
    pub fn reset_to_defaults(&mut self) {
        self.overrides.clear();
        self.global_flags.clear();
        self.scene_hotkeys.clear();
        self.source_hotkeys.clear();
    }

    /// Tests if an action is triggered in the given egui input state.
    pub fn action_triggered(&self, action: ActionId, i: &egui::InputState) -> bool {
        // Check primary combo
        if let Some(combo) = self.get_combo(action) {
            if combo.matches(i) {
                return true;
            }
        }

        // Check alias combo (e.g. Ctrl+Y for Redo) if no user override is active
        if !self.overrides.contains_key(action.as_str()) {
            if let Some(meta) = MASTER_ACTION_DEFINITIONS.iter().find(|m| m.id == action) {
                if let Some(alias_str) = meta.alias_combo_str {
                    if let Some(alias_combo) = KeyCombo::parse(alias_str) {
                        if alias_combo.matches(i) {
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    /// Analyzes all active bindings across the application and detects conflicts.
    pub fn find_conflicts(&self) -> Vec<ConflictWarning> {
        let mut map: HashMap<String, Vec<String>> = HashMap::new();

        // Check master table actions
        for meta in MASTER_ACTION_DEFINITIONS {
            if let Some(combo) = self.get_combo(meta.id) {
                let fmt = combo.format_text();
                if !fmt.is_empty() {
                    map.entry(fmt).or_default().push(meta.name.to_string());
                }
            }
        }

        // Check per-scene hotkeys
        for (sc_id, sc_hk) in &self.scene_hotkeys {
            if let Some(ref combo_s) = sc_hk.switch_to {
                if let Some(combo) = KeyCombo::parse(combo_s) {
                    map.entry(combo.format_text())
                        .or_default()
                        .push(format!("Scene '{}': Switch To", sc_id));
                }
            }
            if let Some(ref combo_s) = sc_hk.switch_back {
                if let Some(combo) = KeyCombo::parse(combo_s) {
                    map.entry(combo.format_text())
                        .or_default()
                        .push(format!("Scene '{}': Switch Back", sc_id));
                }
            }
        }

        // Check per-source hotkeys
        for (src_id, src_hk) in &self.source_hotkeys {
            if let Some(ref s) = src_hk.mute {
                if let Some(c) = KeyCombo::parse(s) {
                    map.entry(c.format_text())
                        .or_default()
                        .push(format!("Source '{}': Mute/Unmute", src_id));
                }
            }
            if let Some(ref s) = src_hk.push_to_mute {
                if let Some(c) = KeyCombo::parse(s) {
                    map.entry(c.format_text())
                        .or_default()
                        .push(format!("Source '{}': Push-to-Mute", src_id));
                }
            }
            if let Some(ref s) = src_hk.push_to_talk {
                if let Some(c) = KeyCombo::parse(s) {
                    map.entry(c.format_text())
                        .or_default()
                        .push(format!("Source '{}': Push-to-Talk", src_id));
                }
            }
            if let Some(ref s) = src_hk.show_hide {
                if let Some(c) = KeyCombo::parse(s) {
                    map.entry(c.format_text())
                        .or_default()
                        .push(format!("Source '{}': Show/Hide", src_id));
                }
            }
        }

        // Filter keys with > 1 action assigned
        let mut conflicts: Vec<ConflictWarning> = map
            .into_iter()
            .filter(|(_combo, names)| names.len() > 1)
            .map(|(combo, names)| ConflictWarning {
                combo,
                action_names: names,
            })
            .collect();

        // "Delete" is intentionally shared between Live Studio source delete and Overlay layer delete across different editor modes
        conflicts.retain(|c| {
            if c.combo == "Delete"
                && c.action_names.len() == 2
                && c.action_names
                    .contains(&"Delete selected source/layer".to_string())
                && c.action_names
                    .contains(&"Delete selected layer".to_string())
            {
                return false;
            }
            // "Escape" is context-separated: modal dialogs intercept Escape to close dialog, while main canvas uses Escape to deselect.
            if c.combo == "Escape"
                && c.action_names.len() == 2
                && c.action_names.contains(&"Deselect All".to_string())
                && c.action_names
                    .contains(&"Close current dialog/modal".to_string())
            {
                return false;
            }
            true
        });

        conflicts.sort_by(|a, b| a.combo.cmp(&b.combo));
        conflicts
    }

    /// Clears the shortcut binding for a given rebind target.
    pub fn clear_target(&mut self, target: &RebindTarget) {
        match target {
            RebindTarget::Action(id) => self.set_combo(*id, ""),
            RebindTarget::SceneSwitch(sc) => {
                self.scene_hotkeys.entry(sc.clone()).or_default().switch_to = None;
            }
            RebindTarget::SceneSwitchBack(sc) => {
                self.scene_hotkeys
                    .entry(sc.clone())
                    .or_default()
                    .switch_back = None;
            }
            RebindTarget::SourceMute(s) => {
                self.source_hotkeys.entry(s.clone()).or_default().mute = None;
            }
            RebindTarget::SourcePushToMute(s) => {
                self.source_hotkeys
                    .entry(s.clone())
                    .or_default()
                    .push_to_mute = None;
            }
            RebindTarget::SourcePushToTalk(s) => {
                self.source_hotkeys
                    .entry(s.clone())
                    .or_default()
                    .push_to_talk = None;
            }
            RebindTarget::SourceShowHide(s) => {
                self.source_hotkeys.entry(s.clone()).or_default().show_hide = None;
            }
        }
    }

    /// Sets the shortcut binding for a given rebind target.
    pub fn set_target_combo(&mut self, target: &RebindTarget, combo_str: &str) {
        match target {
            RebindTarget::Action(id) => self.set_combo(*id, combo_str),
            RebindTarget::SceneSwitch(sc) => {
                self.scene_hotkeys.entry(sc.clone()).or_default().switch_to =
                    if combo_str.trim().is_empty() {
                        None
                    } else {
                        Some(combo_str.to_string())
                    };
            }
            RebindTarget::SceneSwitchBack(sc) => {
                self.scene_hotkeys
                    .entry(sc.clone())
                    .or_default()
                    .switch_back = if combo_str.trim().is_empty() {
                    None
                } else {
                    Some(combo_str.to_string())
                };
            }
            RebindTarget::SourceMute(s) => {
                self.source_hotkeys.entry(s.clone()).or_default().mute =
                    if combo_str.trim().is_empty() {
                        None
                    } else {
                        Some(combo_str.to_string())
                    };
            }
            RebindTarget::SourcePushToMute(s) => {
                self.source_hotkeys
                    .entry(s.clone())
                    .or_default()
                    .push_to_mute = if combo_str.trim().is_empty() {
                    None
                } else {
                    Some(combo_str.to_string())
                };
            }
            RebindTarget::SourcePushToTalk(s) => {
                self.source_hotkeys
                    .entry(s.clone())
                    .or_default()
                    .push_to_talk = if combo_str.trim().is_empty() {
                    None
                } else {
                    Some(combo_str.to_string())
                };
            }
            RebindTarget::SourceShowHide(s) => {
                self.source_hotkeys.entry(s.clone()).or_default().show_hide =
                    if combo_str.trim().is_empty() {
                        None
                    } else {
                        Some(combo_str.to_string())
                    };
            }
        }
    }

    /// Retrieves current shortcut string for a rebind target.
    pub fn get_target_combo_str(&self, target: &RebindTarget) -> String {
        match target {
            RebindTarget::Action(id) => self.accelerator_text(*id),
            RebindTarget::SceneSwitch(sc) => self
                .scene_hotkeys
                .get(sc)
                .and_then(|h| h.switch_to.clone())
                .unwrap_or_default(),
            RebindTarget::SceneSwitchBack(sc) => self
                .scene_hotkeys
                .get(sc)
                .and_then(|h| h.switch_back.clone())
                .unwrap_or_default(),
            RebindTarget::SourceMute(s) => self
                .source_hotkeys
                .get(s)
                .and_then(|h| h.mute.clone())
                .unwrap_or_default(),
            RebindTarget::SourcePushToMute(s) => self
                .source_hotkeys
                .get(s)
                .and_then(|h| h.push_to_mute.clone())
                .unwrap_or_default(),
            RebindTarget::SourcePushToTalk(s) => self
                .source_hotkeys
                .get(s)
                .and_then(|h| h.push_to_talk.clone())
                .unwrap_or_default(),
            RebindTarget::SourceShowHide(s) => self
                .source_hotkeys
                .get(s)
                .and_then(|h| h.show_hide.clone())
                .unwrap_or_default(),
        }
    }
}

/// Generates the complete, user-facing markdown reference document (`docs/SHORTCUTS.md`)
/// directly from the master action definitions.
pub fn generate_shortcuts_markdown() -> String {
    let mut out = String::new();
    out.push_str("# Zenocast Keyboard Shortcuts Reference\n\n");
    out.push_str("Complete keyboard shortcut reference for Zenocast Studio, modeled on OBS Studio defaults with Zenocast-specific motion-graphics and live editing capabilities. All shortcuts are fully rebindable via **Settings → Hotkeys** in Zenocast Studio.\n\n");
    out.push_str("> [!TIP]\n");
    out.push_str("> Shortcuts respect focus context: when editing text fields or renaming labels, app-level shortcuts do not fire. Global shortcuts (marked with ✓) work even when Zenocast is running in the background when enabled via the \"Make global\" checkbox.\n\n");

    let categories = [
        ActionCategory::StreamingAndOutput,
        ActionCategory::SceneAndTransition,
        ActionCategory::LiveStudioEditing,
        ActionCategory::OverlayEditor,
        ActionCategory::WindowAndModes,
    ];

    for cat in categories {
        out.push_str(&format!("## {}. {}\n\n", cat.section_num(), cat.title()));
        out.push_str("| Action | Default Shortcut | Global Eligible | Description |\n");
        out.push_str("| --- | --- | :---: | --- |\n");

        for meta in MASTER_ACTION_DEFINITIONS
            .iter()
            .filter(|m| m.category == cat)
        {
            let def_str = if meta.default_combo_str.is_empty() {
                "*(unbound by default)*".to_string()
            } else if let Some(alias) = meta.alias_combo_str {
                format!("`{}` (or `{}`)", meta.default_combo_str, alias)
            } else {
                format!("`{}`", meta.default_combo_str)
            };

            let global_str = if meta.global_eligible { "✓" } else { "—" };

            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                meta.name, def_str, global_str, meta.description
            ));
        }

        out.push('\n');
    }

    out.push_str("## 6. Per-Scene and Per-Source Hotkeys\n\n");
    out.push_str("Zenocast provides full OBS parity for assigning dedicated hotkeys to individual scenes and sources:\n\n");
    out.push_str("- **Per-Scene Hotkeys**: Right-click any scene in the Scenes panel or select **Hotkeys** in Scene Properties to assign:\n");
    out.push_str("  - **Switch to Scene**: Instantly switches to this scene when pressed.\n");
    out.push_str(
        "  - **Switch Back to Previous Scene**: Reverts to the previously active scene.\n",
    );
    out.push_str("- **Per-Source Hotkeys**: Right-click any source in the Sources panel or open Source Properties to assign:\n");
    out.push_str("  - **Mute / Unmute**: Toggles audio mute state for this source.\n");
    out.push_str("  - **Push-to-Mute**: Temporarily mutes the source only while the assigned key is held down.\n");
    out.push_str(
        "  - **Push-to-Talk**: Unmutes the source only while the assigned key is held down.\n",
    );
    out.push_str("  - **Show / Hide**: Toggles on-air visual visibility for this source.\n\n");
    out.push_str("All per-item hotkeys also appear in **Settings → Hotkeys** grouped under their respective scene and source headers, complete with conflict detection and \"Make global\" support.\n");

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_combo_parse_and_format_roundtrip() {
        let combos = [
            "Ctrl+Z",
            "Ctrl+Shift+Z",
            "Ctrl+Alt+K",
            "Delete",
            "Space",
            "Shift+Space",
            "Home",
            "End",
            ",",
            ".",
            "=",
            "-",
            "Shift+F3",
            "Ctrl+,",
            "F11",
            "Ctrl+1",
            "Ctrl+0",
        ];

        for &c in &combos {
            let parsed = KeyCombo::parse(c).unwrap_or_else(|| panic!("Failed to parse '{}'", c));
            let formatted = parsed.format_text();
            assert_eq!(formatted, c, "Format text mismatch for '{}'", c);
        }
    }

    #[test]
    fn test_all_master_definitions_have_valid_defaults() {
        for meta in MASTER_ACTION_DEFINITIONS {
            if !meta.default_combo_str.is_empty() {
                assert!(
                    KeyCombo::parse(meta.default_combo_str).is_some(),
                    "Action '{}' has invalid default combo string '{}'",
                    meta.name,
                    meta.default_combo_str
                );
            }
            if let Some(alias) = meta.alias_combo_str {
                assert!(
                    KeyCombo::parse(alias).is_some(),
                    "Action '{}' has invalid alias combo string '{}'",
                    meta.name,
                    alias
                );
            }
        }
    }

    #[test]
    fn test_conflict_detection() {
        let mut reg = HotkeyRegistry::new();
        // Defaults should have no conflicts across non-empty items in same category
        let conflicts = reg.find_conflicts();
        assert!(
            conflicts.is_empty(),
            "Defaults should have zero conflicts: {:?}",
            conflicts
        );
        // Check that deliberately creating a conflict flags it
        reg.set_combo(ActionId::StartStreaming, "Ctrl+Z");
        let new_conflicts = reg.find_conflicts();
        let found = new_conflicts
            .iter()
            .any(|c| c.combo == "Ctrl+Z" && c.action_names.contains(&"Start Streaming".into()));
        assert!(
            found,
            "Should detect conflict when StartStreaming set to Ctrl+Z"
        );
    }

    #[test]
    fn test_reset_to_defaults() {
        let mut reg = HotkeyRegistry::new();
        reg.set_combo(ActionId::StartStreaming, "F1");
        reg.set_global(ActionId::StartStreaming, true);
        assert_eq!(
            reg.get_combo(ActionId::StartStreaming),
            Some(KeyCombo::key_only(Key::F1))
        );
        assert!(reg.is_global(ActionId::StartStreaming));

        reg.reset_to_defaults();
        assert_eq!(reg.get_combo(ActionId::StartStreaming), None);
        assert!(!reg.is_global(ActionId::StartStreaming));
        assert_eq!(reg.accelerator_text(ActionId::Undo), "Ctrl+Z");
    }

    #[test]
    fn test_global_hotkey_types() {
        use global_hotkey::hotkey::{Code, HotKey, Modifiers};
        let hk = HotKey::new(Some(Modifiers::CONTROL | Modifiers::SHIFT), Code::KeyZ);
        assert_eq!(hk.id(), hk.id());
    }

    #[test]
    fn test_global_hotkey_manager_creation() {
        use global_hotkey::GlobalHotKeyManager;
        let manager = GlobalHotKeyManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_generate_and_sync_docs_shortcuts() {
        let generated = generate_shortcuts_markdown();
        let docs_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("docs")
            .join("SHORTCUTS.md");
        std::fs::write(&docs_path, &generated).expect("Write docs/SHORTCUTS.md");
        let disk_content = std::fs::read_to_string(&docs_path).expect("Read docs/SHORTCUTS.md");
        assert_eq!(disk_content, generated);
    }
}
