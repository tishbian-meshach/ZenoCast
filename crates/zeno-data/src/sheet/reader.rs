//! Spreadsheet file readers for CSV, Excel (.xlsx, .xlsm), and published sources.
//!
//! Includes locked-file mitigation (temporary copy and retry), `~$` ignore rules,
//! OpenXML parsing via `zip`, and cell coordinate matrix extraction.

use std::collections::HashMap;
use std::io::{Cursor, Read};
use std::path::Path;
use std::thread;
use std::time::Duration;

use crate::error::SheetError;
use crate::sheet::model::{SheetCoordinate, SheetSource};
use crate::value::DataValue;

/// 2D evaluated matrix of spreadsheet cells.
#[derive(Debug, Clone, PartialEq)]
pub struct SpreadsheetTable {
    pub sheet_name: String,
    pub rows: Vec<Vec<DataValue>>,
}

impl SpreadsheetTable {
    /// Creates a new table with the given sheet name and rows.
    pub fn new(sheet_name: impl Into<String>, rows: Vec<Vec<DataValue>>) -> Self {
        Self {
            sheet_name: sheet_name.into(),
            rows,
        }
    }

    /// Number of rows in the table.
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Maximum number of columns across all rows.
    pub fn col_count(&self) -> usize {
        self.rows.iter().map(|r| r.len()).max().unwrap_or(0)
    }

    /// Retrieves cell value at given 0-indexed coordinate.
    pub fn get_cell(&self, coord: SheetCoordinate) -> Option<&DataValue> {
        self.rows
            .get(coord.row as usize)
            .and_then(|row| row.get(coord.col as usize))
    }

    /// Extracts a subgrid between `start` and `end` coordinates inclusive.
    pub fn get_range(&self, start: SheetCoordinate, end: SheetCoordinate) -> Vec<Vec<DataValue>> {
        let min_r = start.row.min(end.row) as usize;
        let max_r = start.row.max(end.row) as usize;
        let min_c = start.col.min(end.col) as usize;
        let max_c = start.col.max(end.col) as usize;

        let mut result = Vec::new();
        for r in min_r..=max_r {
            let mut row_vals = Vec::new();
            if let Some(row) = self.rows.get(r) {
                for c in min_c..=max_c {
                    if let Some(val) = row.get(c) {
                        row_vals.push(val.clone());
                    } else {
                        row_vals.push(DataValue::String(String::new()));
                    }
                }
            } else {
                for _ in min_c..=max_c {
                    row_vals.push(DataValue::String(String::new()));
                }
            }
            result.push(row_vals);
        }
        result
    }
}

/// Standalone spreadsheet file and stream reader.
pub struct SpreadsheetReader;

impl SpreadsheetReader {
    /// Reads a spreadsheet from a `SheetSource` with locked-file retry and `~$` ignore rules.
    pub fn read_source(
        source: &SheetSource,
        sheet_name: Option<&str>,
        mock_registry: Option<&HashMap<String, SpreadsheetTable>>,
    ) -> Result<SpreadsheetTable, SheetError> {
        match source {
            SheetSource::LocalFile { path } => Self::read_file(path, sheet_name),
            SheetSource::GoogleSheet { spreadsheet_id } => {
                if let Some(reg) = mock_registry {
                    if let Some(table) = reg.get(spreadsheet_id) {
                        return Ok(table.clone());
                    }
                }
                Err(SheetError::Network(format!(
                    "Google Sheets ID '{}' not found in registry",
                    spreadsheet_id
                )))
            }
            SheetSource::CsvUrl { url } => {
                if let Some(reg) = mock_registry {
                    if let Some(table) = reg.get(url) {
                        return Ok(table.clone());
                    }
                }
                Err(SheetError::Network(format!(
                    "Published CSV URL '{}' not found in registry",
                    url
                )))
            }
        }
    }

    /// Reads a local spreadsheet file (.xlsx, .xlsm, .csv, .tsv) with locked-file handling.
    pub fn read_file(
        path: &Path,
        sheet_name: Option<&str>,
    ) -> Result<SpreadsheetTable, SheetError> {
        // 1. Ignore Excel lock files prefix "~$"
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.starts_with("~$") {
                return Err(SheetError::LockFileIgnored(path.to_path_buf()));
            }
        }

        // 2. Check existence
        if !path.exists() {
            return Err(SheetError::FileNotFound(path.to_path_buf()));
        }

