//! Project management, serialization (.arxproj), 30s autosave, and crash recovery.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::settings::StudioSettings;

/// Project error enumeration.
#[derive(Debug, Error)]
pub enum ProjectError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Project file corrupt or unsupported version: {0}")]
    InvalidFormat(String),
}

/// Source type tag for UI reconstruction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum SourceKind {
    DisplayCapture,
    WindowCapture,
    CameraFeed,
    MediaFile,
    Color,
    ArxOverlay,
    Browser,
}

/// Serialized source item in a scene.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceDefinition {
    pub id: String,
    pub name: String,
    pub kind: SourceKind,
    pub target: String,
    pub visible: bool,
    pub locked: bool,
    pub volume: f32,
    pub muted: bool,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Serialized scene containing ordered source items.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneDefinition {
    pub id: String,
    pub name: String,
    pub sources: Vec<SourceDefinition>,
}

/// Root Zenocast broadcast production project (`.arxproj`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub format_version: u32,
    pub name: String,
    pub scenes: Vec<SceneDefinition>,
    pub active_scene_id: Option<String>,
    pub preview_scene_id: Option<String>,
    pub settings: StudioSettings,
    pub last_modified: u64,
}

impl Default for Project {
    fn default() -> Self {
        let default_scene_1 = SceneDefinition {
            id: "scene_gameplay".into(),
            name: "Gameplay Main".into(),
            sources: vec![
                SourceDefinition {
                    id: "src_display".into(),
                    name: "Main Display (WGC)".into(),
                    kind: SourceKind::DisplayCapture,
                    target: "Display 1".into(),
                    visible: true,
                    locked: false,
                    volume: 1.0,
                    muted: false,
                    x: 0.0,
                    y: 0.0,
                    width: 1920.0,
                    height: 1080.0,
                },
                SourceDefinition {
                    id: "src_cam".into(),
                    name: "Player Cam".into(),
                    kind: SourceKind::CameraFeed,
                    target: "Media Foundation Cam 0".into(),
                    visible: true,
                    locked: false,
                    volume: 1.0,
                    muted: false,
                    x: 1440.0,
                    y: 810.0,
                    width: 480.0,
                    height: 270.0,
                },
            ],
        };

        let default_scene_2 = SceneDefinition {
            id: "scene_brb".into(),
            name: "Be Right Back".into(),
            sources: vec![SourceDefinition {
                id: "src_brb_overlay".into(),
                name: "BRB Lower-Third".into(),
                kind: SourceKind::ArxOverlay,
                target: "templates/esports/brb.arx".into(),
                visible: true,
                locked: true,
                volume: 1.0,
                muted: false,
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            }],
        };

        Self {
            format_version: 1,
            name: "Untitled Broadcast".into(),
            scenes: vec![default_scene_1, default_scene_2],
            active_scene_id: Some("scene_gameplay".into()),
            preview_scene_id: Some("scene_brb".into()),
            settings: StudioSettings::default(),
            last_modified: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }
}

impl Project {
    /// Saves the project to disk as pretty JSON.
    pub fn save_to_file(&mut self, path: &Path) -> Result<(), ProjectError> {
        self.last_modified = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let json = serde_json::to_string_pretty(self)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// Loads and parses an existing `.arxproj` from disk.
    pub fn load_from_file(path: &Path) -> Result<Self, ProjectError> {
        let content = fs::read_to_string(path)?;
        let project: Self = serde_json::from_str(&content)?;

        if project.format_version == 0 || project.format_version > 1 {
            return Err(ProjectError::InvalidFormat(format!(
                "Unsupported project format version: {}",
                project.format_version
            )));
        }

        Ok(project)
    }
}

/// Periodic 30-second background autosaver and crash restorer.
pub struct Autosaver {
    last_save: Instant,
    autosave_path: PathBuf,
    interval: Duration,
}

impl Autosaver {
    /// Initializes autosaver targeting the designated directory.
    pub fn new(directory: &Path) -> Self {
        Self {
            last_save: Instant::now(),
            autosave_path: directory.join(".autosave.arxproj"),
            interval: Duration::from_secs(30),
        }
    }

    /// Performs an autosave check every 30 seconds.
    pub fn check_and_save(&mut self, project: &mut Project) -> bool {
        if self.last_save.elapsed() >= self.interval {
            let _ = project.save_to_file(&self.autosave_path);
            self.last_save = Instant::now();
            return true;
        }
        false
    }

    /// Checks if a previous crash recovery file is present on disk.
    pub fn check_crash_recovery(directory: &Path) -> Option<PathBuf> {
        let recovery = directory.join(".autosave.arxproj");
        if recovery.exists() {
            Some(recovery)
        } else {
            None
        }
    }

    /// Cleans up the autosave marker upon normal exit.
    pub fn clear_crash_recovery(directory: &Path) {
        let recovery = directory.join(".autosave.arxproj");
        let _ = fs::remove_file(recovery);
    }
}
