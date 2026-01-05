//! # devmer-concurrency
//!
//! Distributed locking and multi-user coordination for Devmer.
//!
//! This crate ensures that **only one cloud update can run at a time** for a given
//! resource (project/stack). It provides:
//!
//! - **Distributed Lock Manager**: Prevents concurrent modifications
//! - **Lock Queuing**: Fair ordering when multiple users wait for the same resource
//! - **Heartbeat & Lease Renewal**: Automatic TTL extension for long operations
//! - **Session Tracking**: Know who else is working on a resource
//! - **Conflict Detection**: Pre-operation checks to prevent conflicts
//! - **Operation Journal**: Full audit trail of all lock operations
//!
//! ## Why This Matters
//!
//! Infrastructure-as-Code tools manipulate cloud resources that can have serious
//! consequences if modified incorrectly or concurrently:
//!
//! - **Race conditions**: Two users deploying at once can cause inconsistent state
//! - **State corruption**: Concurrent writes to state files can corrupt them
//! - **Resource conflicts**: Creating the same resource twice causes errors
//! - **Rollback issues**: Hard to rollback when multiple changes are interleaved
//!
//! ## Example
//!
//! ```rust,ignore
//! use devmer_concurrency::{
//!     LockManager, LockRequest, ConflictDetector, SessionManager,
//! };
//! use std::sync::Arc;
//! use chrono::Duration;
//!
//! // Create managers
//! let lock_manager = Arc::new(LockManager::in_memory());
//! let session_manager = Arc::new(SessionManager::in_memory());
//! let conflict_detector = ConflictDetector::new(
//!     lock_manager.clone(),
//!     session_manager.clone(),
//! );
//!
//! // Before starting an operation, check for conflicts
//! let check = conflict_detector
//!     .check_before_operation("my-project/production", "user@example.com", "deploy")
//!     .await?;
//!
//! if !check.can_proceed {
//!     eprintln!("Cannot proceed due to conflicts:");
//!     for conflict in &check.conflicts {
//!         eprintln!("  - {}", conflict.description);
//!     }
//!     return Err("Blocked by conflicts");
//! }
//!
//! // Warn about other users
//! for user in &check.other_users {
//!     eprintln!("Warning: {} is also accessing this resource", user.user_id);
//! }
//!
//! // Acquire lock
//! let request = LockRequest::new("my-project/production", "user@example.com", "deploy")
//!     .with_ttl(Duration::minutes(30))
//!     .with_message("Deploying new API version");
//!
//! let result = lock_manager.acquire(request).await?;
//!
//! match result {
//!     LockResult::Acquired { info } => {
//!         println!("Lock acquired: {}", info.id);
//!         
//!         // Do the deployment...
//!         
//!         // Release when done
//!         lock_manager.release("my-project/production", &info.id, "user@example.com").await?;
//!     }
//!     LockResult::Denied { holder } => {
//!         eprintln!(
//!             "Resource is locked by {} ({})",
//!             holder.holder_display(),
//!             holder.operation
//!         );
//!     }
//!     LockResult::Queued { entry } => {
//!         println!("You are #{} in the queue", entry.position);
//!     }
//!     _ => {}
//! }
//! ```
//!
//! ## Lock Types and Behavior
//!
//! ### Standard Lock
//! - Exclusive access to a resource
//! - TTL-based expiration (default 30 minutes)
//! - Must be explicitly released or will expire
//!
//! ### Queued Lock
//! - If resource is locked, request can be queued
//! - Fair ordering (FIFO)
//! - Automatic promotion when lock is released
//!
//! ### Force Lock (Admin)
//! - Breaks existing lock
//! - Logged in audit journal
//! - Use with caution
//!
//! ## Heartbeat
//!
//! For long-running operations, use heartbeat to prevent lock expiration:
//!
//! ```rust,ignore
//! // Extend lock TTL periodically
//! lock_manager.heartbeat("my-project/production", &lock_id, "user@example.com").await?;
//! ```

pub mod conflict;
pub mod error;
pub mod journal;
pub mod lock;
pub mod session;

// Re-export main types
pub use error::{ConcurrencyError, Result};

// Lock types
pub use lock::{
    InMemoryLockBackend, LockBackend, LockId, LockInfo, LockManager, LockManagerConfig,
    LockRequest, LockResult, LockStatus, QueueEntry,
};

// Session types
pub use session::{
    ClientInfo, InMemorySessionBackend, SessionBackend, SessionId, SessionInfo, SessionManager,
};

// Conflict types
pub use conflict::{
    Conflict, ConflictDetector, ConflictSeverity, ConflictType, OtherUser, PreOperationCheck,
};

// Journal types
pub use journal::{
    InMemoryJournal, JournalEntry, JournalEventType, JournalQuery, OperationJournal,
};
