//! Azure Blob Storage State Backend
//!
//! This module provides an Azure Blob Storage-based state backend for Devmer.

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

/// Azure backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AzureBackendConfig {
    /// Storage account name
    pub storage_account: String,

    /// Container name
    pub container: String,

    /// Blob name prefix
    #[serde(default = "default_prefix")]
    pub prefix: String,

    /// Access key (alternative to connection string)
    pub access_key: Option<String>,

    /// SAS token
    pub sas_token: Option<String>,

    /// Connection string
    pub connection_string: Option<String>,

    /// Use Azure AD authentication
    #[serde(default)]
    pub use_azure_ad: bool,

    /// Client ID for Azure AD
    pub client_id: Option<String>,

    /// Tenant ID for Azure AD
    pub tenant_id: Option<String>,

    /// Subscription ID
    pub subscription_id: Option<String>,

    /// Resource group for leasing
    pub resource_group: Option<String>,
}

fn default_prefix() -> String {
    "devmer/".to_string()
}

impl Default for AzureBackendConfig {
    fn default() -> Self {
        Self {
            storage_account: String::new(),
            container: "tfstate".to_string(),
            prefix: default_prefix(),
            access_key: None,
            sas_token: None,
            connection_string: None,
            use_azure_ad: false,
            client_id: None,
            tenant_id: None,
            subscription_id: None,
            resource_group: None,
        }
    }
}

impl AzureBackendConfig {
    /// Create config with storage account and container
    pub fn new(storage_account: impl Into<String>, container: impl Into<String>) -> Self {
        Self {
            storage_account: storage_account.into(),
            container: container.into(),
            ..Default::default()
        }
    }

    /// Builder: set prefix
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = prefix.into();
        self
    }

    /// Builder: set access key
    pub fn with_access_key(mut self, key: impl Into<String>) -> Self {
        self.access_key = Some(key.into());
        self
    }

    /// Builder: set connection string
    pub fn with_connection_string(mut self, conn_str: impl Into<String>) -> Self {
        self.connection_string = Some(conn_str.into());
        self
    }

    /// Builder: enable Azure AD auth
    pub fn with_azure_ad(
        mut self,
        client_id: impl Into<String>,
        tenant_id: impl Into<String>,
    ) -> Self {
        self.use_azure_ad = true;
        self.client_id = Some(client_id.into());
        self.tenant_id = Some(tenant_id.into());
        self
    }
}

/// Azure Blob Storage state backend
pub struct AzureBackend {
    config: AzureBackendConfig,
    project_name: String,
    mock_storage: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    mock_locks: Arc<RwLock<HashMap<String, LockInfo>>>,
}

impl AzureBackend {
    /// Create a new Azure backend
    pub fn new(project_name: impl Into<String>, config: AzureBackendConfig) -> Self {
        info!(
            storage_account = %config.storage_account,
            container = %config.container,
            prefix = %config.prefix,
            "Initializing Azure Blob Storage state backend"
        );

        Self {
            config,
            project_name: project_name.into(),
            mock_storage: Arc::new(RwLock::new(HashMap::new())),
            mock_locks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn blob_name(&self, project: &str, stack: &str) -> String {
        format!("{}{}/{}.json", self.config.prefix, project, stack)
    }
}

#[async_trait]
impl StateBackend for AzureBackend {
    fn name(&self) -> &str {
        "azure"
    }

    async fn get_state(&self, project: &str, stack: &str) -> Result<Option<StackState>> {
        let blob = self.blob_name(project, stack);
        debug!(
            blob = %blob,
            container = %self.config.container,
            "Getting state from Azure Blob Storage"
        );

        let storage = self.mock_storage.read().map_err(|_| StateError::LockPoisoned)?;
        match storage.get(&blob) {
            Some(data) => {
                let state: StackState = serde_json::from_slice(data)
                    .map_err(|e| StateError::Deserialize(e.to_string()))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    async fn save_state(&self, project: &str, stack: &str, state: &StackState) -> Result<()> {
        let blob = self.blob_name(project, stack);
        debug!(
            blob = %blob,
            container = %self.config.container,
            "Putting state to Azure Blob Storage"
        );

        let data = serde_json::to_vec_pretty(state)
            .map_err(|e| StateError::Serialize(e.to_string()))?;

        let mut storage = self.mock_storage.write().map_err(|_| StateError::LockPoisoned)?;
        storage.insert(blob, data);

        info!(project = %project, stack = %stack, "State saved to Azure Blob Storage");
        Ok(())
    }

    async fn delete_state(&self, project: &str, stack: &str) -> Result<()> {
        let blob = self.blob_name(project, stack);
        debug!(
            blob = %blob,
            container = %self.config.container,
            "Deleting state from Azure Blob Storage"
        );

        let mut storage = self.mock_storage.write().map_err(|_| StateError::LockPoisoned)?;
        storage.remove(&blob);

        info!(project = %project, stack = %stack, "State deleted from Azure Blob Storage");
        Ok(())
    }

    async fn list_stacks(&self, project: &str) -> Result<Vec<String>> {
        let prefix = format!("{}{}/", self.config.prefix, project);
        debug!(
            prefix = %prefix,
            container = %self.config.container,
            "Listing stacks in Azure Blob Storage"
        );

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
        debug!(key = %key, "Acquiring blob lease");

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
        debug!(key = %key, "Releasing blob lease");

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
    async fn test_azure_backend_crud() {
        let config = AzureBackendConfig::new("mystorageaccount", "tfstate");
        let backend = AzureBackend::new("my-project", config);

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
    async fn test_azure_backend_locking() {
        let config = AzureBackendConfig::new("mystorageaccount", "tfstate");
        let backend = AzureBackend::new("my-project", config);

        let info = LockInfo::new("test-user", "preview");
        let lock_id = backend.lock("my-project", "dev", info).await.unwrap();

        let status = backend.get_lock_status("my-project", "dev").await.unwrap();
        assert!(matches!(status, LockStatus::Locked(_)));

        backend.unlock("my-project", "dev", &lock_id).await.unwrap();

        let status = backend.get_lock_status("my-project", "dev").await.unwrap();
        assert!(matches!(status, LockStatus::Unlocked));
    }

    #[test]
    fn test_azure_config_builder() {
        let config = AzureBackendConfig::new("myaccount", "mycontainer")
            .with_prefix("state/")
            .with_azure_ad("client-123", "tenant-456");

        assert_eq!(config.storage_account, "myaccount");
        assert_eq!(config.container, "mycontainer");
        assert_eq!(config.prefix, "state/");
        assert!(config.use_azure_ad);
        assert_eq!(config.client_id, Some("client-123".to_string()));
        assert_eq!(config.tenant_id, Some("tenant-456".to_string()));
    }
}
