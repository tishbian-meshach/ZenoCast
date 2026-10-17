//! # zeno-script
//!
//! Sandboxed expression execution environment using the Rhai scripting engine.
//! Provides After Effects-style expressions (`wiggle`, `linear`, `ease`, `clamp`, `time`, `index`, `value`),
//! instruction execution limits, and reactive data variable bindings.

pub mod builtins;
pub mod error;
pub mod runtime;

pub use builtins::{calculate_clamp, calculate_ease, calculate_linear, calculate_wiggle};
pub use error::ScriptError;
pub use runtime::{ScriptRuntime, MAX_EXPRESSION_OPERATIONS};
