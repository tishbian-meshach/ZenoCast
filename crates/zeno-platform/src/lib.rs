//! # zeno-platform
//!
//! Windows platform abstractions, COM wrappers, and isolated unsafe FFI logic.
//!
//! In accordance with the project rules, `unsafe` is strictly quarantined to this crate
//! and direct FFI interfaces. Every unsafe block includes a dedicated `// SAFETY:` rationale.

pub mod camera;
pub mod capture;
pub mod display;
pub mod window;

pub use camera::{enumerate_camera_devices, CameraDeviceInfo, CameraFormat};
pub use capture::{capture_display_raw, capture_window_raw};
pub use display::{enumerate_displays, DisplayInfo};
pub use window::{enumerate_windows, WindowInfo};
