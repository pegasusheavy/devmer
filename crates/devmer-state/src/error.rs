//! State error types

use thiserror::Error;

/// Result type for state operations
pub type Result<T> = std::result::Result<T, StateError>;

/// Alias for state result
pub type StateResult<T> = Result<T>;

/// State errors
#[derive(Error, Debug)]
pub enum StateError {
    /// State not found
    #[error("State not found for stack: {0}")]
    NotFound(String),

    /// State is locked
    #[error("State is locked: {0}")]
    Locked(String),

    /// Lock conflict with existing lock
    #[error("Lock conflict: held by {owner} for {operation} since {acquired_at}")]
    LockConflict {
        owner: String,
        operation: String,
        acquired_at: chrono::DateTime<chrono::Utc>,
    },

    /// Lock acquisition failed
    #[error("Failed to acquire lock: {0}")]
    LockFailed(String),

    /// Lock not held
    #[error("Lock not held or expired")]
    LockNotHeld,

    /// Internal lock poisoned
    #[error("Internal lock poisoned")]
    LockPoisoned,

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialize(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialize(String),

    /// Serialization error (legacy)
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Backend error
    #[error("Backend error: {0}")]
    BackendError(String),

    /// Encryption error
    #[error("Encryption error: {0}")]
    EncryptionError(String),

    /// Version conflict
    #[error("State version conflict: expected {expected}, found {found}")]
    VersionConflict { expected: u64, found: u64 },

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl StateError {
    /// Create a not found error
    pub fn not_found(stack: impl Into<String>) -> Self {
        Self::NotFound(stack.into())
    }

    /// Create a locked error
    pub fn locked(message: impl Into<String>) -> Self {
        Self::Locked(message.into())
    }

    /// Create a lock failed error
    pub fn lock_failed(message: impl Into<String>) -> Self {
        Self::LockFailed(message.into())
    }

    /// Create a backend error
    pub fn backend_error(message: impl Into<String>) -> Self {
        Self::BackendError(message.into())
    }
}
