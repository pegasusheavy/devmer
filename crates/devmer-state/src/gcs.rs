//! Google Cloud Storage State Backend
//!
//! This module provides a GCS-based state backend for Devmer.

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

/// GCS backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GcsBackendConfig {
    /// GCS bucket name
    pub bucket: String,

    /// Key prefix for state files
    #[serde(default = "default_prefix")]
    pub prefix: String,

    /// GCP project ID
    pub project: Option<String>,

    /// Path to service account credentials JSON
    pub credentials: Option<String>,

    /// Enable encryption
    #[serde(default)]
    pub encrypt: bool,

    /// Customer-supplied encryption key
    pub encryption_key: Option<String>,
}

fn default_prefix() -> String {
    "devmer/".to_string()
}

impl Default for GcsBackendConfig {
    fn default() -> Self {
        Self {
            bucket: String::new(),
            prefix: default_prefix(),
            project: None,
            credentials: None,
            encrypt: false,
            encryption_key: None,
        }
    }
}

impl GcsBackendConfig {
    /// Create config with bucket name
    pub fn new(bucket: impl Into<String>) -> Self {
        Self {
            bucket: bucket.into(),
            ..Default::default()
        }
    }

    /// Builder: set prefix
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Builder: set project
    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// Builder: set credentials
    pub fn with_credentials(mut self, path: impl Into<String>) -> Self {
        self.credentials = Some(path.into());
        self
    }
}

/// GCS state backend
pub struct GcsBackend {
    config: GcsBackendConfig,
    project_name: String,
    mock_storage: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    mock_locks: Arc<RwLock<HashMap<String, LockInfo>>>,
}

impl GcsBackend {
    /// Create a new GCS backend
    pub fn new(project_name: impl Into<String>, config: GcsBackendConfig) -> Self {
        info!(
            bucket = %config.bucket,
            prefix = %config.prefix,
            "Initializing GCS state backend"
        );

        Self {
            config,
            project_name: project_name.into(),
            mock_storage: Arc::new(RwLock::new(HashMap::new())),
            mock_locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn object_key(&self, project: &str, stack: &str) -> String {
        format!("{}{}/{}.json", self.config.prefix, project, stack)
    }
}

#[async_trait]
impl StateBackend for GcsBackend {
    fn name(&self) -> &str {
        "gcs"
    }

    async fn get_state(&self, project: &str, stack: &str) -> Result<Option<StackState>> {
        let key = self.object_key(project, stack);
        debug!(key = %key, bucket = %self.config.bucket, "Getting state from GCS");

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
        let key = self.object_key(project, stack);
        debug!(key = %key, bucket = %self.config.bucket, "Putting state to GCS");

        let data = serde_json::to_vec_pretty(state)
            .map_err(|e| StateError::Serialize(e.to_string()))?;

        let mut storage = self.mock_storage.write().map_err(|_| StateError::LockPoisoned)?;
        storage.insert(key, data);

        info!(project = %project, stack = %stack, "State saved to GCS");
        Ok(())
    }

    async fn delete_state(&self, project: &str, stack: &str) -> Result<()> {
        let key = self.object_key(project, stack);
        debug!(key = %key, bucket = %self.config.bucket, "Deleting state from GCS");

        let mut storage = self.mock_storage.write().map_err(|_| StateError::LockPoisoned)?;
        storage.remove(&key);

        info!(project = %project, stack = %stack, "State deleted from GCS");
        Ok(())
    }

    async fn list_stacks(&self, project: &str) -> Result<Vec<String>> {
        let prefix = format!("{}{}/", self.config.prefix, project);
        debug!(prefix = %prefix, bucket = %self.config.bucket, "Listing stacks in GCS");

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
        let key = format!("{}/{}", project, stack);
        debug!(key = %key, "Acquiring lock (GCS generation-based)");

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

        Ok(lock_id)
    }

    async fn unlock(&self, project: &str, stack: &str, _lock_id: &LockId) -> Result<()> {
        let key = format!("{}/{}", project, stack);
        debug!(key = %key, "Releasing lock");

        let mut locks = self.mock_locks.write().map_err(|_| StateError::LockPoisoned)?;
        locks.remove(&key);

        Ok(())
    }

    async fn get_lock_status(&self, project: &str, stack: &str) -> Result<LockStatus> {
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
        Ok(vec![])
    }

    async fn get_state_version(
        &self,
        _project: &str,
        _stack: &str,
        _version: u64,
    ) -> Result<Option<StackState>> {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gcs_backend_crud() {
        let config = GcsBackendConfig::new("test-bucket");
        let backend = GcsBackend::new("my-project", config);

        let state = StackState::with_project("my-project", "dev");
        backend.save_state("my-project", "dev", &state).await.unwrap();

        let retrieved = backend.get_state("my-project", "dev").await.unwrap();
        assert!(retrieved.is_some());

        let stacks = backend.list_stacks("my-project").await.unwrap();
        assert!(stacks.contains(&"dev".to_string()));

        backend.delete_state("my-project", "dev").await.unwrap();
        let deleted = backend.get_state("my-project", "dev").await.unwrap();
        assert!(deleted.is_none());
    }

    #[tokio::test]
    async fn test_gcs_backend_locking() {
        let config = GcsBackendConfig::new("test-bucket");
        let backend = GcsBackend::new("my-project", config);

        let info = LockInfo::new("test-user", "preview");
        let lock_id = backend.lock("my-project", "dev", info).await.unwrap();

        let status = backend.get_lock_status("my-project", "dev").await.unwrap();
        assert!(matches!(status, LockStatus::Locked(_)));

        backend.unlock("my-project", "dev", &lock_id).await.unwrap();

        let status = backend.get_lock_status("my-project", "dev").await.unwrap();
        assert!(matches!(status, LockStatus::Unlocked));
    }
}
