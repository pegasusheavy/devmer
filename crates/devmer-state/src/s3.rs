//! S3 State Backend
//!
//! This module provides an S3-based state backend for Devmer.
//! State is stored in an S3 bucket with DynamoDB for locking.

use crate::backend::{StateBackend, StateHistory};
use crate::error::StateError;
use crate::locking::{LockId, LockInfo, LockStatus};
use crate::Result;
use async_trait::async_trait;
use devmer_core::state::StackState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info, warn};

/// S3 backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3BackendConfig {
    /// S3 bucket name
    pub bucket: String,

    /// Key prefix for state files
    #[serde(default = "default_prefix")]
    pub prefix: String,

    /// AWS region
    #[serde(default = "default_region")]
    pub region: String,

    /// DynamoDB table for locking (optional)
    pub lock_table: Option<String>,

    /// Enable server-side encryption
    #[serde(default)]
    pub encrypt: bool,

    /// KMS key ID for encryption
    pub kms_key_id: Option<String>,

    /// ACL for uploaded objects
    #[serde(default = "default_acl")]
    pub acl: String,

    /// Custom endpoint (for MinIO, LocalStack, etc.)
    pub endpoint: Option<String>,

    /// Use path-style addressing (for MinIO)
    #[serde(default)]
    pub force_path_style: bool,

    /// AWS profile to use
    pub profile: Option<String>,

    /// Skip credential validation
    #[serde(default)]
    pub skip_credentials_validation: bool,
}

fn default_prefix() -> String {
    "devmer/".to_string()
}

fn default_region() -> String {
    "us-east-1".to_string()
}

fn default_acl() -> String {
    "private".to_string()
}

impl Default for S3BackendConfig {
    fn default() -> Self {
        Self {
            bucket: String::new(),
            prefix: default_prefix(),
            region: default_region(),
            lock_table: None,
            encrypt: false,
            kms_key_id: None,
            acl: default_acl(),
            endpoint: None,
            force_path_style: false,
            profile: None,
            skip_credentials_validation: false,
        }
    }
}

impl S3BackendConfig {
    /// Create config with bucket name
    pub fn new(bucket: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
            ..Default::default()
        }
    }

    /// Builder: set region
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = region.into();
        self
    }

    /// Builder: set prefix
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Builder: set lock table
    pub fn with_lock_table(mut self, table: impl Into<String>) -> Self {
        self.lock_table = Some(table.into());
        self
    }

    /// Builder: enable encryption
    pub fn with_encryption(mut self, kms_key_id: Option<String>) -> Self {
        self.encrypt = true;
        self.kms_key_id = kms_key_id;
        self
    }

    /// Builder: set custom endpoint
    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.endpoint = Some(endpoint.into());
        self.force_path_style = true;
        self
    }
}

/// S3 state backend
///
/// Stores infrastructure state in S3 with optional DynamoDB locking.
///
/// # State Path Convention
/// State files are stored at: `{prefix}{project}/{stack}.json`
///
/// # Locking
/// When a DynamoDB lock table is configured, exclusive locks are acquired
/// before modifying state.
pub struct S3Backend {
    config: S3BackendConfig,
    project: String,
    // In a real implementation, this would be AWS SDK clients
    // For now, we use an in-memory mock for testing
    mock_storage: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    mock_locks: Arc<RwLock<HashMap<String, LockInfo>>>,
}

