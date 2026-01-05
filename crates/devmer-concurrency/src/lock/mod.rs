//! Distributed locking for concurrent operation control.

mod manager;
mod types;

pub use manager::{InMemoryLockBackend, LockBackend, LockManager, LockManagerConfig};
pub use types::{LockId, LockInfo, LockRequest, LockResult, LockStatus, QueueEntry};
