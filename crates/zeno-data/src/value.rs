//! Strongly typed data values stored in the live reactive data store.

use serde::{Deserialize, Serialize};

/// Concrete data value stored in the data store.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DataValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    ImagePath(String),
    Table(Vec<Vec<DataValue>>),
}

impl DataValue {
    /// Converts this value into a human-readable display string.
    pub fn to_display_string(&self) -> String {
        match self {
            Self::String(s) => s.clone(),
            Self::Integer(i) => i.to_string(),
            Self::Float(f) => format!("{:.2}", f),
            Self::Boolean(b) => b.to_string(),
            Self::ImagePath(p) => p.clone(),
            Self::Table(rows) => format!("[Table: {} rows]", rows.len()),
        }
    }

    /// Attempts to extract or coerce this value into an `f64`.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Integer(i) => Some(*i as f64),
            Self::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
            Self::String(s) => s.parse::<f64>().ok(),
            _ => None,
        }
    }

    /// Attempts to extract or coerce this value into an `i64`.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            Self::Float(f) => Some(*f as i64),
            Self::Boolean(b) => Some(if *b { 1 } else { 0 }),
            Self::String(s) => s.parse::<i64>().ok(),
            _ => None,
        }
    }

    /// Attempts to extract or coerce this value into a `bool`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            Self::Integer(i) => Some(*i != 0),
            Self::Float(f) => Some(*f != 0.0),
            Self::String(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" => Some(true),
                "false" | "0" | "no" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    /// Converts this value to a `serde_json::Value`.
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Self::String(s) => serde_json::Value::String(s.clone()),
            Self::Integer(i) => serde_json::Value::Number((*i).into()),
            Self::Float(f) => serde_json::Number::from_f64(*f)
                .map(serde_json::Value::Number)
                .unwrap_or(serde_json::Value::Null),
            Self::Boolean(b) => serde_json::Value::Bool(*b),
            Self::ImagePath(p) => serde_json::Value::String(p.clone()),
            Self::Table(rows) => {
                let json_rows: Vec<serde_json::Value> = rows
                    .iter()
                    .map(|r| serde_json::Value::Array(r.iter().map(|c| c.to_json()).collect()))
                    .collect();
                serde_json::Value::Array(json_rows)
            }
        }
    }

    /// Constructs a `DataValue` from a `serde_json::Value`.
    pub fn from_json(val: &serde_json::Value) -> Self {
        match val {
            serde_json::Value::String(s) => Self::String(s.clone()),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Self::Integer(i)
                } else if let Some(f) = n.as_f64() {
                    Self::Float(f)
                } else {
                    Self::Integer(0)
                }
            }
            serde_json::Value::Bool(b) => Self::Boolean(*b),
            serde_json::Value::Array(arr) => {
                let rows = arr
                    .iter()
                    .filter_map(|item| {
                        if let serde_json::Value::Array(row) = item {
                            Some(row.iter().map(Self::from_json).collect())
                        } else {
                            None
                        }
                    })
                    .collect();
                Self::Table(rows)
            }
            serde_json::Value::Null => Self::String(String::new()),
            serde_json::Value::Object(_) => Self::String(val.to_string()),
        }
    }
}

impl From<&str> for DataValue {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<String> for DataValue {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<i32> for DataValue {
    fn from(i: i32) -> Self {
        Self::Integer(i as i64)
    }
}

impl From<i64> for DataValue {
    fn from(i: i64) -> Self {
        Self::Integer(i)
    }
}

impl From<f32> for DataValue {
    fn from(f: f32) -> Self {
        Self::Float(f as f64)
    }
}

impl From<f64> for DataValue {
    fn from(f: f64) -> Self {
        Self::Float(f)
    }
}

impl From<bool> for DataValue {
    fn from(b: bool) -> Self {
        Self::Boolean(b)
    }
}
