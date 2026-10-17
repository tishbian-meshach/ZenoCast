//! Spreadsheet synchronization engine with change detection, source tagging,
//! locked file resilience, and data store population.

use std::collections::{HashMap, HashSet};
use std::time::SystemTime;

use tracing::{debug, warn};

use crate::error::SheetError;
use crate::sheet::model::{CellTarget, SheetCoordinate, SheetLink, SheetLinkStatus, VariableKind};
use crate::sheet::reader::{SpreadsheetReader, SpreadsheetTable};
use crate::store::DataStore;
use crate::value::DataValue;

/// Central synchronization engine driving spreadsheet poll cycles and change detection.
#[derive(Debug, Default)]
pub struct SheetSyncEngine {
    links: Vec<SheetLink>,
    mock_registry: HashMap<String, SpreadsheetTable>,
    locked_paths: HashSet<String>,
}

impl SheetSyncEngine {
    /// Creates a new empty synchronization engine.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a spreadsheet link to be monitored.
    pub fn add_link(&mut self, link: SheetLink) -> usize {
        let idx = self.links.len();
        for b in &link.bindings {
            if b.locked_to_sheet {
                self.locked_paths.insert(b.data_path.clone());
            }
        }
        self.links.push(link);
        idx
    }

    /// Returns a slice of active links.
    pub fn links(&self) -> &[SheetLink] {
        &self.links
    }

    /// Returns a mutable slice of active links.
    pub fn links_mut(&mut self) -> &mut [SheetLink] {
        &mut self.links
    }

    /// Registers a mock table for offline testing of Google Sheets IDs or published CSV URLs.
    pub fn register_mock_table(&mut self, id_or_url: impl Into<String>, table: SpreadsheetTable) {
        self.mock_registry.insert(id_or_url.into(), table);
    }

    /// Checks if a path is marked "Locked to Sheet" (blocking manual operator edits).
    pub fn is_path_locked(&self, path: &str) -> bool {
        self.locked_paths.contains(path)
    }

    /// Sets whether a path is locked to spreadsheet writes.
    pub fn set_path_locked(&mut self, path: impl Into<String>, locked: bool) {
        let p = path.into();
        if locked {
            self.locked_paths.insert(p);
        } else {
            self.locked_paths.remove(&p);
        }
    }

    /// Synchronizes all registered links against the data store.
    pub fn sync_all(&mut self, store: &DataStore) {
        for i in 0..self.links.len() {
            let _ = self.sync_link(i, store);
        }
    }

