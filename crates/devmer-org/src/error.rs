//! Error types for organization management

use thiserror::Error;

/// Result type for organization operations
pub type Result<T> = std::result::Result<T, OrgError>;

/// Organization errors
#[derive(Error, Debug)]
pub enum OrgError {
    /// Organization not found
    #[error("Organization not found: {0}")]
    OrganizationNotFound(String),

    /// Team not found
    #[error("Team not found: {0}")]
    TeamNotFound(String),

    /// User not found
    #[error("User not found: {0}")]
    UserNotFound(String),

    /// Role not found
    #[error("Role not found: {0}")]
    RoleNotFound(String),

    /// Permission denied
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Access denied by policy
    #[error("Access denied by policy: {action} on {resource}")]
    AccessDenied { action: String, resource: String },

    /// Approval required
    #[error("Approval required for: {0}")]
    ApprovalRequired(String),

    /// Approval pending
    #[error("Approval pending: {0}")]
    ApprovalPending(String),

    /// Approval rejected
    #[error("Approval rejected: {0}")]
    ApprovalRejected(String),

    /// Invalid policy
    #[error("Invalid policy: {0}")]
    InvalidPolicy(String),

    /// Duplicate entry
    #[error("Duplicate entry: {0}")]
    Duplicate(String),

    /// Invalid operation
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Storage error
    #[error("Storage error: {0}")]
    Storage(String),
}
