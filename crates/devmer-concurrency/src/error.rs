//! Error types for concurrency operations.

use thiserror::Error;

use crate::lock::LockInfo;

/// Errors that can occur during concurrency operations.
#[derive(Error, Debug)]
pub enum ConcurrencyError {
    /// Resource is locked by another user/process.
    #[error("resource '{resource}' is locked by {holder} since {since} (operation: {operation})")]
    ResourceLocked {
        resource: String,
        holder: String,
        operation: String,
        since: chrono::DateTime<chrono::Utc>,
        lock_id: String,
    },

    /// Lock not found.
    #[error("lock not found: {0}")]
    LockNotFound(String),

    /// Lock expired.
    #[error("lock expired: {0}")]
    LockExpired(String),

    /// Invalid lock owner - trying to release someone else's lock.
    #[error("cannot release lock: owned by '{owner}', not '{requester}'")]
    InvalidLockOwner { owner: String, requester: String },

    /// Lock acquisition timeout.
    #[error("timeout waiting for lock on '{resource}' after {timeout_secs} seconds")]
    LockTimeout { resource: String, timeout_secs: u64 },

    /// Queue position error.
    #[error("queue error: {0}")]
    QueueError(String),

    /// Session not found.
    #[error("session not found: {0}")]
    SessionNotFound(String),

    /// Session expired.
    #[error("session expired: {0}")]
    SessionExpired(String),

    /// Conflict detected.
    #[error("conflict detected: {0}")]
    ConflictDetected(String),

    /// Operation already in progress.
    #[error("operation '{operation}' already in progress on '{resource}' by {holder}")]
    OperationInProgress {
        resource: String,
        operation: String,
        holder: String,
    },

    /// Backend error.
    #[error("backend error: {0}")]
    BackendError(String),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl ConcurrencyError {
    /// Create a ResourceLocked error from LockInfo.
    pub fn from_lock_info(resource: impl Into<String>, info: &LockInfo) -> Self {
        Self::ResourceLocked {
            resource: resource.into(),
            holder: info.holder.clone(),
            operation: info.operation.clone(),
            since: info.acquired_at,
            lock_id: info.id.to_string(),
        }
    }
}

/// Result type alias for concurrency operations.
pub type Result<T> = std::result::Result<T, ConcurrencyError>;