    /// Synchronizes a specific link index against the data store.
    /// Returns the number of changed values written.
    pub fn sync_link(&mut self, index: usize, store: &DataStore) -> Result<usize, SheetError> {
        if index >= self.links.len() {
            return Ok(0);
        }

        let link = &self.links[index];
        let read_res = SpreadsheetReader::read_source(
            &link.source,
            Some(&link.sheet_name),
            Some(&self.mock_registry),
        );

        let table = match read_res {
            Ok(t) => t,
            Err(e) => {
                let link_mut = &mut self.links[index];
                match &e {
                    SheetError::LockFileIgnored(_) => {
                        // Silent ignore
                        return Ok(0);
                    }
                    SheetError::FileLocked(p) => {
                        let msg = format!(
                            "File '{}' is open/locked by Excel. Retaining last values.",
                            p.display()
                        );
                        warn!("{}", msg);
                        link_mut.status = SheetLinkStatus::Warning(msg.clone());
                        link_mut.last_error = Some(msg);
                        return Err(e);
                    }
                    SheetError::FileNotFound(p) => {
                        let msg =
                            format!("File '{}' not found. Retaining last values.", p.display());
                        warn!("{}", msg);
                        link_mut.status = SheetLinkStatus::Warning(msg.clone());
                        link_mut.last_error = Some(msg);
                        return Err(e);
                    }
                    _ => {
                        let msg = e.to_string();
                        warn!("Spreadsheet sync error: {}", msg);
                        link_mut.status = SheetLinkStatus::Error(msg.clone());
                        link_mut.last_error = Some(msg);
                        return Err(e);
                    }
                }
            }
        };

        // Table read successfully: clear error badges and update sync timestamp
        let link_mut = &mut self.links[index];
        link_mut.status = SheetLinkStatus::Ok;
        link_mut.last_error = None;
        link_mut.last_sync = Some(SystemTime::now());

        let mut changed_count = 0;
        let bindings = link_mut.bindings.clone();

        for binding in bindings {
            if binding.locked_to_sheet {
                self.locked_paths.insert(binding.data_path.clone());
            }

            match &binding.target {
                CellTarget::Cell(address) => {
                    if let Some(coord) = SheetCoordinate::parse(address) {
                        if let Some(raw_val) = table.get_cell(coord) {
                            let typed_val = coerce_variable_kind(raw_val, binding.value_kind);
                            let current_val = store.get(&binding.data_path);
                            if current_val.as_ref() != Some(&typed_val) {
                                store.set(&binding.data_path, typed_val);
                                changed_count += 1;
                                debug!(
                                    "Spreadsheet updated '{}' to {:?}",
                                    binding.data_path,
                                    store.get(&binding.data_path)
                                );
                            }
                        }
                    }
                }
                CellTarget::Range(range_str) => {
                    if let Some((start, end)) = SheetCoordinate::parse_range(range_str) {
                        let subgrid = table.get_range(start, end);
                        let row_count = subgrid.len();

                        // 1. Store entire table as DataValue::Table
                        let new_table_val = DataValue::Table(subgrid.clone());
                        if store.get(&binding.data_path).as_ref() != Some(&new_table_val) {
                            store.set(&binding.data_path, new_table_val);
                            changed_count += 1;
                        }

                        // 2. Store row-level and cell-level subpaths for direct bindings
                        for (r_idx, row) in subgrid.iter().enumerate() {
                            let row_key = format!("{}[{}]", binding.data_path, r_idx);
                            store.set(&row_key, DataValue::Table(vec![row.clone()]));

                            for (c_idx, cell) in row.iter().enumerate() {
                                let cell_key =
                                    format!("{}[{}][{}]", binding.data_path, r_idx, c_idx);
                                store.set(&cell_key, cell.clone());
                            }
                        }

                        // 3. If first row looks like string headers, map named object attributes
                        if row_count > 1 {
                            let headers: Vec<String> = subgrid[0]
                                .iter()
                                .map(|c| c.to_display_string().to_lowercase().replace(' ', "_"))
                                .collect();

                            for (r_idx, data_row) in subgrid.iter().enumerate().skip(1) {
                                for (c_idx, header) in headers.iter().enumerate() {
                                    if !header.is_empty() {
                                        if let Some(cell) = data_row.get(c_idx) {
                                            let named_key = format!(
                                                "{}[{}].{}",
                                                binding.data_path,
                                                r_idx - 1,
                                                header
                                            );
                                            store.set(&named_key, cell.clone());
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                CellTarget::NamedRange(_name) => {
                    // Fallback or named range lookup if defined
                }
            }
        }

        Ok(changed_count)
    }
}

/// Coerces a raw spreadsheet cell value to the expected target `VariableKind`.
fn coerce_variable_kind(val: &DataValue, kind: VariableKind) -> DataValue {
    match kind {
        VariableKind::Number => match val {
            DataValue::Integer(i) => DataValue::Integer(*i),
            DataValue::Float(f) => DataValue::Float(*f),
            _ => {
                if let Some(i) = val.as_i64() {
                    DataValue::Integer(i)
                } else if let Some(f) = val.as_f64() {
                    DataValue::Float(f)
                } else {
                    val.clone()
                }
            }
        },
        VariableKind::Text | VariableKind::ImagePath | VariableKind::Color => {
            DataValue::String(val.to_display_string())
        }
        VariableKind::Boolean => {
            if let Some(b) = val.as_bool() {
                DataValue::Boolean(b)
            } else {
                val.clone()
            }
        }
    }
}
