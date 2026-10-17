//! Spreadsheet link models, cell coordinates, targets, and synchronization bindings.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use zeno_core::LinkId;

/// Expected variable data type for spreadsheet values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VariableKind {
    Number,
    Text,
    Color,
    ImagePath,
    Boolean,
}

impl fmt::Display for VariableKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number => write!(f, "Number"),
            Self::Text => write!(f, "Text"),
            Self::Color => write!(f, "Color"),
            Self::ImagePath => write!(f, "ImagePath"),
            Self::Boolean => write!(f, "Boolean"),
        }
    }
}

/// Direction of spreadsheet synchronization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum LinkDirection {
    #[default]
    SheetToApp,
    AppToSheet,
    TwoWay,
}

/// Target within a spreadsheet: individual cell, named range, or multi-cell bounding range.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CellTarget {
    /// Single cell by coordinate address, e.g. "B3" or "A1".
    Cell(String),
    /// Named range defined in the workbook.
    NamedRange(String),
    /// Multi-cell rectangular range, e.g. "A1:D10".
    Range(String),
}

impl fmt::Display for CellTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cell(addr) => write!(f, "Cell({})", addr),
            Self::NamedRange(name) => write!(f, "NamedRange({})", name),
            Self::Range(rng) => write!(f, "Range({})", rng),
        }
    }
}

/// Source type and location of the spreadsheet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SheetSource {
    /// Local file on disk (.xlsx, .xlsm, .xls, .ods, .csv).
    LocalFile { path: PathBuf },
    /// Remote Google Spreadsheet.
    GoogleSheet { spreadsheet_id: String },
    /// Published CSV endpoint over HTTP/HTTPS.
    CsvUrl { url: String },
}

/// Single binding mapping a spreadsheet target to a data store path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellBinding {
    /// Target path in DataStore, e.g. "scoreboard.team1.score".
    pub data_path: String,
    /// Spreadsheet target location (Cell, NamedRange, Range).
    pub target: CellTarget,
    /// Expected variable data type.
    pub value_kind: VariableKind,
    /// Data synchronization direction.
    pub direction: LinkDirection,
    /// Optional expression or transform string.
    pub transform: Option<String>,
    /// When true, manual UI edits to this data path are blocked.
    pub locked_to_sheet: bool,
}

impl CellBinding {
    /// Creates a new default Sheet-to-App cell binding.
    pub fn new(data_path: impl Into<String>, target: CellTarget, value_kind: VariableKind) -> Self {
        Self {
            data_path: data_path.into(),
            target,
            value_kind,
            direction: LinkDirection::SheetToApp,
            transform: None,
            locked_to_sheet: false,
        }
    }

    /// Marks this binding as locked to sheet.
    pub fn with_locked(mut self, locked: bool) -> Self {
        self.locked_to_sheet = locked;
        self
    }
}

/// Health and sync status of a SheetLink.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SheetLinkStatus {
    Ok,
    Warning(String),
    Error(String),
}

/// Active spreadsheet link configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SheetLink {
    pub id: LinkId,
    pub source: SheetSource,
    pub sheet_name: String,
    pub bindings: Vec<CellBinding>,
    pub poll_interval_ms: u32,
    pub status: SheetLinkStatus,
    pub last_sync: Option<std::time::SystemTime>,
    pub last_error: Option<String>,
}

impl SheetLink {
    /// Creates a new sheet link.
    pub fn new(source: SheetSource, sheet_name: impl Into<String>) -> Self {
        Self {
            id: LinkId::new(),
            source,
            sheet_name: sheet_name.into(),
            bindings: Vec::new(),
            poll_interval_ms: 2000,
            status: SheetLinkStatus::Ok,
            last_sync: None,
            last_error: None,
        }
    }

    /// Adds a cell binding.
    pub fn add_binding(&mut self, binding: CellBinding) {
        self.bindings.push(binding);
    }
}

/// 0-indexed column and row coordinate in a spreadsheet grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SheetCoordinate {
    pub col: u32, // 0-based: A=0, B=1, ...
    pub row: u32, // 0-based: 1=0, 2=1, 3=2, ...
}

impl SheetCoordinate {
    /// Parses an alphanumeric A1 address like "B3" or "AA12".
    pub fn parse(address: &str) -> Option<Self> {
        let trimmed = address.trim();
        let letters: String = trimmed
            .chars()
            .take_while(|c| c.is_ascii_alphabetic())
            .collect();
        let digits: String = trimmed
            .chars()
            .skip_while(|c| c.is_ascii_alphabetic())
            .collect();
        if letters.is_empty() || digits.is_empty() {
            return None;
        }

        let mut col: u32 = 0;
        for ch in letters.to_ascii_uppercase().chars() {
            if !ch.is_ascii_uppercase() {
                return None;
            }
            col = col
                .checked_mul(26)?
                .checked_add(ch as u32 - 'A' as u32 + 1)?;
        }
        if col == 0 {
            return None;
        }
        let col = col - 1;

        let row: u32 = digits.parse().ok()?;
        if row == 0 {
            return None;
        }
        let row = row - 1;

        Some(Self { col, row })
    }

    /// Converts back to standard A1-notation address string.
    pub fn to_address(&self) -> String {
        let mut n = self.col + 1;
        let mut letters = String::new();
        while n > 0 {
            let rem = (n - 1) % 26;
            letters.insert(0, (b'A' + rem as u8) as char);
            n = (n - 1) / 26;
        }
        format!("{}{}", letters, self.row + 1)
    }

    /// Parses a bounding range like "A1:D10".
    pub fn parse_range(range_str: &str) -> Option<(Self, Self)> {
        let parts: Vec<&str> = range_str.split(':').collect();
        if parts.len() != 2 {
            return None;
        }
        let start = Self::parse(parts[0])?;
        let end = Self::parse(parts[1])?;
        Some((
            Self {
                col: start.col.min(end.col),
                row: start.row.min(end.row),
            },
            Self {
                col: start.col.max(end.col),
                row: start.row.max(end.row),
            },
        ))
    }
}
