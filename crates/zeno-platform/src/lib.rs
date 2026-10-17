//! # zeno-platform
//!
//! Windows platform abstractions, COM wrappers, and isolated unsafe FFI logic.
//!
//! In accordance with the project rules, `unsafe` is strictly quarantined to this crate
//! and direct FFI interfaces. Every unsafe block includes a dedicated `// SAFETY:` rationale.
