//! # zeno-ui
//!
//! User interface: broadcast production panels (Live, Edit Overlay, Data, Settings),
//! timeline keyframe manipulation, bezier curve editor, and strict design token theme.

pub mod app;
pub mod data_panel;
pub mod dock;
pub mod editor;
pub mod import_dialog;
pub mod log_viewer;
pub mod overlay_control;
pub mod performance_hud;
pub mod project;
pub mod settings;
pub mod spreadsheet_panel;
pub mod theme;

pub use app::ZenocastApp;
pub use data_panel::{ui_esports_data_panel, DataPanelState};
pub use dock::{create_default_live_layout, StudioTab, StudioTabViewer};
pub use editor::OverlayEditor;
pub use import_dialog::ImportReportDialog;
pub use log_viewer::{LogEntry, LogRingBuffer, LogSeverity, LogViewerPanel};
pub use overlay_control::ui_overlay_lifecycle_panel;
pub use performance_hud::{PerformanceOverlay, PerformanceSnapshot};
pub use project::{
    Autosaver, Project, ProjectError, SceneDefinition, SourceDefinition, SourceKind,
};
pub use settings::{AudioSettings, HotkeySettings, OutputSettings, StudioSettings, VideoSettings};
pub use spreadsheet_panel::{ui_spreadsheet_links_panel, SpreadsheetPanelState};
pub use theme::{
    apply_theme, color_to_egui, ACCENT, AMBER_WARN, BORDER_WIDTH, BUTTON_HEIGHT, CORNER_RADIUS,
    GREEN_OK, GRID_UNIT, RED_LIVE, ROW_HEIGHT_LIST, SURFACE_0, SURFACE_1, SURFACE_2, SURFACE_3,
    TEXT, TEXT_MUTED,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn collect_rs_files(dir: &Path, list: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    collect_rs_files(&path, list);
                } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                    list.push(path);
                }
            }
        }
    }

    #[test]
    fn test_strict_theme_and_no_color_literals_outside_theme() {
        let ui_src_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        assert!(ui_src_dir.exists(), "zeno-ui/src directory must exist");

        let mut files = Vec::new();
        collect_rs_files(&ui_src_dir, &mut files);

        for path in files {
            let file_name = path.file_name().unwrap().to_str().unwrap();
            if file_name == "theme.rs" || file_name == "lib.rs" {
                continue; // theme.rs defines tokens; lib.rs contains the scanner test itself
            }

            let content = fs::read_to_string(&path).expect("Failed to read file");

            // Check 1: No color constructor literals outside theme.rs
            assert!(
                !content.contains("Color32::from_rgb"),
                "File '{}' contains Color32::from_rgb! All colors must be imported from theme::*",
                file_name
            );
            assert!(
                !content.contains("Color32::from_rgba"),
                "File '{}' contains Color32::from_rgba! All colors must be imported from theme::*",
                file_name
            );

            // Check 2: No gradient APIs allowed anywhere per Section 7
            assert!(
                !content.to_lowercase().contains("gradient"),
                "File '{}' mentions gradients! Broadcast specification strictly forbids gradients.",
                file_name
            );
        }
    }

    #[test]
    fn test_project_serialization_round_trip() {
        let temp = tempdir().unwrap();
        let proj_path = temp.path().join("broadcast_test.arxproj");

        let mut original = Project {
            name: "Major League Grand Finals".into(),
            ..Default::default()
        };
        original.save_to_file(&proj_path).unwrap();

        let loaded = Project::load_from_file(&proj_path).unwrap();
        assert_eq!(loaded.name, "Major League Grand Finals");
        assert_eq!(loaded.format_version, 1);
        assert_eq!(loaded.scenes.len(), original.scenes.len());
        assert_eq!(loaded.scenes[0].name, original.scenes[0].name);
    }

    #[test]
    fn test_autosave_and_crash_recovery() {
        let temp = tempdir().unwrap();
        let mut autosaver = Autosaver::new(temp.path());

        assert!(Autosaver::check_crash_recovery(temp.path()).is_none());

        let mut project = Project {
            name: "Autosaved Broadcast".into(),
            ..Default::default()
        };

        // Exercise autosaver
        let _ = autosaver.check_and_save(&mut project);

        // Simulate save
        let recovery_path = temp.path().join(".autosave.arxproj");
        project.save_to_file(&recovery_path).unwrap();

        let recovery = Autosaver::check_crash_recovery(temp.path());
        assert!(recovery.is_some());

        let restored = Project::load_from_file(&recovery.unwrap()).unwrap();
        assert_eq!(restored.name, "Autosaved Broadcast");

        Autosaver::clear_crash_recovery(temp.path());
        assert!(Autosaver::check_crash_recovery(temp.path()).is_none());
    }

    #[test]
    fn test_dock_layout_initialization() {
        let dock_state = create_default_live_layout();
        let surface = dock_state.main_surface();
        assert_eq!(surface.num_tabs(), 6);
    }
}
