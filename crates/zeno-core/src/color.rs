//! Color representations and conversions.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// 32-bit floating point RGBA color with components in [0.0, 1.0].
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    /// Red channel [0.0, 1.0].
    pub r: f32,
    /// Green channel [0.0, 1.0].
    pub g: f32,
    /// Blue channel [0.0, 1.0].
    pub b: f32,
    /// Alpha / Opacity channel [0.0, 1.0].
    pub a: f32,
}

impl Color {
    /// Pure transparent black (0, 0, 0, 0).
    pub const TRANSPARENT: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };
    /// Opaque black.
    pub const BLACK: Self = Self {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    /// Opaque white.
    pub const WHITE: Self = Self {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };

    /// Create a new RGBA color.
    #[inline]
    pub const fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create an opaque RGB color (alpha = 1.0).
    #[inline]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Convert from 8-bit unsigned integers (0..=255).
    pub fn from_u8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Convert to 8-bit unsigned integers (0..=255).
    pub fn to_u8_array(&self) -> [u8; 4] {
        [
            (self.r.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.g.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.b.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.a.clamp(0.0, 1.0) * 255.0).round() as u8,
        ]
    }

    /// Convert to array of 4 floats [r, g, b, a].
    #[inline]
    pub const fn to_f32_array(&self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Linear interpolation between two colors.
    pub fn lerp(&self, other: &Self, t: f32) -> Self {
        let t = t.clamp(0.0, 1.0);
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    /// Convert from hex string `#RRGGBB` or `#RRGGBBAA`.
    pub fn from_hex(hex: &str) -> Result<Self, String> {
        let clean = hex.trim().trim_start_matches('#');
        match clean.len() {
            6 => {
                let r = u8::from_str_radix(&clean[0..2], 16).map_err(|e| e.to_string())?;
                let g = u8::from_str_radix(&clean[2..4], 16).map_err(|e| e.to_string())?;
                let b = u8::from_str_radix(&clean[4..6], 16).map_err(|e| e.to_string())?;
                Ok(Self::from_u8(r, g, b, 255))
            }
            8 => {
                let r = u8::from_str_radix(&clean[0..2], 16).map_err(|e| e.to_string())?;
                let g = u8::from_str_radix(&clean[2..4], 16).map_err(|e| e.to_string())?;
                let b = u8::from_str_radix(&clean[4..6], 16).map_err(|e| e.to_string())?;
                let a = u8::from_str_radix(&clean[6..8], 16).map_err(|e| e.to_string())?;
                Ok(Self::from_u8(r, g, b, a))
            }
            _ => Err(format!("Invalid hex color string: {}", hex)),
        }
    }

    /// Format as hex string `#RRGGBBAA`.
    pub fn to_hex(&self) -> String {
        let [r, g, b, a] = self.to_u8_array();
        format!("#{:02X}{:02X}{:02X}{:02X}", r, g, b, a)
    }
}

impl Default for Color {
    fn default() -> Self {
        Self::WHITE
    }
}

impl From<[f32; 4]> for Color {
    fn from(arr: [f32; 4]) -> Self {
        Self {
            r: arr[0],
            g: arr[1],
            b: arr[2],
            a: arr[3],
        }
    }
}

impl From<Color> for [f32; 4] {
    fn from(c: Color) -> Self {
        [c.r, c.g, c.b, c.a]
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl FromStr for Color {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s)
    }
}
