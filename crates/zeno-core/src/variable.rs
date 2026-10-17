//! Variable schemas, typed values, and template schema declarations.

use crate::Color;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Primitive data types supported by the live data store and template variables.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum VariableKind {
    /// String / UTF-8 text (e.g. player names, team tags).
    Text,
    /// 64-bit floating point number or integer (e.g. scores, rounds).
    Number,
    /// Color value.
    Color,
    /// Image asset identifier or asset path.
    Image,
    /// Boolean flag (e.g. visibility toggles, match point indicator).
    Boolean,
}

impl fmt::Display for VariableKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => write!(f, "Text"),
            Self::Number => write!(f, "Number"),
            Self::Color => write!(f, "Color"),
            Self::Image => write!(f, "Image"),
            Self::Boolean => write!(f, "Boolean"),
        }
    }
}

/// Dynamic typed value stored in a variable or data node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum VariableValue {
    /// Text value.
    Text(String),
    /// Numeric value.
    Number(f64),
    /// Color value.
    Color(Color),
    /// Image path or asset id string.
    Image(String),
    /// Boolean state.
    Boolean(bool),
}

impl VariableValue {
    /// Returns the kind corresponding to this value.
    pub fn kind(&self) -> VariableKind {
        match self {
            Self::Text(_) => VariableKind::Text,
            Self::Number(_) => VariableKind::Number,
            Self::Color(_) => VariableKind::Color,
            Self::Image(_) => VariableKind::Image,
            Self::Boolean(_) => VariableKind::Boolean,
        }
    }

    /// Attempt to interpret value as string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Text(s) | Self::Image(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// Attempt to interpret value as f64.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Self::Number(n) => Some(*n),
            Self::Text(s) => s.parse::<f64>().ok(),
            _ => None,
        }
    }

    /// Attempt to interpret value as boolean.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            Self::Number(n) => Some(*n != 0.0),
            Self::Text(s) => match s.to_lowercase().as_str() {
                "true" | "1" | "yes" | "on" => Some(true),
                "false" | "0" | "no" | "off" => Some(false),
                _ => None,
            },
            _ => None,
        }
    }

    /// Attempt to interpret value as Color.
    pub fn as_color(&self) -> Option<Color> {
        match self {
            Self::Color(c) => Some(*c),
            Self::Text(s) => Color::from_hex(s).ok(),
            _ => None,
        }
    }
}

impl Default for VariableValue {
    fn default() -> Self {
        Self::Text(String::new())
    }
}

/// Schema declaration for a variable expected by a motion-graphics template or overlay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VariableDeclaration {
    /// Hierarchical dot-separated path (e.g. "scoreboard.team1.name").
    pub path: String,
    /// Expected data kind.
    pub kind: VariableKind,
    /// Default fallback value.
    pub default_value: VariableValue,
    /// Human-readable label displayed in the operator control panel.
    pub label: String,
}

impl VariableDeclaration {
    /// Construct a new variable declaration.
    pub fn new(
        path: impl Into<String>,
        kind: VariableKind,
        default_value: VariableValue,
        label: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            kind,
            default_value,
            label: label.into(),
        }
    }

    /// Helper for text variable declaration.
    pub fn text(
        path: impl Into<String>,
        default_text: impl Into<String>,
        label: impl Into<String>,
    ) -> Self {
        let val = default_text.into();
        Self::new(path, VariableKind::Text, VariableValue::Text(val), label)
    }

    /// Helper for numeric variable declaration.
    pub fn number(path: impl Into<String>, default_num: f64, label: impl Into<String>) -> Self {
        Self::new(
            path,
            VariableKind::Number,
            VariableValue::Number(default_num),
            label,
        )
    }

    /// Helper for boolean variable declaration.
    pub fn boolean(path: impl Into<String>, default_bool: bool, label: impl Into<String>) -> Self {
        Self::new(
            path,
            VariableKind::Boolean,
            VariableValue::Boolean(default_bool),
            label,
        )
    }
}
