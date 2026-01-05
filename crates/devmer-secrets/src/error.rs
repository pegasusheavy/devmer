//! Secrets error types

use thiserror::Error;

/// Result type for secrets operations
pub type Result<T> = std::result::Result<T, SecretsError>;

/// Secrets errors
#[derive(Error, Debug)]
pub enum SecretsError {
    /// Encryption failed
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),

    /// Decryption failed
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),

    /// Invalid key
    #[error("Invalid encryption key: {0}")]
    InvalidKey(String),

    /// Key derivation failed
    #[error("Key derivation failed: {0}")]
    KeyDerivationFailed(String),

    /// Provider not configured
    #[error("Secrets provider not configured: {0}")]
    ProviderNotConfigured(String),

    /// Secret not found
    #[error("Secret not found: {0}")]
    SecretNotFound(String),

    /// Invalid ciphertext format
    #[error("Invalid ciphertext format: {0}")]
    InvalidCiphertext(String),

    /// Access denied
    #[error("Access denied: {0}")]
    AccessDenied(String),

    /// Provider error
    #[error("Provider error: {0}")]
    ProviderError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

impl SecretsError {
    /// Create an encryption failed error
    pub fn encryption_failed(message: impl Into<String>) -> Self {
        Self::EncryptionFailed(message.into())
    }

    /// Create a decryption failed error
    pub fn decryption_failed(message: impl Into<String>) -> Self {
        Self::DecryptionFailed(message.into())
    }

    /// Create an invalid key error
    pub fn invalid_key(message: impl Into<String>) -> Self {
        Self::InvalidKey(message.into())
    }
}
