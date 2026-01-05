//! Configuration error types

use thiserror::Error;

/// Result type for configuration operations
pub type Result<T> = std::result::Result<T, ConfigError>;

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    /// File not found
    #[error("Configuration file not found: {0}")]
    FileNotFound(String),

    /// Parse error
    #[error("Failed to parse configuration: {0}")]
    ParseError(String),

    /// Missing required field
    #[error("Missing required field: {0}")]
    MissingField(String),

    /// Invalid value
    #[error("Invalid value for '{field}': {message}")]
    InvalidValue { field: String, message: String },

    /// Environment variable not found
    #[error("Environment variable not found: {0}")]
    EnvVarNotFound(String),

    /// Environment variable interpolation error
    #[error("Failed to interpolate '{variable}': {message}")]
    InterpolationError { variable: String, message: String },

    /// File read error
    #[error("Failed to read file '{path}': {message}")]
    FileReadError { path: String, message: String },

    /// Validation error
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// TOML parse error
    #[error("TOML parse error: {0}")]
    TomlError(#[from] toml::de::Error),
}

impl ConfigError {
    /// Create a missing field error
    pub fn missing_field(field: impl Into<String>) -> Self {
        Self::MissingField(field.into())
    }

    /// Create an invalid value error
    pub fn invalid_value(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InvalidValue {
            field: field.into(),
            message: message.into(),
        }
    }

    /// Create an interpolation error
    pub fn interpolation_error(variable: impl Into<String>, message: impl Into<String>) -> Self {
        Self::InterpolationError {
            variable: variable.into(),
            message: message.into(),
        }
    }
}
