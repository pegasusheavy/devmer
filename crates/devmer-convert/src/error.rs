//! Error types for HCL conversion

use std::path::PathBuf;
use thiserror::Error;

/// Result type for conversion operations
pub type Result<T> = std::result::Result<T, ConvertError>;

/// Conversion errors
#[derive(Error, Debug)]
pub enum ConvertError {
    /// HCL parsing error
    #[error("Failed to parse HCL in {file}: {message}")]
    ParseError { file: PathBuf, message: String },

    /// Unsupported HCL feature
    #[error("Unsupported HCL feature: {0}")]
    UnsupportedFeature(String),

    /// Unknown resource type
    #[error("Unknown resource type: {0}")]
    UnknownResourceType(String),

    /// Unknown provider
    #[error("Unknown provider: {0}")]
    UnknownProvider(String),

    /// Invalid expression
    #[error("Invalid expression: {0}")]
    InvalidExpression(String),

    /// Code generation error
    #[error("Code generation error: {0}")]
    CodeGenError(String),

    /// Template error
    #[error("Template error: {0}")]
    TemplateError(String),

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// No HCL files found
    #[error("No HCL files found in directory: {0}")]
    NoHclFiles(PathBuf),

    /// IO error
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// Walkdir error
    #[error("Directory traversal error: {0}")]
    WalkdirError(#[from] walkdir::Error),
}

impl ConvertError {
    /// Create a parse error
    pub fn parse_error(file: impl Into<PathBuf>, message: impl Into<String>) -> Self {
        Self::ParseError {
            file: file.into(),
            message: message.into(),
        }
    }

    /// Create an unsupported feature error
    pub fn unsupported(feature: impl Into<String>) -> Self {
        Self::UnsupportedFeature(feature.into())
    }

    /// Create a code generation error
    pub fn codegen(message: impl Into<String>) -> Self {
        Self::CodeGenError(message.into())
    }
}

impl From<hcl::Error> for ConvertError {
    fn from(err: hcl::Error) -> Self {
        Self::ParseError {
            file: PathBuf::new(),
            message: err.to_string(),
        }
    }
}

impl From<tera::Error> for ConvertError {
    fn from(err: tera::Error) -> Self {
        Self::TemplateError(err.to_string())
    }
}
