//! Cloudmer-backed distributed locking.
//!
//! This module provides a distributed lock backend that uses Cloudmer
//! for coordination across multiple machines/users.
//!
//! ## When to Use Cloudmer Locking
//!
//! - **Single user, single machine**: Use local locking (default)
//! - **Single user, multiple machines**: Use state backend locking (S3, GCS, etc.)
//! - **Multiple users, any machines**: Use Cloudmer locking (this module)
//!
//! ## Fallback Behavior
//!
//! If Cloudmer is not configured, operations gracefully fall back to
//! local/state-backend locking. You won't get distributed coordination,
//! but everything still works for single-user scenarios.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::client::CloudmerClient;
use crate::error::{CloudmerError, Result};

/// Lock info stored in Cloudmer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudmerLock {
    /// Lock ID.
    pub lock_id: String,
    /// Resource being locked (project/stack).
    pub resource: String,
    /// Who holds the lock.
    pub holder_id: String,
    /// Holder display name.
    pub holder_name: Option<String>,
    /// Operation being performed.
    pub operation: String,
    /// When acquired.
    pub acquired_at: DateTime<Utc>,
    /// When it expires.
    pub expires_at: DateTime<Utc>,
    /// Last heartbeat.
    pub last_heartbeat: DateTime<Utc>,
    /// Machine hostname.
    pub hostname: Option<String>,
    /// Additional message.
    pub message: Option<String>,
}

/// Request to acquire a lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcquireLockRequest {
    /// Resource to lock.
    pub resource: String,
    /// Requester ID.
    pub holder_id: String,
    /// Requester name.
    pub holder_name: Option<String>,
    /// Operation.
    pub operation: String,
    /// Requested TTL in seconds.
    pub ttl_secs: u64,
    /// Wait for lock if not available.
    pub wait: bool,
    /// Max wait time in seconds.
    pub wait_timeout_secs: Option<u64>,
    /// Force acquire (admin only).
    pub force: bool,
    /// Message.
    pub message: Option<String>,
}

impl AcquireLockRequest {
    /// Create a new acquire request.
    pub fn new(
        resource: impl Into<String>,
        holder_id: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        Self {
            resource: resource.into(),
            holder_id: holder_id.into(),
            holder_name: None,
            operation: operation.into(),
            ttl_secs: 30 * 60, // 30 minutes
            wait: false,
            wait_timeout_secs: None,
            force: false,
            message: None,
        }
    }

    /// Set holder name.
    pub fn with_holder_name(mut self, name: impl Into<String>) -> Self {
        self.holder_name = Some(name.into());
        self
    }

    /// Set TTL.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl_secs = ttl.num_seconds() as u64;
        self
    }

    /// Wait for lock.
    pub fn wait_for(mut self, timeout: Duration) -> Self {
        self.wait = true;
        self.wait_timeout_secs = Some(timeout.num_seconds() as u64);
        self
    }

    /// Force acquire (admin).
    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }

    /// Set message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }
}

/// Response from lock acquisition.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum AcquireLockResponse {
    /// Lock acquired.
    #[serde(rename = "acquired")]
    Acquired {
        lock: CloudmerLock,
    },
    /// Added to queue.
    #[serde(rename = "queued")]
    Queued {
        position: u32,
        estimated_wait_secs: Option<u64>,
    },
    /// Lock denied - held by another user.
    #[serde(rename = "denied")]
    Denied {
        current_holder: CloudmerLock,
    },
    /// Force acquired (broke another lock).
    #[serde(rename = "force_acquired")]
    ForceAcquired {
        lock: CloudmerLock,
        previous_holder: CloudmerLock,
    },
}

/// Lock status check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "status")]
pub enum LockStatusResponse {
    /// No lock exists.
    #[serde(rename = "available")]
    Available,
    /// Locked by the requester.
    #[serde(rename = "locked_by_you")]
    LockedByYou {
        lock: CloudmerLock,
    },
    /// Locked by another user.
    #[serde(rename = "locked_by_other")]
    LockedByOther {
        lock: CloudmerLock,
    },
}

/// Cloudmer locking client.
pub struct CloudmerLockingClient {
    client: CloudmerClient,
}

impl CloudmerLockingClient {
    /// Create a new locking client.
    pub fn new(client: CloudmerClient) -> Self {
        Self { client }
    }

