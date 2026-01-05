//! Language runtime trait and types

use crate::registry::ResourceRegistry;
use crate::Result;
use async_trait::async_trait;
use devmer_config::DevmerConfig;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// Kind of runtime
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeKind {
    /// TypeScript/JavaScript with Node.js
    Node,
    /// TypeScript/JavaScript with Deno
    Deno,
    /// TypeScript/JavaScript with Bun
    Bun,
    /// Python
    Python,
    /// Go
    Go,
    /// Rhai (embedded)
    Rhai,
}

impl RuntimeKind {
    /// Get the executable name for this runtime
    pub fn executable(&self) -> &'static str {
        match self {
            RuntimeKind::Node => "node",
            RuntimeKind::Deno => "deno",
            RuntimeKind::Bun => "bun",
            RuntimeKind::Python => "python3",
            RuntimeKind::Go => "go",
            RuntimeKind::Rhai => "", // Embedded
        }
    }

    /// Get the default entry point for this runtime
    pub fn default_entry_point(&self) -> &'static str {
        match self {
            RuntimeKind::Node | RuntimeKind::Deno | RuntimeKind::Bun => "index.ts",
            RuntimeKind::Python => "__main__.py",
            RuntimeKind::Go => "main.go",
            RuntimeKind::Rhai => "main.rhai",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "node" | "nodejs" => Some(RuntimeKind::Node),
            "deno" => Some(RuntimeKind::Deno),
            "bun" => Some(RuntimeKind::Bun),
            "python" | "python3" | "py" => Some(RuntimeKind::Python),
            "go" | "golang" => Some(RuntimeKind::Go),
            "rhai" => Some(RuntimeKind::Rhai),
            "typescript" | "ts" | "javascript" | "js" => Some(RuntimeKind::Node),
            _ => None,
        }
    }

    /// Check if this is an embedded runtime
    pub fn is_embedded(&self) -> bool {
        matches!(self, RuntimeKind::Rhai)
    }
}

impl std::fmt::Display for RuntimeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeKind::Node => write!(f, "node"),
            RuntimeKind::Deno => write!(f, "deno"),
            RuntimeKind::Bun => write!(f, "bun"),
            RuntimeKind::Python => write!(f, "python"),
            RuntimeKind::Go => write!(f, "go"),
            RuntimeKind::Rhai => write!(f, "rhai"),
        }
    }
}

/// Configuration for a runtime
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Runtime kind
    pub kind: RuntimeKind,

    /// Working directory
    pub working_dir: PathBuf,

    /// Entry point file
    pub entry_point: PathBuf,

    /// Stack name
    pub stack: String,

    /// Project configuration
    pub project_config: DevmerConfig,

    /// Environment variables
    pub env_vars: std::collections::HashMap<String, String>,

    /// Execution timeout
    pub timeout: Duration,

    /// Whether this is a preview (dry run)
    pub preview: bool,

    /// gRPC server address for IPC
    pub grpc_address: String,
}

impl RuntimeConfig {
    /// Create a new runtime config
    pub fn new(kind: RuntimeKind, working_dir: PathBuf, stack: &str) -> Self {
        Self {
            kind,
            working_dir: working_dir.clone(),
            entry_point: working_dir.join(kind.default_entry_point()),
            stack: stack.to_string(),
            project_config: DevmerConfig::default(),
            env_vars: std::collections::HashMap::new(),
            timeout: Duration::from_secs(3600), // 1 hour default
            preview: false,
            grpc_address: "127.0.0.1:0".to_string(),
        }
    }

    /// Set the entry point
    pub fn with_entry_point(mut self, entry: PathBuf) -> Self {
        self.entry_point = entry;
        self
    }

    /// Set preview mode
    pub fn with_preview(mut self, preview: bool) -> Self {
        self.preview = preview;
        self
    }

    /// Add environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
}

/// Result of running a program
#[derive(Debug, Clone)]
pub struct RunResult {
    /// Whether execution succeeded
    pub success: bool,

    /// Exit code (if applicable)
    pub exit_code: Option<i32>,

    /// Collected resources
    pub resources: ResourceRegistry,

    /// Stdout output
    pub stdout: String,

    /// Stderr output
    pub stderr: String,

    /// Duration
    pub duration: Duration,

    /// Errors during execution
    pub errors: Vec<String>,
}

impl RunResult {
    /// Create a successful result
    pub fn success(resources: ResourceRegistry, duration: Duration) -> Self {
        Self {
            success: true,
            exit_code: Some(0),
            resources,
            stdout: String::new(),
            stderr: String::new(),
            duration,
            errors: vec![],
        }
    }

    /// Create a failure result
    pub fn failure(error: impl Into<String>, duration: Duration) -> Self {
        Self {
            success: false,
            exit_code: None,
            resources: ResourceRegistry::new(),
            stdout: String::new(),
            stderr: String::new(),
            duration,
            errors: vec![error.into()],
        }
    }
}

/// Language runtime trait
#[async_trait]
pub trait LanguageRuntime: Send + Sync {
    /// Get the runtime kind
    fn kind(&self) -> RuntimeKind;

    /// Check if the runtime is available on the system
    async fn is_available(&self) -> bool;

    /// Get the runtime version
    async fn version(&self) -> Result<String>;

    /// Run a program and collect resources
    async fn run(&self, config: &RuntimeConfig) -> Result<RunResult>;

    /// Install dependencies (npm install, pip install, etc.)
    async fn install_dependencies(&self, config: &RuntimeConfig) -> Result<()>;
}
