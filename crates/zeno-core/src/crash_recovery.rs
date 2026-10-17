//! Crash recovery and periodic autosave management.
//!
//! Tracks active session locks and writes atomic state snapshots every 30s.
//! If an unclean shutdown occurs, Zenocast detects the leftover lock on next launch
//! and offers one-click project state restoration.

use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;

use crate::error::{CoreError, CoreResult};

/// Active session lock metadata written at startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionLockData {
    /// Process ID that owns this session
    pub pid: u32,
    /// Unix timestamp when the session started
    pub start_timestamp: u64,
    /// Human readable application version
    pub app_version: String,
    /// Clean exit flag (set to true on orderly shutdown)
    pub clean_exit: bool,
}

/// Report returned when an unclosed session and autosave snapshot are discovered.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashRecoveryReport {
    /// Path to the autosave snapshot file
    pub autosave_path: PathBuf,
    /// Session lock info from the crashed session
    pub session_info: SessionLockData,
    /// Age of the autosave snapshot in seconds
    pub snapshot_timestamp: u64,
}

/// Manager handling session locking, atomic autosaving, and crash detection.
pub struct CrashRecoveryManager {
    work_dir: PathBuf,
    lock_file: PathBuf,
    autosave_file: PathBuf,
}

impl CrashRecoveryManager {
    /// Creates a new crash recovery manager operating in `work_dir`.
    pub fn new(work_dir: impl Into<PathBuf>) -> Self {
        let work_dir = work_dir.into();
        let lock_file = work_dir.join("session.lock");
        let autosave_file = work_dir.join("autosave.json");
        Self {
            work_dir,
            lock_file,
            autosave_file,
        }
    }

    /// Initializes a new session. Checks for prior crash before creating the new session lock.
    pub fn check_recovery(&self) -> Option<CrashRecoveryReport> {
        if !self.lock_file.exists() || !self.autosave_file.exists() {
            return None;
        }

        let lock_data_str = fs::read_to_string(&self.lock_file).ok()?;
        let lock_data: SessionLockData = serde_json::from_str(&lock_data_str).ok()?;

        if lock_data.clean_exit {
            return None;
        }

        let metadata = fs::metadata(&self.autosave_file).ok()?;
        let snapshot_timestamp = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Some(CrashRecoveryReport {
            autosave_path: self.autosave_file.clone(),
            session_info: lock_data,
            snapshot_timestamp,
        })
    }

    /// Creates the active session lock file for the current process.
    pub fn acquire_session(&self, pid: u32, app_version: impl Into<String>) -> CoreResult<()> {
        let _ = fs::create_dir_all(&self.work_dir);
        let start_timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let lock_data = SessionLockData {
            pid,
            start_timestamp,
            app_version: app_version.into(),
            clean_exit: false,
        };

        let json = serde_json::to_string_pretty(&lock_data).map_err(|e| {
            CoreError::Validation(format!("Failed to serialize session lock: {}", e))
        })?;

        let mut file = File::create(&self.lock_file)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    /// Performs an atomic autosave snapshot of serializable application state.
    pub fn record_autosave<T: Serialize>(&self, state: &T) -> CoreResult<()> {
        let _ = fs::create_dir_all(&self.work_dir);
        let temp_path = self.work_dir.join("autosave.json.tmp");

        let json = serde_json::to_string_pretty(state).map_err(|e| {
            CoreError::Validation(format!("Failed to serialize autosave state: {}", e))
        })?;

        {
            let mut file = File::create(&temp_path)?;
            file.write_all(json.as_bytes())?;
            file.flush()?;
        }

        // Atomic rename guarantees autosave.json is never partially written
        fs::rename(&temp_path, &self.autosave_file)?;
        Ok(())
    }

    /// Reads the raw autosave JSON contents for restoration.
    pub fn load_autosave(&self) -> CoreResult<String> {
        fs::read_to_string(&self.autosave_file).map_err(CoreError::Io)
    }

    /// Marks the session as cleanly exited and removes the lock file.
    pub fn release_session(&self) -> CoreResult<()> {
        if self.lock_file.exists() {
            let _ = fs::remove_file(&self.lock_file);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crash_recovery_lifecycle() {
        let temp_dir = tempfile::tempdir().unwrap();
        let manager = CrashRecoveryManager::new(temp_dir.path());

        // Initial state: no crash
        assert!(manager.check_recovery().is_none());

        // Acquire session
        manager.acquire_session(12345, "0.1.0").unwrap();

        // Save autosave state
        let dummy_state = serde_json::json!({
            "project_name": "Valorant Finals",
            "active_scene": "Live Match",
            "score": [12, 11]
        });
        manager.record_autosave(&dummy_state).unwrap();

        // If process dies without release_session:
        let recovery = manager
            .check_recovery()
            .expect("Expected crash recovery report");
        assert_eq!(recovery.session_info.pid, 12345);
        assert_eq!(recovery.session_info.app_version, "0.1.0");

        let loaded_json = manager.load_autosave().unwrap();
        let loaded_val: serde_json::Value = serde_json::from_str(&loaded_json).unwrap();
        assert_eq!(loaded_val["project_name"], "Valorant Finals");

        // Clean release
        manager.release_session().unwrap();
        assert!(manager.check_recovery().is_none());
    }
}
