//! Thread-safe in-memory live reactive data store with dot-path querying,
//! recursive JSON flattening, fast CSV ingestion, and snapshot generation.

use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use zeno_engine::DataSnapshot;

use crate::error::DataError;
use crate::value::DataValue;

/// Shared change listener callback.
pub type ListenerCallback = Arc<dyn Fn(&str, &DataValue) + Send + Sync>;

/// In-memory reactive data store holding live key-value state for broadcasts.
#[derive(Default)]
pub struct DataStore {
    values: Arc<RwLock<HashMap<String, DataValue>>>,
    listeners: Arc<RwLock<Vec<ListenerCallback>>>,
}

impl std::fmt::Debug for DataStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let guard = self.values.read();
        f.debug_struct("DataStore")
            .field("values", &*guard)
            .field("listeners_count", &self.listeners.read().len())
            .finish()
    }
}

impl DataStore {
    /// Constructs a new empty data store.
    pub fn new() -> Self {
        Self {
            values: Arc::new(RwLock::new(HashMap::new())),
            listeners: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Sets or updates a single key in the store, notifying active listeners.
    pub fn set(&self, path: impl Into<String>, value: impl Into<DataValue>) {
        let path = path.into();
        let value = value.into();

        {
            let mut guard = self.values.write();
            guard.insert(path.clone(), value.clone());
        }

        let listeners = self.listeners.read();
        for listener in listeners.iter() {
            listener(&path, &value);
        }
    }

    /// Retrieves a cloned copy of the value at the given path.
    pub fn get(&self, path: &str) -> Option<DataValue> {
        let guard = self.values.read();
        guard.get(path).cloned()
    }

    /// Checks if a key exists in the data store.
    pub fn contains_key(&self, path: &str) -> bool {
        let guard = self.values.read();
        guard.contains_key(path)
    }

    /// Number of distinct paths currently in the store.
    pub fn len(&self) -> usize {
        let guard = self.values.read();
        guard.len()
    }

    /// Returns `true` if the store contains no entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Clears all stored values.
    pub fn clear(&self) {
        let mut guard = self.values.write();
        guard.clear();
    }

    /// High-throughput atomic batch update.
    /// Updates all entries while holding the write lock once, then fires listeners.
    pub fn batch_update(&self, entries: &[(String, DataValue)]) {
        if entries.is_empty() {
            return;
        }

        {
            let mut guard = self.values.write();
            for (k, v) in entries {
                guard.insert(k.clone(), v.clone());
            }
        }

        let listeners = self.listeners.read();
        if !listeners.is_empty() {
            for (k, v) in entries {
                for listener in listeners.iter() {
                    listener(k, v);
                }
            }
        }
    }

    /// Registers a change listener callback that triggers on any updated key.
    pub fn add_change_listener<F>(&self, callback: F)
    where
        F: Fn(&str, &DataValue) + Send + Sync + 'static,
    {
        let mut guard = self.listeners.write();
        guard.push(Arc::new(callback));
    }

    /// Ingests and recursively flattens a JSON string into dot-separated paths.
    /// Example: `{"scoreboard": {"team1": {"score": 42}}}` becomes `"scoreboard.team1.score" = 42`.
    pub fn load_json_str(&self, json_str: &str) -> Result<usize, DataError> {
        let parsed: serde_json::Value = serde_json::from_str(json_str)?;
        let mut flattened = Vec::new();

        fn flatten(prefix: &str, value: &serde_json::Value, out: &mut Vec<(String, DataValue)>) {
            match value {
                serde_json::Value::Object(map) => {
                    for (k, v) in map {
                        let new_key = if prefix.is_empty() {
                            k.clone()
                        } else {
                            format!("{}.{}", prefix, k)
                        };
                        flatten(&new_key, v, out);
                    }
                }
                serde_json::Value::Array(_)
                | serde_json::Value::String(_)
                | serde_json::Value::Number(_)
                | serde_json::Value::Bool(_) => {
                    if !prefix.is_empty() {
                        out.push((prefix.to_string(), DataValue::from_json(value)));
                    }
                }
                serde_json::Value::Null => {
                    if !prefix.is_empty() {
                        out.push((prefix.to_string(), DataValue::String(String::new())));
                    }
                }
            }
        }

        flatten("", &parsed, &mut flattened);
        let count = flattened.len();
        self.batch_update(&flattened);
        Ok(count)
    }

    /// Ingests a CSV string mapping key and value columns into the store.
    /// If `val_col` is None, stores entire row cells under `<key_val>.<header>`.
    pub fn load_csv_str(
        &self,
        csv_str: &str,
        key_col: &str,
        val_col: Option<&str>,
    ) -> Result<usize, DataError> {
        let lines: Vec<&str> = csv_str
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        if lines.is_empty() {
            return Ok(0);
        }

        let headers: Vec<&str> = lines[0].split(',').map(|s| s.trim()).collect();
        let key_idx = headers
            .iter()
            .position(|&h| h == key_col)
            .ok_or_else(|| DataError::Csv(format!("Missing key column '{}'", key_col)))?;

        let val_idx = if let Some(vc) = val_col {
            let idx = headers
                .iter()
                .position(|&h| h == vc)
                .ok_or_else(|| DataError::Csv(format!("Missing value column '{}'", vc)))?;
            Some(idx)
        } else {
            None
        };

        let mut entries = Vec::new();
        for line in lines.iter().skip(1) {
            let cells: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
            if cells.len() <= key_idx {
                continue;
            }

            let key_val = cells[key_idx];
            if let Some(v_idx) = val_idx {
                if cells.len() > v_idx {
                    let cell_str = cells[v_idx];
                    let val = if let Ok(i) = cell_str.parse::<i64>() {
                        DataValue::Integer(i)
                    } else if let Ok(f) = cell_str.parse::<f64>() {
                        DataValue::Float(f)
                    } else if let Ok(b) = cell_str.parse::<bool>() {
                        DataValue::Boolean(b)
                    } else {
                        DataValue::String(cell_str.to_string())
                    };
                    entries.push((key_val.to_string(), val));
                }
            } else {
                // Flatten all columns under key_val.<header>
                for (i, &hdr) in headers.iter().enumerate() {
                    if i != key_idx && i < cells.len() {
                        let cell_str = cells[i];
                        let val = if let Ok(num) = cell_str.parse::<i64>() {
                            DataValue::Integer(num)
                        } else if let Ok(f) = cell_str.parse::<f64>() {
                            DataValue::Float(f)
                        } else if let Ok(b) = cell_str.parse::<bool>() {
                            DataValue::Boolean(b)
                        } else {
                            DataValue::String(cell_str.to_string())
                        };
                        entries.push((format!("{}.{}", key_val, hdr), val));
                    }
                }
            }
        }

        let count = entries.len();
        self.batch_update(&entries);
        Ok(count)
    }

    /// Produces a `DataSnapshot` for pure per-frame evaluation passes in `zeno-engine`.
    pub fn to_snapshot(&self) -> DataSnapshot {
        let guard = self.values.read();
        let mut snapshot = DataSnapshot::new();
        for (k, v) in guard.iter() {
            snapshot.set(k.clone(), v.to_json());
        }
        snapshot
    }
}
