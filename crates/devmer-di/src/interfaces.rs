//! Service interface definitions

use async_trait::async_trait;
use devmer_config::DevmerConfig;
use devmer_core::provider::Provider;
use devmer_core::state::StackState;
use devmer_runtime::runtime::{RunResult, RuntimeConfig, RuntimeKind};
use devmer_secrets::types::{EncryptedValue, EncryptionContext};
use devmer_state::locking::{LockId, LockInfo, LockStatus};
use shaku::Interface;
use std::sync::Arc;

/// Configuration service interface
pub trait ConfigService: Interface {
    /// Get a configuration value by key
    fn get(&self, key: &str) -> Option<String>;

    /// Get the full configuration
    fn config(&self) -> &DevmerConfig;

    /// Get stack names
    fn stack_names(&self) -> Vec<String>;
}

/// State backend service interface
#[async_trait]
pub trait StateService: Interface {
    /// Get state for a stack
    async fn get_state(&self, stack: &str) -> anyhow::Result<Option<StackState>>;

    /// Save state for a stack
    async fn save_state(&self, stack: &str, state: &StackState) -> anyhow::Result<()>;

    /// Delete state for a stack
    async fn delete_state(&self, stack: &str) -> anyhow::Result<()>;

    /// List all stacks
    async fn list_stacks(&self) -> anyhow::Result<Vec<String>>;

    /// Acquire a lock
    async fn lock(&self, stack: &str, info: LockInfo) -> anyhow::Result<LockId>;

    /// Release a lock
    async fn unlock(&self, stack: &str, lock_id: &LockId) -> anyhow::Result<()>;

    /// Get lock status
    async fn get_lock_status(&self, stack: &str) -> anyhow::Result<LockStatus>;
}

/// Secrets provider service interface
#[async_trait]
pub trait SecretsService: Interface {
    /// Encrypt data
    async fn encrypt(
        &self,
        plaintext: &[u8],
        context: &EncryptionContext,
    ) -> anyhow::Result<EncryptedValue>;

    /// Decrypt data
    async fn decrypt(
        &self,
        ciphertext: &EncryptedValue,
        context: &EncryptionContext,
    ) -> anyhow::Result<Vec<u8>>;
}

/// Provider registry interface
pub trait ProviderRegistryService: Interface {
    /// Get a provider by name
    fn get_provider(&self, name: &str) -> Option<Arc<dyn Provider>>;

    /// Register a provider
    fn register_provider(&self, name: &str, provider: Arc<dyn Provider>);

    /// List all registered providers
    fn list_providers(&self) -> Vec<String>;
}

/// Runtime service interface - executes infrastructure programs
#[async_trait]
pub trait RuntimeService: Interface {
    /// Run an infrastructure program and collect resources
    async fn run(&self, config: &RuntimeConfig) -> anyhow::Result<RunResult>;

    /// Get the configured runtime kind
    fn runtime_kind(&self) -> RuntimeKind;

    /// Check if the runtime is available
    async fn is_available(&self) -> bool;

    /// Install dependencies
    async fn install_dependencies(&self, config: &RuntimeConfig) -> anyhow::Result<()>;
}

/// Execution service interface
#[async_trait]
pub trait ExecutionService: Interface {
    /// Preview changes for a stack
    async fn preview(&self, stack: &str) -> anyhow::Result<PreviewResult>;

    /// Deploy a stack
    async fn deploy(&self, stack: &str, auto_approve: bool) -> anyhow::Result<DeployResult>;

    /// Destroy a stack
    async fn destroy(&self, stack: &str, auto_approve: bool) -> anyhow::Result<DestroyResult>;

    /// Refresh state from cloud
    async fn refresh(&self, stack: &str) -> anyhow::Result<RefreshResult>;
}

/// Preview result
#[derive(Debug, Clone)]
pub struct PreviewResult {
    /// Stack name
    pub stack: String,
    /// Resources to create
    pub creates: Vec<ResourceChange>,
    /// Resources to update
    pub updates: Vec<ResourceChange>,
    /// Resources to delete
    pub deletes: Vec<ResourceChange>,
    /// Resources unchanged
    pub same: usize,
}

impl PreviewResult {
    /// Create an empty preview result
    pub fn empty(stack: &str) -> Self {
        Self {
            stack: stack.to_string(),
            creates: vec![],
            updates: vec![],
            deletes: vec![],
            same: 0,
        }
    }

    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        !self.creates.is_empty() || !self.updates.is_empty() || !self.deletes.is_empty()
    }

    /// Total number of changes
    pub fn total_changes(&self) -> usize {
        self.creates.len() + self.updates.len() + self.deletes.len()
    }
}

/// A resource change
#[derive(Debug, Clone)]
pub struct ResourceChange {
    /// Resource URN
    pub urn: String,
    /// Resource type
    pub resource_type: String,
    /// Resource name
    pub name: String,
    /// Change type
    pub change_type: ChangeType,
    /// Property diffs
    pub diffs: Vec<PropertyDiff>,
}

/// Type of change
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeType {
    Create,
    Update,
    Replace,
    Delete,
    Same,
}

/// Property diff
#[derive(Debug, Clone)]
pub struct PropertyDiff {
    /// Property path
    pub path: String,
    /// Old value (display)
    pub old_value: Option<String>,
    /// New value (display)
    pub new_value: Option<String>,
}

/// Deploy result
#[derive(Debug, Clone)]
pub struct DeployResult {
    /// Stack name
    pub stack: String,
    /// Whether deployment succeeded
    pub success: bool,
    /// Resources created
    pub resources_created: usize,
    /// Resources updated
    pub resources_updated: usize,
    /// Resources deleted
    pub resources_deleted: usize,
    /// Errors encountered
    pub errors: Vec<String>,
    /// Duration in seconds
    pub duration_secs: f64,
}

/// Destroy result
#[derive(Debug, Clone)]
pub struct DestroyResult {
    /// Stack name
    pub stack: String,
    /// Whether destruction succeeded
    pub success: bool,
    /// Resources destroyed
    pub resources_destroyed: usize,
    /// Errors encountered
    pub errors: Vec<String>,
    /// Duration in seconds
    pub duration_secs: f64,
}

/// Refresh result
#[derive(Debug, Clone)]
pub struct RefreshResult {
    /// Stack name
    pub stack: String,
    /// Whether refresh succeeded
    pub success: bool,
    /// Resources refreshed
    pub resources_refreshed: usize,
    /// Resources with drift
    pub drift_detected: usize,
}