        // 3. Retry loop for open/locked files (Excel locks with sharing restrictions)
        let mut attempts = 0;
        let bytes = loop {
            attempts += 1;
            match std::fs::read(path) {
                Ok(b) => break b,
                Err(_e) => {
                    if attempts >= 3 {
                        return Err(SheetError::FileLocked(path.to_path_buf()));
                    }
                    // Wait briefly before retrying
                    thread::sleep(Duration::from_millis(20));
                }
            }
        };

        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        match ext.as_str() {
            "xlsx" | "xlsm" => Self::read_xlsx_bytes(&bytes, sheet_name),
            "csv" | "tsv" | "txt" => {
                let text = String::from_utf8_lossy(&bytes);
                Self::read_csv_str(&text, sheet_name.unwrap_or("Sheet1"))
            }
            _ => {
                // Try reading as CSV by default
                let text = String::from_utf8_lossy(&bytes);
                Self::read_csv_str(&text, sheet_name.unwrap_or("Sheet1"))
            }
        }
    }

    /// Ingests and parses a CSV or TSV string into a `SpreadsheetTable`.
    pub fn read_csv_str(csv_str: &str, sheet_name: &str) -> Result<SpreadsheetTable, SheetError> {
        let lines: Vec<&str> = csv_str.lines().collect();
        if lines.is_empty() {
            return Ok(SpreadsheetTable::new(sheet_name, Vec::new()));
        }

        // Detect delimiter: comma, semicolon, tab
        let first_line = lines[0];
        let delimiter = if first_line.contains('\t') {
            '\t'
        } else if first_line.matches(';').count() > first_line.matches(',').count() {
            ';'
        } else {
            ','
        };

        let mut rows = Vec::new();
        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let fields = parse_delimited_line(trimmed, delimiter);
            let row_vals: Vec<DataValue> = fields.into_iter().map(infer_data_value).collect();
            rows.push(row_vals);
        }

        Ok(SpreadsheetTable::new(sheet_name, rows))
    }

    /// Ingests and parses an OpenXML Excel `.xlsx` package from raw bytes.
    pub fn read_xlsx_bytes(
        bytes: &[u8],
        target_sheet: Option<&str>,
    ) -> Result<SpreadsheetTable, SheetError> {
        let cursor = Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor)
            .map_err(|e| SheetError::Format(format!("Failed to open xlsx zip archive: {}", e)))?;

        // 1. Read shared strings if present
        let mut shared_strings = Vec::new();
        if let Ok(mut ss_file) = archive.by_name("xl/sharedStrings.xml") {
            let mut ss_xml = String::new();
            if ss_file.read_to_string(&mut ss_xml).is_ok() {
                shared_strings = extract_shared_strings(&ss_xml);
            }
        }

        // 2. Locate worksheet file: default to sheet1.xml or resolve via workbook.xml
        let sheet_file_name = if let Some(target) = target_sheet {
            find_sheet_filename(&mut archive, target)
                .unwrap_or_else(|| "xl/worksheets/sheet1.xml".to_string())
        } else {
            "xl/worksheets/sheet1.xml".to_string()
        };

        let mut ws_file = archive
            .by_name(&sheet_file_name)
            .map_err(|_| SheetError::SheetNotFound(target_sheet.unwrap_or("Sheet1").to_string()))?;

        let mut ws_xml = String::new();
        ws_file
            .read_to_string(&mut ws_xml)
            .map_err(|e| SheetError::Format(format!("Failed to read worksheet xml: {}", e)))?;

        // 3. Parse cells from sheet XML
        let cells = parse_sheet_cells(&ws_xml, &shared_strings);

        // 4. Construct rows matrix
        let mut max_row = 0;
        let mut max_col = 0;
        for &(c, r) in cells.keys() {
            if r > max_row {
                max_row = r;
            }
            if c > max_col {
                max_col = c;
            }
        }

        let num_rows = if cells.is_empty() {
            0
        } else {
            max_row as usize + 1
        };
        let num_cols = if cells.is_empty() {
            0
        } else {
            max_col as usize + 1
        };

        let mut rows = vec![vec![DataValue::String(String::new()); num_cols]; num_rows];
        for ((c, r), val) in cells {
            rows[r as usize][c as usize] = val;
        }

        Ok(SpreadsheetTable::new(
            target_sheet.unwrap_or("Sheet1"),
            rows,
        ))
    }
}

/// Parses a delimited line taking quotes into account.
fn parse_delimited_line(line: &str, delimiter: char) -> Vec<String> {
    let mut fields = Vec::new();
    let mut cur = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '"' {
            if in_quotes && chars.peek() == Some(&'"') {
                cur.push('"');
                chars.next();
            } else {
                in_quotes = !in_quotes;
            }
        } else if ch == delimiter && !in_quotes {
            fields.push(cur.trim().to_string());
            cur.clear();
        } else {
            cur.push(ch);
        }
    }
    fields.push(cur.trim().to_string());
    fields
}

/// Infers the typed `DataValue` from raw cell string.
fn infer_data_value(s: String) -> DataValue {
    let trimmed = s.trim();
    if trimmed.eq_ignore_ascii_case("true") {
        DataValue::Boolean(true)
    } else if trimmed.eq_ignore_ascii_case("false") {
        DataValue::Boolean(false)
    } else if let Ok(i) = trimmed.parse::<i64>() {
        DataValue::Integer(i)
    } else if let Ok(f) = trimmed.parse::<f64>() {
        DataValue::Float(f)
    } else {
        DataValue::String(s)
    }
}

