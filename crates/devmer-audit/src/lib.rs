//! # devmer-audit
//!
//! Comprehensive audit logging and compliance reporting for Devmer.
//!
//! This crate provides:
//! - Audit event capture and storage
//! - Hash chaining for tamper-evidence
//! - Multiple storage backends (file, S3, CloudWatch, PostgreSQL)
//! - Compliance report generation (SOC2, HIPAA, PCI-DSS)
//! - SIEM integration and export formats
//!
//! ## Features
//!
//! - `parquet` - Enable Parquet file format for archival
//! - `cloudwatch` - Enable AWS CloudWatch Logs backend
//! - `s3` - Enable AWS S3 archival backend
//! - `postgres` - Enable PostgreSQL backend
//! - `full` - Enable all features
//!
//! ## Example
//!
//! ```rust,ignore
//! use devmer_audit::{AuditLogger, AuditEvent, EventType, FileBackend};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create a file-based audit logger
//!     let backend = FileBackend::new(".devmer/audit")?;
//!     let logger = AuditLogger::new(backend);
//!
//!     // Log an event
//!     logger.log(AuditEvent::deployment_started("dev", "user@example.com")).await?;
//!
//!     Ok(())
//! }
//! ```

pub mod backend;
pub mod chain;
pub mod compliance;
pub mod event;
pub mod export;
pub mod logger;
pub mod query;
pub mod report;

pub use backend::{AuditBackend, FileBackend, MemoryBackend};
pub use chain::{HashChain, ChainedEvent};
pub use compliance::{ComplianceFramework, ComplianceReport, ComplianceChecker};
pub use event::{AuditEvent, EventType, EventSeverity, EventOutcome, Actor, Resource as AuditResource};
pub use export::{ExportFormat, Exporter};
pub use logger::AuditLogger;
pub use query::{AuditQuery, QueryResult, TimeRange};
pub use report::{ReportGenerator, ReportFormat, ReportConfig};

/// Result type for audit operations
pub type Result<T> = std::result::Result<T, AuditError>;

/// Audit-specific errors
#[derive(Debug, thiserror::Error)]
pub enum AuditError {
    #[error("Failed to write audit event: {0}")]
    WriteError(String),

    #[error("Failed to read audit events: {0}")]
    ReadError(String),

    #[error("Hash chain integrity violation at event {event_id}")]
    ChainIntegrityError { event_id: String },

    #[error("Backend not available: {0}")]
    BackendUnavailable(String),

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Report generation failed: {0}")]
    ReportError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Template error: {0}")]
    TemplateError(String),
}
