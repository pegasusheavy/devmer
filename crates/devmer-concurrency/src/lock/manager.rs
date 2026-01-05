//! Distributed lock manager.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{Duration, Utc};
use tokio::sync::{Mutex, RwLock};

use crate::error::{ConcurrencyError, Result};
use crate::journal::{JournalEntry, JournalEventType, OperationJournal};

use super::types::{LockId, LockInfo, LockRequest, LockResult, LockStatus, QueueEntry};

/// Trait for lock storage backends.
#[async_trait]
pub trait LockBackend: Send + Sync {
    /// Get lock for a resource.
    async fn get_lock(&self, resource: &str) -> Result<Option<LockInfo>>;

    /// Store a lock.
    async fn store_lock(&self, info: &LockInfo) -> Result<()>;

    /// Remove a lock.
    async fn remove_lock(&self, resource: &str, lock_id: &LockId) -> Result<()>;

    /// List all locks.
    async fn list_locks(&self) -> Result<Vec<LockInfo>>;

    /// Get queue for a resource.
    async fn get_queue(&self, resource: &str) -> Result<VecDeque<LockRequest>>;

    /// Add to queue.
    async fn enqueue(&self, resource: &str, request: LockRequest) -> Result<u32>;

    /// Remove from queue.
    async fn dequeue(&self, resource: &str) -> Result<Option<LockRequest>>;

    /// Remove specific request from queue.
    async fn remove_from_queue(&self, resource: &str, holder: &str) -> Result<bool>;
}

/// In-memory lock backend for single-instance deployments and testing.
pub struct InMemoryLockBackend {
    locks: RwLock<HashMap<String, LockInfo>>,
    queues: RwLock<HashMap<String, VecDeque<LockRequest>>>,
}

