//! Error types for Cloudmer integration.

use thiserror::Error;

/// Errors that can occur during Cloudmer operations.
#[derive(Error, Debug)]
pub enum CloudmerError {
    /// Authentication failed.
    #[error("authentication failed: {0}")]
    AuthenticationFailed(String),

    /// API token is missing or invalid.
    #[error("invalid or missing API token")]
    InvalidToken,

    /// API request failed.
    #[error("API request failed: {0}")]
    RequestFailed(String),

    /// Rate limit exceeded.
    #[error("rate limit exceeded, retry after {retry_after_secs} seconds")]
    RateLimited { retry_after_secs: u64 },

    /// Resource not found.
    #[error("resource not found: {0}")]
    NotFound(String),

    /// Invalid response from API.
    #[error("invalid API response: {0}")]
    InvalidResponse(String),

    /// Network error.
    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    /// Serialization error.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Configuration error.
    #[error("configuration error: {0}")]
    ConfigError(String),

    /// Project not linked.
    #[error("project not linked to Cloudmer, run `devmer cloudmer link` first")]
    ProjectNotLinked,
}

/// Result type alias for Cloudmer operations.
pub type Result<T> = std::result::Result<T, CloudmerError>;
