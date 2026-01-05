//! Language host service and client
//!
//! The LanguageHost service is called by the engine to:
//! - Run programs written in various languages
//! - Get required plugins/providers
//! - Install dependencies

use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use tonic::Status;

/// Language host service trait
#[async_trait]
pub trait LanguageHostService: Send + Sync {
    /// Get plugin info
    async fn get_plugin_info(&self) -> Result<PluginInfo, HostError>;

    /// Run a program
    async fn run(&self, request: RunRequest) -> Result<RunResponse, HostError>;

    /// Get required plugins for a program
    async fn get_required_plugins(&self, request: GetRequiredPluginsRequest) -> Result<Vec<PluginDependency>, HostError>;

    /// Install dependencies
    async fn install_dependencies(&self, directory: PathBuf, is_terminal: bool) -> Result<(), HostError>;

    /// Get program dependencies
    async fn get_program_dependencies(&self, request: GetProgramDependenciesRequest) -> Result<Vec<DependencyInfo>, HostError>;
}

/// Plugin information
#[derive(Debug, Clone)]
pub struct PluginInfo {
    /// Plugin name
    pub name: String,
    /// Plugin version
    pub version: String,
}

/// Run request
#[derive(Debug, Clone)]
pub struct RunRequest {
    /// Project name
    pub project: String,
    /// Stack name
    pub stack: String,
    /// Working directory
    pub pwd: PathBuf,
    /// Program path
    pub program: PathBuf,
    /// Program arguments
    pub args: Vec<String>,
    /// Configuration values
    pub config: HashMap<String, String>,
    /// Dry run (preview) mode
    pub dry_run: bool,
    /// Parallelism
    pub parallel: i32,
    /// Engine address for callbacks
    pub engine_address: String,
    /// Organization name
    pub organization: Option<String>,
}

/// Run response
#[derive(Debug, Clone)]
pub struct RunResponse {
    /// Error message if failed
    pub error: Option<String>,
    /// Whether to bail out
    pub bail: bool,
}

impl RunResponse {
    /// Create a successful response
    pub fn success() -> Self {
        Self {
            error: None,
            bail: false,
        }
    }

    /// Create an error response
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            error: Some(message.into()),
            bail: false,
        }
    }

    /// Create a bail response
    pub fn bail(message: impl Into<String>) -> Self {
        Self {
            error: Some(message.into()),
            bail: true,
        }
    }
}

/// Request to get required plugins
#[derive(Debug, Clone)]
pub struct GetRequiredPluginsRequest {
    /// Project name
    pub project: String,
    /// Working directory
    pub pwd: PathBuf,
    /// Program path
    pub program: PathBuf,
}

/// Plugin dependency
#[derive(Debug, Clone)]
pub struct PluginDependency {
    /// Plugin name
    pub name: String,
    /// Plugin kind ("resource" or "analyzer")
    pub kind: String,
    /// Version constraint
    pub version: String,
    /// Optional download server
    pub server: Option<String>,
}

/// Request to get program dependencies
#[derive(Debug, Clone)]
pub struct GetProgramDependenciesRequest {
    /// Project name
    pub project: String,
    /// Working directory
    pub pwd: PathBuf,
    /// Program path
    pub program: PathBuf,
    /// Include transitive dependencies
    pub transitive: bool,
}

/// Dependency information
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    /// Dependency name
    pub name: String,
    /// Version
    pub version: String,
}

/// Language host errors
#[derive(Debug, thiserror::Error)]
pub enum HostError {
    #[error("Program not found: {0}")]
    ProgramNotFound(String),

    #[error("Program execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Dependency installation failed: {0}")]
    DependencyInstallFailed(String),

    #[error("Invalid program: {0}")]
    InvalidProgram(String),

    #[error("Timeout")]
    Timeout,

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<HostError> for Status {
    fn from(err: HostError) -> Self {
        match err {
            HostError::ProgramNotFound(msg) => Status::not_found(msg),
            HostError::ExecutionFailed(msg) => Status::internal(msg),
            HostError::DependencyInstallFailed(msg) => Status::failed_precondition(msg),
            HostError::InvalidProgram(msg) => Status::invalid_argument(msg),
            HostError::Timeout => Status::deadline_exceeded("Operation timed out"),
            HostError::Internal(msg) => Status::internal(msg),
        }
    }
}

/// Language host client for connecting to external language hosts
pub struct LanguageHostClient {
    address: String,
}

impl LanguageHostClient {
    /// Create a new language host client
    pub fn new(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
        }
    }

    /// Get the server address
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Connect to the language host
    pub async fn connect(&self) -> Result<(), HostError> {
        // In a real implementation, this would establish a gRPC connection
        Ok(())
    }
}

/// Supported language runtimes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanguageRuntime {
    /// Node.js (TypeScript/JavaScript)
    NodeJs,
    /// Deno
    Deno,
    /// Bun
    Bun,
    /// Python
    Python,
    /// Go
    Go,
    /// Rhai (embedded)
    Rhai,
}

impl LanguageRuntime {
    /// Get the language host binary name
    pub fn host_binary(&self) -> &'static str {
        match self {
            LanguageRuntime::NodeJs => "devmer-language-nodejs",
            LanguageRuntime::Deno => "devmer-language-deno",
            LanguageRuntime::Bun => "devmer-language-bun",
            LanguageRuntime::Python => "devmer-language-python",
            LanguageRuntime::Go => "devmer-language-go",
            LanguageRuntime::Rhai => "embedded", // No external binary
        }
    }

    /// Detect runtime from file extension
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "ts" | "js" | "mts" | "mjs" => Some(LanguageRuntime::NodeJs),
            "py" => Some(LanguageRuntime::Python),
            "go" => Some(LanguageRuntime::Go),
            "rhai" => Some(LanguageRuntime::Rhai),
            _ => None,
        }
    }

    /// Detect runtime from program path
    pub fn detect(program: &std::path::Path) -> Option<Self> {
        program
            .extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        assert_eq!(
            LanguageRuntime::from_extension("ts"),
            Some(LanguageRuntime::NodeJs)
        );
        assert_eq!(
            LanguageRuntime::from_extension("py"),
            Some(LanguageRuntime::Python)
        );
        assert_eq!(
            LanguageRuntime::from_extension("rhai"),
            Some(LanguageRuntime::Rhai)
        );
        assert_eq!(LanguageRuntime::from_extension("unknown"), None);
    }

    #[test]
    fn test_run_response() {
        let success = RunResponse::success();
        assert!(success.error.is_none());
        assert!(!success.bail);

        let error = RunResponse::error("failed");
        assert_eq!(error.error, Some("failed".to_string()));
        assert!(!error.bail);

        let bail = RunResponse::bail("critical");
        assert!(bail.bail);
    }
}