impl InMemoryLockBackend {
    /// Create a new in-memory backend.
    pub fn new() -> Self {
        Self {
            locks: RwLock::new(HashMap::new()),
            queues: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemoryLockBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LockBackend for InMemoryLockBackend {
    async fn get_lock(&self, resource: &str) -> Result<Option<LockInfo>> {
        let locks = self.locks.read().await;
        Ok(locks.get(resource).cloned())
    }

    async fn store_lock(&self, info: &LockInfo) -> Result<()> {
        let mut locks = self.locks.write().await;
        locks.insert(info.resource.clone(), info.clone());
        Ok(())
    }

    async fn remove_lock(&self, resource: &str, lock_id: &LockId) -> Result<()> {
        let mut locks = self.locks.write().await;
        if let Some(info) = locks.get(resource) {
            if &info.id == lock_id {
                locks.remove(resource);
            }
        }
        Ok(())
    }

    async fn list_locks(&self) -> Result<Vec<LockInfo>> {
        let locks = self.locks.read().await;
        Ok(locks.values().cloned().collect())
    }

    async fn get_queue(&self, resource: &str) -> Result<VecDeque<LockRequest>> {
        let queues = self.queues.read().await;
        Ok(queues.get(resource).cloned().unwrap_or_default())
    }

    async fn enqueue(&self, resource: &str, request: LockRequest) -> Result<u32> {
        let mut queues = self.queues.write().await;
        let queue = queues.entry(resource.to_string()).or_default();
        queue.push_back(request);
        Ok(queue.len() as u32)
    }

    async fn dequeue(&self, resource: &str) -> Result<Option<LockRequest>> {
        let mut queues = self.queues.write().await;
        if let Some(queue) = queues.get_mut(resource) {
            return Ok(queue.pop_front());
        }
        Ok(None)
    }

    async fn remove_from_queue(&self, resource: &str, holder: &str) -> Result<bool> {
        let mut queues = self.queues.write().await;
        if let Some(queue) = queues.get_mut(resource) {
            let len_before = queue.len();
            queue.retain(|r| r.holder != holder);
            return Ok(queue.len() < len_before);
        }
        Ok(false)
    }
}

/// Configuration for the lock manager.
#[derive(Debug, Clone)]
pub struct LockManagerConfig {
    /// Default lock TTL.
    pub default_ttl: Duration,
    /// Heartbeat interval.
    pub heartbeat_interval: Duration,
    /// Maximum queue size per resource.
    pub max_queue_size: usize,
    /// Auto-cleanup expired locks.
    pub auto_cleanup: bool,
    /// Cleanup interval.
    pub cleanup_interval: Duration,
}

impl Default for LockManagerConfig {
    fn default() -> Self {
        Self {
            default_ttl: Duration::minutes(30),
            heartbeat_interval: Duration::minutes(5),
            max_queue_size: 100,
            auto_cleanup: true,
            cleanup_interval: Duration::minutes(1),
        }
    }
}

/// Distributed lock manager.
///
/// Ensures only one operation can run at a time on a given resource.
/// Supports queuing, heartbeats, and automatic cleanup.
pub struct LockManager {
    backend: Arc<dyn LockBackend>,
    config: LockManagerConfig,
    journal: Option<Arc<dyn OperationJournal>>,
    // For coordinating waiters
    waiters: Mutex<HashMap<String, Vec<tokio::sync::oneshot::Sender<()>>>>,
}

impl LockManager {
    /// Create a new lock manager.
    pub fn new(backend: Arc<dyn LockBackend>, config: LockManagerConfig) -> Self {
        Self {
            backend,
            config,
            journal: None,
            waiters: Mutex::new(HashMap::new()),
        }
    }

    /// Create with in-memory backend.
    pub fn in_memory() -> Self {
        Self::new(Arc::new(InMemoryLockBackend::new()), LockManagerConfig::default())
    }

    /// Set journal for audit logging.
    pub fn with_journal(mut self, journal: Arc<dyn OperationJournal>) -> Self {
        self.journal = Some(journal);
        self
    }

    /// Get the status of a lock on a resource.
    pub async fn status(&self, resource: &str, requester: &str) -> Result<LockStatus> {
        let lock = self.backend.get_lock(resource).await?;

        match lock {
            None => Ok(LockStatus::Available),
            Some(info) => {
                if info.is_expired() {
                    Ok(LockStatus::Expired { info })
                } else if info.holder == requester {
                    Ok(LockStatus::LockedByYou { info })
                } else {
                    Ok(LockStatus::LockedByOther { info })
                }
            }
        }
    }

    /// Attempt to acquire a lock.
    pub async fn acquire(&self, request: LockRequest) -> Result<LockResult> {
        let status = self.status(&request.resource, &request.holder).await?;

        match status {
            LockStatus::Available | LockStatus::Expired { .. } => {
                // Clean up expired lock if any
                if let LockStatus::Expired { info } = &status {
                    self.backend.remove_lock(&request.resource, &info.id).await?;
                }

                // Create and store the lock
                let info = self.create_lock_info(&request);
                self.backend.store_lock(&info).await?;

                // Log to journal
                if let Some(journal) = &self.journal {
                    journal.log(JournalEntry::new(
                        &request.resource,
                        &request.holder,
                        JournalEventType::LockAcquired,
                    ).with_message(format!("Operation: {}", request.operation))).await?;
                }

                tracing::info!(
                    resource = %request.resource,
                    holder = %request.holder,
                    operation = %request.operation,
                    lock_id = %info.id,
                    "Lock acquired"
                );

                Ok(LockResult::Acquired { info })
            }

            LockStatus::LockedByYou { info } => {
                // Already have the lock - extend it
                let mut updated = info.clone();
                updated.refresh_heartbeat(Some(request.ttl));
                self.backend.store_lock(&updated).await?;

                Ok(LockResult::Acquired { info: updated })
            }

            LockStatus::LockedByOther { info } => {
                if request.force {
                    // Force acquire - break the existing lock
                    let previous = info.clone();
                    self.backend.remove_lock(&request.resource, &info.id).await?;

                    let mut new_info = self.create_lock_info(&request);
                    new_info.force_acquired = true;
                    new_info.previous_holder = Some(info.holder.clone());
                    self.backend.store_lock(&new_info).await?;

                    // Log force acquisition
                    if let Some(journal) = &self.journal {
                        journal.log(JournalEntry::new(
                            &request.resource,
                            &request.holder,
                            JournalEventType::LockForceAcquired,
                        ).with_message(format!(
                            "Broke lock from {} (operation: {})",
                            previous.holder, previous.operation
                        ))).await?;
                    }

                    tracing::warn!(
                        resource = %request.resource,
                        new_holder = %request.holder,
                        previous_holder = %previous.holder,
                        "Lock force-acquired"
                    );

                    Ok(LockResult::ForceAcquired {
                        info: new_info,
                        previous,
                    })
                } else if request.wait {
                    // Add to queue
                    let position = self.backend.enqueue(&request.resource, request.clone()).await?;

                    // Estimate wait time based on queue position and default TTL
                    let estimated_wait = Duration::seconds(
                        (position as i64) * self.config.default_ttl.num_seconds() / 2
                    );

                    let entry = QueueEntry {
                        position,
                        request: request.clone(),
                        queued_at: Utc::now(),
                        estimated_wait: Some(estimated_wait),
                    };

                    tracing::info!(
                        resource = %request.resource,
                        holder = %request.holder,
                        position = %position,
                        "Added to lock queue"
                    );

                    Ok(LockResult::Queued { entry })
                } else {
                    Ok(LockResult::Denied { holder: info })
                }
            }
        }
    }

    /// Wait for a lock to become available.
    pub async fn acquire_with_wait(&self, request: LockRequest) -> Result<LockResult> {
        let timeout = request.wait_timeout.unwrap_or(Duration::minutes(10));
        let deadline = Utc::now() + timeout;

        loop {
            let result = self.acquire(request.clone()).await?;

            match result {
                LockResult::Acquired { .. } | LockResult::ForceAcquired { .. } => {
                    return Ok(result);
                }
                LockResult::Denied { ref holder } => {
                    if Utc::now() >= deadline {
                        return Err(ConcurrencyError::LockTimeout {
                            resource: request.resource.clone(),
                            timeout_secs: timeout.num_seconds() as u64,
                        });
                    }

                    // Wait for notification or poll
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    {
                        let mut waiters = self.waiters.lock().await;
                        waiters
                            .entry(request.resource.clone())
                            .or_default()
                            .push(tx);
                    }

                    // Wait with timeout
                    let remaining = (deadline - Utc::now()).to_std().unwrap_or_default();
                    let _ = tokio::time::timeout(
                        remaining.min(std::time::Duration::from_secs(5)),
                        rx,
                    ).await;
                }
                LockResult::Queued { .. } => {
                    // Already queued, wait for our turn
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    {
                        let mut waiters = self.waiters.lock().await;
                        waiters
                            .entry(request.resource.clone())
                            .or_default()
                            .push(tx);
                    }

                    let remaining = (deadline - Utc::now()).to_std().unwrap_or_default();
                    if remaining.is_zero() {
                        // Remove from queue
                        self.backend.remove_from_queue(&request.resource, &request.holder).await?;
                        return Err(ConcurrencyError::LockTimeout {
                            resource: request.resource.clone(),
                            timeout_secs: timeout.num_seconds() as u64,
                        });
                    }

                    let _ = tokio::time::timeout(
                        remaining.min(std::time::Duration::from_secs(5)),
                        rx,
                    ).await;
                }
            }
        }
    }

    /// Release a lock.
    pub async fn release(&self, resource: &str, lock_id: &LockId, holder: &str) -> Result<()> {
        let current = self.backend.get_lock(resource).await?;

        match current {
            None => {
                return Err(ConcurrencyError::LockNotFound(resource.to_string()));
            }
            Some(info) => {
                if &info.id != lock_id {
                    return Err(ConcurrencyError::LockNotFound(lock_id.to_string()));
                }
                if info.holder != holder {
                    return Err(ConcurrencyError::InvalidLockOwner {
                        owner: info.holder.clone(),
                        requester: holder.to_string(),
                    });
                }

                self.backend.remove_lock(resource, lock_id).await?;

                // Log release
                if let Some(journal) = &self.journal {
                    journal.log(JournalEntry::new(
                        resource,
                        holder,
                        JournalEventType::LockReleased,
                    )).await?;
                }

                tracing::info!(
                    resource = %resource,
                    holder = %holder,
                    lock_id = %lock_id,
                    "Lock released"
                );

                // Notify waiters
                self.notify_waiters(resource).await;

                // Check queue for next waiter
                if let Some(next) = self.backend.dequeue(resource).await? {
                    tracing::debug!(
                        resource = %resource,
                        next_holder = %next.holder,
                        "Next in queue notified"
                    );
                }
            }
        }

        Ok(())
    }

    /// Renew a lock's TTL (heartbeat).
    pub async fn heartbeat(&self, resource: &str, lock_id: &LockId, holder: &str) -> Result<LockInfo> {
        let current = self.backend.get_lock(resource).await?;

        match current {
            None => Err(ConcurrencyError::LockNotFound(resource.to_string())),
            Some(mut info) => {
                if &info.id != lock_id {
                    return Err(ConcurrencyError::LockNotFound(lock_id.to_string()));
                }
                if info.holder != holder {
                    return Err(ConcurrencyError::InvalidLockOwner {
                        owner: info.holder.clone(),
                        requester: holder.to_string(),
                    });
                }

                info.refresh_heartbeat(Some(self.config.default_ttl));
                self.backend.store_lock(&info).await?;

                tracing::debug!(
                    resource = %resource,
                    holder = %holder,
                    new_expiry = %info.expires_at,
                    "Lock heartbeat"
                );

                Ok(info)
            }
        }
    }

    /// Force-release a lock (admin operation).
    pub async fn force_release(&self, resource: &str, admin: &str, reason: &str) -> Result<Option<LockInfo>> {
        let current = self.backend.get_lock(resource).await?;

        match current {
            None => Ok(None),
            Some(info) => {
                self.backend.remove_lock(resource, &info.id).await?;

                // Log force release
                if let Some(journal) = &self.journal {
                    journal.log(JournalEntry::new(
                        resource,
                        admin,
                        JournalEventType::LockForceReleased,
                    ).with_message(format!(
                        "Released lock from {} (reason: {})",
                        info.holder, reason
                    ))).await?;
                }

                tracing::warn!(
                    resource = %resource,
                    admin = %admin,
                    previous_holder = %info.holder,
                    reason = %reason,
                    "Lock force-released"
                );

                // Notify waiters
                self.notify_waiters(resource).await;

                Ok(Some(info))
            }
        }
    }

    /// List all active locks.
    pub async fn list_locks(&self) -> Result<Vec<LockInfo>> {
        self.backend.list_locks().await
    }

    /// Get queue for a resource.
    pub async fn get_queue(&self, resource: &str) -> Result<Vec<QueueEntry>> {
        let queue = self.backend.get_queue(resource).await?;
        Ok(queue
            .into_iter()
            .enumerate()
            .map(|(i, request)| QueueEntry {
                position: (i + 1) as u32,
                request: request.clone(),
                queued_at: Utc::now(), // Approximate
                estimated_wait: None,
            })
            .collect())
    }

    /// Clean up expired locks.
    pub async fn cleanup_expired(&self) -> Result<Vec<LockInfo>> {
        let locks = self.backend.list_locks().await?;
        let mut cleaned = Vec::new();

        for lock in locks {
            if lock.is_expired() {
                self.backend.remove_lock(&lock.resource, &lock.id).await?;
                
                tracing::info!(
                    resource = %lock.resource,
                    holder = %lock.holder,
                    expired_at = %lock.expires_at,
                    "Cleaned up expired lock"
                );

                // Notify waiters
                self.notify_waiters(&lock.resource).await;

                cleaned.push(lock);
            }
        }

        Ok(cleaned)
    }

    // Internal helpers

    fn create_lock_info(&self, request: &LockRequest) -> LockInfo {
        let mut info = LockInfo::new(&request.resource, &request.holder, &request.operation)
            .with_ttl(request.ttl);

        if let Some(name) = &request.holder_name {
            info = info.with_holder_name(name);
        }
        if let Some(msg) = &request.message {
            info = info.with_message(msg);
        }

        info
    }

    async fn notify_waiters(&self, resource: &str) {
        let mut waiters = self.waiters.lock().await;
        if let Some(senders) = waiters.remove(resource) {
            for tx in senders {
                let _ = tx.send(());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_acquire_and_release() {
        let manager = LockManager::in_memory();

        let request = LockRequest::new("project/stack", "user1", "deploy");
        let result = manager.acquire(request).await.unwrap();

        assert!(result.is_acquired());
        let info = result.lock_info().unwrap();

        // Verify status
        let status = manager.status("project/stack", "user1").await.unwrap();
        assert!(matches!(status, LockStatus::LockedByYou { .. }));

        // Release
        manager.release("project/stack", &info.id, "user1").await.unwrap();

        // Verify unlocked
        let status = manager.status("project/stack", "user1").await.unwrap();
        assert!(matches!(status, LockStatus::Available));
    }

    #[tokio::test]
    async fn test_lock_denied() {
        let manager = LockManager::in_memory();

        // User1 acquires
        let req1 = LockRequest::new("project/stack", "user1", "deploy");
        let result1 = manager.acquire(req1).await.unwrap();
        assert!(result1.is_acquired());

        // User2 tries to acquire
        let req2 = LockRequest::new("project/stack", "user2", "deploy");
        let result2 = manager.acquire(req2).await.unwrap();
        assert!(matches!(result2, LockResult::Denied { .. }));

        // Verify status for user2
        let status = manager.status("project/stack", "user2").await.unwrap();
        assert!(matches!(status, LockStatus::LockedByOther { .. }));
    }

    #[tokio::test]
    async fn test_force_acquire() {
        let manager = LockManager::in_memory();

        // User1 acquires
        let req1 = LockRequest::new("project/stack", "user1", "deploy");
        manager.acquire(req1).await.unwrap();

        // User2 force acquires
        let req2 = LockRequest::new("project/stack", "user2", "emergency").force();
        let result = manager.acquire(req2).await.unwrap();

        assert!(matches!(result, LockResult::ForceAcquired { .. }));
        
        if let LockResult::ForceAcquired { info, previous } = result {
            assert_eq!(info.holder, "user2");
            assert_eq!(previous.holder, "user1");
            assert!(info.force_acquired);
        }
    }

    #[tokio::test]
    async fn test_heartbeat() {
        let manager = LockManager::in_memory();

        let request = LockRequest::new("project/stack", "user1", "deploy")
            .with_ttl(Duration::seconds(10));
        let result = manager.acquire(request).await.unwrap();
        let info = result.lock_info().unwrap().clone();

        // Wait a bit
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Heartbeat
        let renewed = manager.heartbeat("project/stack", &info.id, "user1").await.unwrap();
        assert!(renewed.expires_at > info.expires_at);
    }

    #[tokio::test]
    async fn test_invalid_release() {
        let manager = LockManager::in_memory();

        let request = LockRequest::new("project/stack", "user1", "deploy");
        let result = manager.acquire(request).await.unwrap();
        let info = result.lock_info().unwrap();

        // User2 tries to release user1's lock
        let err = manager.release("project/stack", &info.id, "user2").await;
        assert!(matches!(err, Err(ConcurrencyError::InvalidLockOwner { .. })));
    }
}
