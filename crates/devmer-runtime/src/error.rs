//! Runtime error types

use std::path::PathBuf;
use thiserror::Error;

/// Result type for runtime operations
pub type Result<T> = std::result::Result<T, RuntimeError>;

/// Runtime errors
#[derive(Error, Debug)]
pub enum RuntimeError {
    /// Runtime not found
    #[error("Runtime '{0}' not found on system")]
    RuntimeNotFound(String),

    /// Runtime executable not found
    #[error("Executable not found: {0}")]
    ExecutableNotFound(PathBuf),

    /// Program execution failed
    #[error("Program execution failed: {0}")]
    ExecutionFailed(String),

    /// Program exited with non-zero code
    #[error("Program exited with code {code}: {stderr}")]
    NonZeroExit { code: i32, stderr: String },

    /// Timeout during execution
    #[error("Execution timed out after {0} seconds")]
    Timeout(u64),

    /// IPC error
    #[error("IPC error: {0}")]
    IpcError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Invalid program
    #[error("Invalid program: {0}")]
    InvalidProgram(String),

    /// Resource registration error
    #[error("Resource registration error: {0}")]
    RegistrationError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Rhai script error
    #[error("Rhai script error: {0}")]
    RhaiError(String),
}

impl RuntimeError {
    /// Create an execution failed error
    pub fn execution_failed(msg: impl Into<String>) -> Self {
        Self::ExecutionFailed(msg.into())
    }

    /// Create a non-zero exit error
    pub fn non_zero_exit(code: i32, stderr: impl Into<String>) -> Self {
        Self::NonZeroExit {
            code,
            stderr: stderr.into(),
        }
    }
}
