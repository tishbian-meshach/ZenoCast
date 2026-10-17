//! Script evaluation and compilation errors.

use thiserror::Error;

/// Errors arising during expression compilation or sandboxed evaluation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ScriptError {
    /// Expression syntax or compilation failure.
    #[error("Script compilation error: {0}")]
    CompileError(String),

    /// Runtime evaluation execution error.
    #[error("Script evaluation error: {0}")]
    RuntimeError(String),

    /// Instruction count limit exceeded (infinite loop protection).
    #[error("Script instruction limit exceeded (infinite loop terminated): {0}")]
    InstructionLimitExceeded(String),

    /// Execution timeout exceeded.
    #[error("Script execution timed out after {0:?}")]
    Timeout(String),

    /// Return value type mismatch.
    #[error("Unexpected return type: expected {expected}, got {actual}")]
    TypeMismatch {
        expected: &'static str,
        actual: String,
    },
}