impl S3Backend {
    /// Create a new S3 backend
    pub fn new(project: impl Into<String>, config: S3BackendConfig) -> Self {
        info!(
            bucket = %config.bucket,
            region = %config.region,
            prefix = %config.prefix,
            "Initializing S3 state backend"
        );

        Self {
            config,
            project: project.into(),
            mock_storage: Arc::new(RwLock::new(HashMap::new())),
            mock_locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get the S3 key for a stack
    fn state_key(&self, project: &str, stack: &str) -> String {
        format!("{}{}/{}.json", self.config.prefix, project, stack)
    }
}

#[async_trait]
impl StateBackend for S3Backend {
    fn name(&self) -> &str {
        "s3"
    }

    async fn get_state(&self, project: &str, stack: &str) -> Result<Option<StackState>> {
        let key = self.state_key(project, stack);
        debug!(key = %key, bucket = %self.config.bucket, "Getting state from S3");

        let storage = self.mock_storage.read().map_err(|_| StateError::LockPoisoned)?;
        match storage.get(&key) {
            Some(data) => {
                let state: StackState = serde_json::from_slice(data)
                    .map_err(|e| StateError::Deserialize(e.to_string()))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    async fn save_state(&self, project: &str, stack: &str, state: &StackState) -> Result<()> {
        let key = self.state_key(project, stack);
        debug!(
            key = %key,
            bucket = %self.config.bucket,
            encrypt = %self.config.encrypt,
            "Putting state to S3"
        );

        let data = serde_json::to_vec_pretty(state)
            .map_err(|e| StateError::Serialize(e.to_string()))?;

        let mut storage = self.mock_storage.write().map_err(|_| StateError::LockPoisoned)?;
        storage.insert(key, data);

        info!(project = %project, stack = %stack, "State saved to S3");
        Ok(())
    }

    async fn delete_state(&self, project: &str, stack: &str) -> Result<()> {
        let key = self.state_key(project, stack);
        debug!(key = %key, bucket = %self.config.bucket, "Deleting state from S3");

        let mut storage = self.mock_storage.write().map_err(|_| StateError::LockPoisoned)?;
        storage.remove(&key);

        info!(project = %project, stack = %stack, "State deleted from S3");
        Ok(())
    }

    async fn list_stacks(&self, project: &str) -> Result<Vec<String>> {
        let prefix = format!("{}{}/", self.config.prefix, project);
        debug!(prefix = %prefix, bucket = %self.config.bucket, "Listing stacks in S3");

        let storage = self.mock_storage.read().map_err(|_| StateError::LockPoisoned)?;
        let stacks: Vec<String> = storage
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .filter_map(|k| {
                k.strip_prefix(&prefix)
                    .and_then(|s| s.strip_suffix(".json"))
                    .map(|s| s.to_string())
            })
            .collect();

        Ok(stacks)
    }

    async fn lock(&self, project: &str, stack: &str, info: LockInfo) -> Result<LockId> {
        if self.config.lock_table.is_none() {
            warn!("S3 backend has no lock table configured - locking is disabled");
            return Ok(LockId::new());
        }

        let key = format!("{}/{}", project, stack);
        debug!(
            key = %key,
            lock_table = ?self.config.lock_table,
            "Acquiring lock in DynamoDB"
        );

        let mut locks = self.mock_locks.write().map_err(|_| StateError::LockPoisoned)?;

        if let Some(existing) = locks.get(&key) {
            return Err(StateError::LockConflict {
                owner: existing.owner.clone(),
                operation: existing.operation.clone(),
                acquired_at: existing.created_at,
            });
        }

        let lock_id = info.id.clone();
        locks.insert(key, info);

        info!(project = %project, stack = %stack, lock_id = %lock_id, "Lock acquired");
        Ok(lock_id)
    }

    async fn unlock(&self, project: &str, stack: &str, _lock_id: &LockId) -> Result<()> {
        if self.config.lock_table.is_none() {
            return Ok(());
        }

        let key = format!("{}/{}", project, stack);
        debug!(
            key = %key,
            lock_table = ?self.config.lock_table,
            "Releasing lock in DynamoDB"
        );

        let mut locks = self.mock_locks.write().map_err(|_| StateError::LockPoisoned)?;
        locks.remove(&key);

        info!(project = %project, stack = %stack, "Lock released");
        Ok(())
    }

    async fn get_lock_status(&self, project: &str, stack: &str) -> Result<LockStatus> {
        if self.config.lock_table.is_none() {
            return Ok(LockStatus::Unlocked);
        }

        let key = format!("{}/{}", project, stack);
        let locks = self.mock_locks.read().map_err(|_| StateError::LockPoisoned)?;

        match locks.get(&key) {
            Some(info) => Ok(LockStatus::Locked(info.clone())),
            None => Ok(LockStatus::Unlocked),
        }
    }

    async fn force_unlock(&self, project: &str, stack: &str) -> Result<()> {
        let key = format!("{}/{}", project, stack);
        let mut locks = self.mock_locks.write().map_err(|_| StateError::LockPoisoned)?;
        locks.remove(&key);
        warn!(project = %project, stack = %stack, "Force unlocked state");
        Ok(())
    }

    async fn get_history(
        &self,
        _project: &str,
        _stack: &str,
        _limit: Option<u32>,
    ) -> Result<Vec<StateHistory>> {
        // S3 versioning could be used here
        Ok(vec![])
    }

    async fn get_state_version(
        &self,
        _project: &str,
        _stack: &str,
        _version: u64,
    ) -> Result<Option<StackState>> {
        // Would need S3 versioning enabled
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_s3_backend_crud() {
        let config = S3BackendConfig::new("test-bucket").with_prefix("test/");
        let backend = S3Backend::new("my-project", config);

        let state = StackState::with_project("my-project", "dev");
        backend.save_state("my-project", "dev", &state).await.unwrap();

        let retrieved = backend.get_state("my-project", "dev").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().project, "my-project");

        let stacks = backend.list_stacks("my-project").await.unwrap();
        assert!(stacks.contains(&"dev".to_string()));

        backend.delete_state("my-project", "dev").await.unwrap();
        let deleted = backend.get_state("my-project", "dev").await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_s3_backend_locking() {
        let config = S3BackendConfig::new("test-bucket")
            .with_lock_table("devmer-locks");
        let backend = S3Backend::new("my-project", config);

        let info = LockInfo::new("test-user", "preview");
        let lock_id = backend.lock("my-project", "dev", info.clone()).await.unwrap();

        let status = backend.get_lock_status("my-project", "dev").await.unwrap();
        assert!(matches!(status, LockStatus::Locked(_)));

        let info2 = LockInfo::new("other-user", "deploy");
        let result = backend.lock("my-project", "dev", info2).await;
        assert!(result.is_err());

        backend.unlock("my-project", "dev", &lock_id).await.unwrap();

        let status = backend.get_lock_status("my-project", "dev").await.unwrap();
        assert!(matches!(status, LockStatus::Unlocked));
    }

    #[tokio::test]
    async fn test_s3_backend_no_lock_table() {
        let config = S3BackendConfig::new("test-bucket");
        let backend = S3Backend::new("my-project", config);

        let info = LockInfo::new("test-user", "preview");
        let lock_id = backend.lock("my-project", "dev", info).await.unwrap();

        let status = backend.get_lock_status("my-project", "dev").await.unwrap();
        assert!(matches!(status, LockStatus::Unlocked));

        backend.unlock("my-project", "dev", &lock_id).await.unwrap();
    }

    #[test]
    fn test_s3_config_builder() {
        let config = S3BackendConfig::new("my-bucket")
            .with_region("eu-west-1")
            .with_prefix("infra/")
            .with_lock_table("my-locks")
            .with_encryption(Some("alias/my-key".to_string()));

        assert_eq!(config.bucket, "my-bucket");
        assert_eq!(config.region, "eu-west-1");
        assert_eq!(config.prefix, "infra/");
        assert_eq!(config.lock_table, Some("my-locks".to_string()));
        assert!(config.encrypt);
        assert_eq!(config.kms_key_id, Some("alias/my-key".to_string()));
    }
}