    /// Check if a resource is locked.
    pub async fn status(&self, _resource: &str, _requester_id: &str) -> Result<LockStatusResponse> {
        let _project_id = self.client.config().project_id.as_ref()
            .ok_or(CloudmerError::ProjectNotLinked)?;

        // This would be an actual API call to Cloudmer
        // For now, return available
        Ok(LockStatusResponse::Available)
    }

    /// Acquire a lock.
    pub async fn acquire(&self, request: &AcquireLockRequest) -> Result<AcquireLockResponse> {
        let _project_id = self.client.config().project_id.as_ref()
            .ok_or(CloudmerError::ProjectNotLinked)?;

        // This would be an actual API call to Cloudmer
        let lock = CloudmerLock {
            lock_id: uuid::Uuid::new_v4().to_string(),
            resource: request.resource.clone(),
            holder_id: request.holder_id.clone(),
            holder_name: request.holder_name.clone(),
            operation: request.operation.clone(),
            acquired_at: Utc::now(),
            expires_at: Utc::now() + Duration::seconds(request.ttl_secs as i64),
            last_heartbeat: Utc::now(),
            hostname: None, // Would be populated by Cloudmer
            message: request.message.clone(),
        };

        Ok(AcquireLockResponse::Acquired { lock })
    }

    /// Release a lock.
    pub async fn release(&self, _resource: &str, _lock_id: &str, _holder_id: &str) -> Result<()> {
        let _project_id = self.client.config().project_id.as_ref()
            .ok_or(CloudmerError::ProjectNotLinked)?;

        // This would be an actual API call to Cloudmer
        Ok(())
    }

    /// Heartbeat to renew a lock.
    pub async fn heartbeat(&self, resource: &str, lock_id: &str, holder_id: &str) -> Result<CloudmerLock> {
        let _project_id = self.client.config().project_id.as_ref()
            .ok_or(CloudmerError::ProjectNotLinked)?;

        // This would be an actual API call to Cloudmer
        let lock = CloudmerLock {
            lock_id: lock_id.to_string(),
            resource: resource.to_string(),
            holder_id: holder_id.to_string(),
            holder_name: None,
            operation: "renewed".to_string(),
            acquired_at: Utc::now(),
            expires_at: Utc::now() + Duration::minutes(30),
            last_heartbeat: Utc::now(),
            hostname: None,
            message: None,
        };

        Ok(lock)
    }

    /// List all active locks for a project.
    pub async fn list(&self) -> Result<Vec<CloudmerLock>> {
        let _project_id = self.client.config().project_id.as_ref()
            .ok_or(CloudmerError::ProjectNotLinked)?;

        // This would be an actual API call to Cloudmer
        Ok(Vec::new())
    }

    /// Get the lock queue for a resource.
    pub async fn queue(&self, _resource: &str) -> Result<Vec<QueuedLockRequest>> {
        let _project_id = self.client.config().project_id.as_ref()
            .ok_or(CloudmerError::ProjectNotLinked)?;

        // This would be an actual API call to Cloudmer
        Ok(Vec::new())
    }

    /// Force release a lock (admin operation).
    pub async fn force_release(&self, _resource: &str, _reason: &str) -> Result<Option<CloudmerLock>> {
        let _project_id = self.client.config().project_id.as_ref()
            .ok_or(CloudmerError::ProjectNotLinked)?;

        // This would be an actual API call to Cloudmer
        Ok(None)
    }
}

/// A queued lock request.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueuedLockRequest {
    /// Position in queue (1-based).
    pub position: u32,
    /// Requester ID.
    pub holder_id: String,
    /// Requester name.
    pub holder_name: Option<String>,
    /// Requested operation.
    pub operation: String,
    /// When queued.
    pub queued_at: DateTime<Utc>,
    /// Estimated wait time.
    pub estimated_wait_secs: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquire_request() {
        let request = AcquireLockRequest::new(
            "my-project/production",
            "user@example.com",
            "deploy",
        )
        .with_holder_name("Alice")
        .with_ttl(Duration::minutes(60));

        assert_eq!(request.resource, "my-project/production");
        assert_eq!(request.holder_id, "user@example.com");
        assert_eq!(request.ttl_secs, 3600);
    }
}
