//! State backend trait

use crate::locking::{LockId, LockInfo, LockStatus};
use crate::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use devmer_core::state::StackState;
use serde::{Deserialize, Serialize};

/// History entry for state changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateHistory {
    /// Version number
    pub version: u64,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Who made the change
    pub actor: Option<String>,

    /// Operation type
    pub operation: String,

    /// Message/description
    pub message: Option<String>,

    /// Checksum of the state
    pub checksum: String,
}

/// Trait for state storage backends
#[async_trait]
pub trait StateBackend: Send + Sync {
    /// Get the backend name
    fn name(&self) -> &str;

    /// Get state for a stack
    async fn get_state(&self, project: &str, stack: &str) -> Result<Option<StackState>>;

    /// Save state for a stack
    async fn save_state(&self, project: &str, stack: &str, state: &StackState) -> Result<()>;

    /// Delete state for a stack
    async fn delete_state(&self, project: &str, stack: &str) -> Result<()>;

    /// List all stacks for a project
    async fn list_stacks(&self, project: &str) -> Result<Vec<String>>;

    /// Acquire a lock on a stack
    async fn lock(&self, project: &str, stack: &str, info: LockInfo) -> Result<LockId>;

    /// Release a lock
    async fn unlock(&self, project: &str, stack: &str, lock_id: &LockId) -> Result<()>;

    /// Get current lock status
    async fn get_lock_status(&self, project: &str, stack: &str) -> Result<LockStatus>;

    /// Force unlock (admin operation)
    async fn force_unlock(&self, project: &str, stack: &str) -> Result<()>;

    /// Get state history
    async fn get_history(
        &self,
        project: &str,
        stack: &str,
        limit: Option<u32>,
    ) -> Result<Vec<StateHistory>>;

    /// Get a specific historical state version
    async fn get_state_version(
        &self,
        project: &str,
        stack: &str,
        version: u64,
    ) -> Result<Option<StackState>>;
}

/// A locked state guard that releases the lock on drop
pub struct LockedState<'a, B: StateBackend> {
    backend: &'a B,
    project: String,
    stack: String,
    lock_id: LockId,
    released: bool,
}

impl<'a, B: StateBackend> LockedState<'a, B> {
    /// Create a new locked state guard
    pub fn new(backend: &'a B, project: String, stack: String, lock_id: LockId) -> Self {
        Self {
            backend,
            project,
            stack,
            lock_id,
            released: false,
        }
    }

    /// Get the lock ID
    pub fn lock_id(&self) -> &LockId {
        &self.lock_id
    }

    /// Release the lock explicitly
    pub async fn release(mut self) -> Result<()> {
        self.released = true;
        self.backend
            .unlock(&self.project, &self.stack, &self.lock_id)
            .await
    }
}

impl<B: StateBackend> Drop for LockedState<'_, B> {
    fn drop(&mut self) {
        if !self.released {
            // Can't do async in drop, so just log a warning
            tracing::warn!(
                "LockedState dropped without explicit release for {}/{}",
                self.project,
                self.stack
            );
        }
    }
}