/// Extracts strings from `xl/sharedStrings.xml`.
fn extract_shared_strings(xml: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut cur = xml;

    while let Some(si_start) = cur.find("<si") {
        cur = &cur[si_start..];
        let si_end = match cur.find("</si>") {
            Some(e) => e + 5,
            None => break,
        };
        let si_slice = &cur[..si_end];
        cur = &cur[si_end..];

        // Inside <si>, find all <t>...</t> tags and concatenate
        let mut text = String::new();
        let mut t_cur = si_slice;
        while let Some(t_open) = t_cur.find("<t") {
            let after_open = &t_cur[t_open..];
            let tag_close = match after_open.find('>') {
                Some(idx) => idx + 1,
                None => break,
            };
            let content_start = &after_open[tag_close..];
            let content_end = match content_start.find("</t>") {
                Some(idx) => idx,
                None => break,
            };
            text.push_str(&unescape_xml(&content_start[..content_end]));
            t_cur = &content_start[content_end + 4..];
        }
        result.push(text);
    }

    result
}

/// Simple XML unescaper.
fn unescape_xml(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
}

/// Resolves a sheet name to its XML filename in the zip archive.
fn find_sheet_filename(
    archive: &mut zip::ZipArchive<Cursor<&[u8]>>,
    sheet_name: &str,
) -> Option<String> {
    // 1. Check workbook.xml and extract r:id
    let r_id = {
        let mut wb_file = archive.by_name("xl/workbook.xml").ok()?;
        let mut wb_xml = String::new();
        wb_file.read_to_string(&mut wb_xml).ok()?;
        let pattern = format!("name=\"{}\"", sheet_name);
        let pos = wb_xml.find(&pattern)?;
        let slice = &wb_xml[pos..];
        let rid_pos = slice.find("r:id=\"")?;
        let rid_slice = &slice[rid_pos + 6..];
        let end_quote = rid_slice.find('"')?;
        rid_slice[..end_quote].to_string()
    };

    // 2. Map r_id via xl/_rels/workbook.xml.rels
    let mut rels_file = archive.by_name("xl/_rels/workbook.xml.rels").ok()?;
    let mut rels_xml = String::new();
    rels_file.read_to_string(&mut rels_xml).ok()?;
    let id_pat = format!("Id=\"{}\"", r_id);
    let id_idx = rels_xml.find(&id_pat)?;
    let rel_slice = &rels_xml[id_idx..];
    let target_idx = rel_slice.find("Target=\"")?;
    let target_slice = &rel_slice[target_idx + 8..];
    let t_end = target_slice.find('"')?;
    let target = &target_slice[..t_end];
    let clean = target.trim_start_matches('/');
    Some(if clean.starts_with("xl/") {
        clean.to_string()
    } else {
        format!("xl/{}", clean)
    })
}

/// Parses `<c r="B3" ...>` cell elements in worksheet XML.
fn parse_sheet_cells(xml: &str, shared_strings: &[String]) -> HashMap<(u32, u32), DataValue> {
    let mut cells = HashMap::new();
    let mut cur = xml;

    while let Some(c_start) = cur.find("<c ") {
        cur = &cur[c_start..];
        let tag_close = match cur.find('>') {
            Some(idx) => idx,
            None => break,
        };
        let tag_header = &cur[..tag_close];

        // Extract coordinate `r="B3"`
        let coord = if let Some(r_idx) = tag_header.find("r=\"") {
            let r_slice = &tag_header[r_idx + 3..];
            if let Some(end_quote) = r_slice.find('"') {
                SheetCoordinate::parse(&r_slice[..end_quote])
            } else {
                None
            }
        } else {
            None
        };

        // Extract type `t="s"` (shared string), `t="b"` (bool)
        let is_shared_string = tag_header.contains("t=\"s\"");
        let is_boolean = tag_header.contains("t=\"b\"");

        // Determine if self-closing `<c .../>`
        if tag_header.ends_with('/') {
            cur = &cur[tag_close + 1..];
            continue;
        }

        // Find cell close `</c>`
        let c_end = match cur.find("</c>") {
            Some(idx) => idx,
            None => break,
        };
        let cell_body = &cur[tag_close + 1..c_end];
        cur = &cur[c_end + 4..];

        if let Some(c) = coord {
            // Extract value `<v>...</v>`
            let raw_val = if let Some(v_start) = cell_body.find("<v>") {
                let v_slice = &cell_body[v_start + 3..];
                v_slice.find("</v>").map(|v_end| &v_slice[..v_end])
            } else if let Some(t_start) = cell_body.find("<t>") {
                // Inline string <is><t>...</t></is>
                let t_slice = &cell_body[t_start + 3..];
                t_slice.find("</t>").map(|t_end| &t_slice[..t_end])
            } else {
                None
            };

            if let Some(v_str) = raw_val {
                let val = if is_shared_string {
                    if let Ok(idx) = v_str.trim().parse::<usize>() {
                        if let Some(s) = shared_strings.get(idx) {
                            DataValue::String(s.clone())
                        } else {
                            DataValue::String(String::new())
                        }
                    } else {
                        DataValue::String(v_str.to_string())
                    }
                } else if is_boolean {
                    DataValue::Boolean(v_str.trim() == "1")
                } else {
                    infer_data_value(v_str.to_string())
                };

                cells.insert((c.col, c.row), val);
            }
        }
    }

    cells
}
