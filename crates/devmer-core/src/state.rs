//! Stack state types

use crate::resource::Resource;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// State version for format compatibility
pub const STATE_VERSION: u32 = 1;

/// Complete state of a stack
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackState {
    /// State format version
    pub version: u32,

    /// Stack name
    pub stack: String,

    /// Project name
    pub project: String,

    /// Current checkpoint
    pub checkpoint: StateCheckpoint,

    /// Deployment history (most recent first)
    #[serde(default)]
    pub history: Vec<DeploymentRecord>,

    /// Pending operations (for crash recovery)
    #[serde(default)]
    pub pending_operations: Vec<PendingOperation>,

    /// Metadata
    #[serde(default)]
    pub metadata: StateMetadata,
}

impl StackState {
    /// Create a new empty stack state
    pub fn new(stack: &str) -> Self {
        Self {
            version: STATE_VERSION,
            stack: stack.to_string(),
            project: String::new(),
            checkpoint: StateCheckpoint::new(),
            history: vec![],
            pending_operations: vec![],
            metadata: StateMetadata::default(),
        }
    }

    /// Create a new stack state with project
    pub fn with_project(project: &str, stack: &str) -> Self {
        Self {
            version: STATE_VERSION,
            stack: stack.to_string(),
            project: project.to_string(),
            checkpoint: StateCheckpoint::new(),
            history: vec![],
            pending_operations: vec![],
            metadata: StateMetadata::default(),
        }
    }

    /// Get a resource by URN
    pub fn get_resource(&self, urn: &str) -> Option<&Resource> {
        self.checkpoint.resources.get(urn)
    }

    /// Get a mutable resource by URN
    pub fn get_resource_mut(&mut self, urn: &str) -> Option<&mut Resource> {
        self.checkpoint.resources.get_mut(urn)
    }

    /// Add or update a resource
    pub fn upsert_resource(&mut self, resource: Resource) {
        self.checkpoint
            .resources
            .insert(resource.urn.as_str().to_string(), resource);
    }

    /// Add or update a resource (alias for upsert_resource)
    pub fn add_or_update_resource(&mut self, resource: Resource) {
        self.upsert_resource(resource);
    }

    /// Remove a resource by URN
    pub fn remove_resource(&mut self, urn: &str) -> Option<Resource> {
        self.checkpoint.resources.remove(urn)
    }

    /// Get all resources
    pub fn resources(&self) -> impl Iterator<Item = &Resource> {
        self.checkpoint.resources.values()
    }

    /// Get all resources as a Vec (for compatibility)
    pub fn resources_vec(&self) -> Vec<Resource> {
        self.checkpoint.resources.values().cloned().collect()
    }

    /// Count resources
    pub fn resource_count(&self) -> usize {
        self.checkpoint.resources.len()
    }
}

/// A checkpoint represents a point-in-time snapshot of resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateCheckpoint {
    /// Resources by URN
    pub resources: HashMap<String, Resource>,

    /// Timestamp
    pub timestamp: DateTime<Utc>,

    /// Secrets provider used
    pub secrets_provider: Option<String>,
}

impl StateCheckpoint {
    /// Create a new empty checkpoint
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            timestamp: Utc::now(),
            secrets_provider: None,
        }
    }
}

impl Default for StateCheckpoint {
    fn default() -> Self {
        Self::new()
    }
}

/// Record of a deployment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentRecord {
    /// Unique deployment ID
    pub id: Uuid,

    /// Deployment kind
    pub kind: DeploymentKind,

    /// When deployment started
    pub started_at: DateTime<Utc>,

    /// When deployment ended
    pub ended_at: Option<DateTime<Utc>>,

    /// Result of the deployment
    pub result: DeploymentResult,

    /// Resources created
    pub resources_created: u32,

    /// Resources updated
    pub resources_updated: u32,

    /// Resources deleted
    pub resources_deleted: u32,

    /// Who initiated the deployment
    pub initiator: Option<String>,

    /// Message/description
    pub message: Option<String>,

    /// Environment variables (sanitized)
    #[serde(default)]
    pub environment: HashMap<String, String>,
}

/// Kind of deployment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentKind {
    /// Create/update resources
    Update,
    /// Preview changes
    Preview,
    /// Refresh state from cloud
    Refresh,
    /// Destroy all resources
    Destroy,
    /// Import existing resources
    Import,
}

/// Result of a deployment
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeploymentResult {
    /// Deployment succeeded
    Succeeded,
    /// Deployment failed
    Failed,
    /// Deployment was cancelled
    Cancelled,
    /// Deployment is in progress
    InProgress,
}

/// Pending operation for crash recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingOperation {
    /// Resource URN
    pub resource_urn: String,

    /// Operation type
    pub operation: PendingOperationType,

    /// When operation started
    pub started_at: DateTime<Utc>,
}

/// Type of pending operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PendingOperationType {
    /// Creating a resource
    Creating,
    /// Updating a resource
    Updating,
    /// Deleting a resource
    Deleting,
    /// Reading/refreshing a resource
    Reading,
}

/// State metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StateMetadata {
    /// Last modified timestamp
    pub last_modified: Option<DateTime<Utc>>,

    /// Last deployment ID
    pub last_deployment_id: Option<Uuid>,

    /// Configuration hash (for drift detection)
    pub config_hash: Option<String>,

    /// Provider versions used
    #[serde(default)]
    pub provider_versions: HashMap<String, String>,

    /// Custom key-value metadata
    #[serde(default)]
    pub custom: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::ResourceType;
    use crate::types::PropertyValues;

    #[test]
    fn test_stack_state_operations() {
        let mut state = StackState::with_project("my-project", "dev");
        assert_eq!(state.resource_count(), 0);

        let resource = Resource::new(
            "dev",
            ResourceType::new("aws", "s3", "Bucket"),
            "my-bucket",
            PropertyValues::new(),
        );

        state.upsert_resource(resource.clone());
        assert_eq!(state.resource_count(), 1);

        let found = state.get_resource(resource.urn.as_str());
        assert!(found.is_some());
        assert_eq!(found.unwrap().name, "my-bucket");
    }
}
