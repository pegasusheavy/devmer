//! Error types for devmer-core

use thiserror::Error;

/// Result type alias for devmer operations
pub type Result<T> = std::result::Result<T, DevmerError>;

/// Core error types for Devmer
#[derive(Error, Debug)]
pub enum DevmerError {
    /// Resource not found
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    /// Provider not found
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),

    /// Invalid resource URN
    #[error("Invalid URN: {0}")]
    InvalidUrn(String),

    /// Dependency cycle detected in resource graph
    #[error("Dependency cycle detected: {0}")]
    DependencyCycle(String),

    /// Resource creation failed
    #[error("Failed to create resource '{name}': {message}")]
    CreateFailed { name: String, message: String },

    /// Resource update failed
    #[error("Failed to update resource '{name}': {message}")]
    UpdateFailed { name: String, message: String },

    /// Resource deletion failed
    #[error("Failed to delete resource '{name}': {message}")]
    DeleteFailed { name: String, message: String },

    /// State serialization/deserialization error
    #[error("State error: {0}")]
    StateError(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// Provider error
    #[error("Provider error: {0}")]
    ProviderError(String),

    /// Graph operation error
    #[error("Graph error: {0}")]
    GraphError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl DevmerError {
    /// Create a new resource not found error
    pub fn resource_not_found(id: impl Into<String>) -> Self {
        Self::ResourceNotFound(id.into())
    }

    /// Create a new provider not found error
    pub fn provider_not_found(name: impl Into<String>) -> Self {
        Self::ProviderNotFound(name.into())
    }

    /// Create a new create failed error
    pub fn create_failed(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::CreateFailed {
            name: name.into(),
            message: message.into(),
        }
    }

    /// Create a new update failed error
    pub fn update_failed(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::UpdateFailed {
            name: name.into(),
            message: message.into(),
        }
    }

    /// Create a new delete failed error
    pub fn delete_failed(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self::DeleteFailed {
            name: name.into(),
            message: message.into(),
        }
    }
}
