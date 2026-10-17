//! Import audit report tracking imported layers, warnings, unsupported features,
//! and automatically generated data bindings.

use serde::{Deserialize, Serialize};

/// Detailed import summary displayed in the import report dialog.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportReport {
    /// Source file name or path.
    pub file_name: String,
    /// Importer engine used ("PSD" or "Lottie").
    pub importer: String,
    /// Whether importation completed successfully.
    pub success: bool,
    /// Total count of scene layers imported.
    pub imported_layers: usize,
    /// Group / hierarchy count.
    pub groups_count: usize,
    /// Warnings encountered during import.
    pub warnings: Vec<String>,
    /// Features unsupported in native engine that were rasterized or substituted.
    pub unsupported_features: Vec<String>,
    /// Auto-created bindings generated from `bind:` prefixed layer names.
    pub bindings_created: Vec<(String, String)>,
}

impl ImportReport {
    /// Creates a new empty import report.
    pub fn new(file_name: impl Into<String>, importer: impl Into<String>) -> Self {
        Self {
            file_name: file_name.into(),
            importer: importer.into(),
            success: true,
            imported_layers: 0,
            groups_count: 0,
            warnings: Vec::new(),
            unsupported_features: Vec::new(),
            bindings_created: Vec::new(),
        }
    }

    /// Adds a warning.
    pub fn add_warning(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    /// Records an unsupported feature.
    pub fn add_unsupported(&mut self, feature: impl Into<String>) {
        self.unsupported_features.push(feature.into());
    }

    /// Records an auto-generated data binding from a `bind:` layer name.
    pub fn add_binding(&mut self, layer_name: impl Into<String>, data_path: impl Into<String>) {
        self.bindings_created
            .push((layer_name.into(), data_path.into()));
    }

    /// Returns `true` if any warnings or unsupported features were logged.
    pub fn has_issues(&self) -> bool {
        !self.warnings.is_empty() || !self.unsupported_features.is_empty()
    }
}
