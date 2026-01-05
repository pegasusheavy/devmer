//! # devmer-state
//!
//! State management and backend implementations for Devmer.
//!
//! This crate provides:
//! - State backend trait definition
//! - Multiple backend implementations:
//!   - **Local** - File-based storage (default)
//!   - **S3** - AWS S3 with DynamoDB locking
//!   - **GCS** - Google Cloud Storage
//!   - **Azure** - Azure Blob Storage
//! - State locking mechanisms
//! - State encryption
//!
//! ## Example
//!
//! ```rust,ignore
//! use devmer_state::s3::{S3Backend, S3BackendConfig};
//!
//! let config = S3BackendConfig::new("my-bucket")
//!     .with_region("us-west-2")
//!     .with_lock_table("devmer-locks");
//!
//! let backend = S3Backend::new("my-project", config);
//! ```

pub mod backend;
pub mod error;
pub mod locking;

// Backend implementations
#[cfg(feature = "local")]
pub mod local;

pub mod s3;
pub mod gcs;
pub mod azure;

pub use backend::{StateBackend, StateHistory};
pub use error::{StateError, StateResult, Result};
pub use locking::{LockId, LockInfo, LockStatus};

// Re-export backend types
pub use s3::{S3Backend, S3BackendConfig};
pub use gcs::{GcsBackend, GcsBackendConfig};
pub use azure::{AzureBackend, AzureBackendConfig};
