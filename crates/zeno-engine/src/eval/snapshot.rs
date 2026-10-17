//! In-memory live variable snapshot for dynamic property data-binding evaluation.

use std::collections::HashMap;

/// Live variable key-value store snapshot passed into pure evaluation passes.
#[derive(Debug, Clone, Default)]
pub struct DataSnapshot {
    /// Mapping of JSON dot-separated paths (e.g. "scoreboard.team1.score") to JSON values.
    pub values: HashMap<String, serde_json::Value>,
}

impl DataSnapshot {
    /// Creates an empty data snapshot.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets or updates a variable path.
    pub fn set(&mut self, path: impl Into<String>, value: impl Into<serde_json::Value>) {
        self.values.insert(path.into(), value.into());
    }

    /// Retrieves a variable value reference by path.
    pub fn get(&self, path: &str) -> Option<&serde_json::Value> {
        self.values.get(path)
    }

    /// Checks if a variable path is present.
    pub fn contains(&self, path: &str) -> bool {
        self.values.contains_key(path)
    }
}
