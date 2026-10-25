//! User preferences persistence (per-install settings, not saved in .arxproj).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Persisted user preferences across sessions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UserPreferences {
    /// Whether Studio Mode (dual monitors with Preview/Program split) is enabled.
    /// Defaults to false (Normal Mode with single Program monitor, matching OBS Studio).
    pub studio_mode: bool,
}

/// Returns the configuration file path for user preferences.
pub fn user_preferences_path() -> PathBuf {
    if let Ok(dir) = std::env::var("ZENOCAST_CONFIG_DIR") {
        return PathBuf::from(dir).join("preferences.json");
    }
    #[cfg(target_os = "windows")]
    if let Ok(appdata) = std::env::var("APPDATA") {
        return PathBuf::from(appdata)
            .join("Zenocast")
            .join("preferences.json");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".zenocast")
            .join("preferences.json");
    }
    PathBuf::from(".zenocast_preferences.json")
}

impl UserPreferences {
    /// Loads preferences from disk, returning Default if file does not exist or fails to parse.
    pub fn load() -> Self {
        let path = user_preferences_path();
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(prefs) = serde_json::from_str::<UserPreferences>(&content) {
                return prefs;
            }
        }
        Self::default()
    }

    /// Persists preferences to disk.
    pub fn save(&self) {
        let path = user_preferences_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(&path, json);
        }
    }
}
