//! # devmer-rpc
//!
//! gRPC protocol definitions and services for Devmer language hosts.
//!
//! This crate provides:
//! - Protocol buffer definitions for resource operations
//! - gRPC service implementations
//! - Communication between core engine and language hosts
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐         gRPC          ┌─────────────────┐
//! │  Language Host  │◄──────────────────────►│  Devmer Engine  │
//! │  (Python/TS/Go) │                        │                 │
//! └─────────────────┘                        └─────────────────┘
//!        │                                          │
//!        │ RegisterResource()                       │
//!        │ RegisterComponent()                      │
//!        │ GetConfig()                              │
//!        │ GetSecret()                              │
//!        │ Log()                                    │
//!        ▼                                          ▼
//! ┌─────────────────┐                        ┌─────────────────┐
//! │   SDK Library   │                        │    Providers    │
//! └─────────────────┘                        └─────────────────┘
//! ```
//!
//! ## Services
//!
//! - **Engine**: Called by language hosts to register resources, get config, etc.
//! - **LanguageHost**: Called by engine to run programs
//! - **Provider**: Resource provider operations (CRUD)

pub mod proto {
    // Generated protobuf code
    #[cfg(feature = "codegen")]
    tonic::include_proto!("devmer");
}

pub mod engine;
pub mod host;
pub mod provider;
pub mod convert;

/// Re-export tonic for use by dependent crates
pub use tonic;
pub use prost;

// Re-export service traits and types
pub use engine::{EngineService, EngineServer};
pub use host::{LanguageHostClient, LanguageHostService};
pub use provider::{ProviderClient, ProviderService};

/// Log severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSeverity {
    Debug,
    Info,
    Warning,
    Error,
}

impl From<i32> for LogSeverity {
    fn from(v: i32) -> Self {
        match v {
            1 => LogSeverity::Debug,
            2 => LogSeverity::Info,
            3 => LogSeverity::Warning,
            4 => LogSeverity::Error,
            _ => LogSeverity::Info,
        }
    }
}

impl From<LogSeverity> for i32 {
    fn from(v: LogSeverity) -> Self {
        match v {
            LogSeverity::Debug => 1,
            LogSeverity::Info => 2,
            LogSeverity::Warning => 3,
            LogSeverity::Error => 4,
        }
    }
}

/// Resource registration request (native Rust type)
#[derive(Debug, Clone)]
pub struct RegisterResourceRequest {
    /// Resource type (e.g., "aws:s3:Bucket")
    pub resource_type: String,
    /// Resource name
    pub name: String,
    /// Input properties as JSON
    pub inputs: serde_json::Value,
    /// Parent URN
    pub parent: Option<String>,
    /// Provider reference
    pub provider: Option<String>,
    /// Dependencies
    pub dependencies: Vec<String>,
    /// Protect from deletion
    pub protect: bool,
    /// Properties to ignore during diff
    pub ignore_changes: Vec<String>,
}

/// Resource registration response (native Rust type)
#[derive(Debug, Clone)]
pub struct RegisterResourceResponse {
    /// Assigned URN
    pub urn: String,
    /// Resource ID
    pub id: String,
    /// Output properties as JSON
    pub outputs: serde_json::Value,
    /// Whether creation is complete
    pub stable: bool,
}

/// Component registration request
#[derive(Debug, Clone)]
pub struct RegisterComponentRequest {
    /// Component type
    pub component_type: String,
    /// Component name
    pub name: String,
    /// Parent URN
    pub parent: Option<String>,
}

/// Component registration response
#[derive(Debug, Clone)]
pub struct RegisterComponentResponse {
    /// Component URN
    pub urn: String,
}

/// Log message from language host
#[derive(Debug, Clone)]
pub struct LogRequest {
    /// Severity level
    pub severity: LogSeverity,
    /// Log message
    pub message: String,
    /// Optional resource URN
    pub urn: Option<String>,
    /// Stream ID (stdout/stderr)
    pub stream_id: Option<i32>,
    /// Whether this is ephemeral
    pub ephemeral: bool,
}

/// Configuration for connecting to a gRPC server
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Server address
    pub address: String,
    /// Connection timeout in seconds
    pub timeout_secs: u64,
    /// Enable TLS
    pub tls: bool,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1:50051".to_string(),
            timeout_secs: 30,
            tls: false,
        }
    }
}

impl ConnectionConfig {
    /// Create config for local connection
    pub fn local(port: u16) -> Self {
        Self {
            address: format!("127.0.0.1:{}", port),
            ..Default::default()
        }
    }

    /// Create config from address string
    pub fn from_address(address: impl Into<String>) -> Self {
        Self {
            address: address.into(),
            ..Default::default()
        }
    }
}
