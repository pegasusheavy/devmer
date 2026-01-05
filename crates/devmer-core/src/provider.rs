//! Provider traits and types

use crate::resource::{Resource, ResourceType};
use crate::types::PropertyValues;
use crate::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Configuration for a provider instance
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfig {
    /// Provider name
    pub name: String,

    /// Provider version
    pub version: Option<String>,

    /// Configuration values
    #[serde(default)]
    pub config: PropertyValues,

    /// Plugin path (for external providers)
    pub plugin_path: Option<String>,
}

/// Schema for a resource type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSchema {
    /// Resource type identifier
    pub resource_type: ResourceType,

    /// Description of the resource
    pub description: Option<String>,

    /// Input property schemas
    pub input_properties: HashMap<String, PropertySchema>,

    /// Output property schemas
    pub output_properties: HashMap<String, PropertySchema>,

    /// Required input properties
    #[serde(default)]
    pub required: Vec<String>,
}

/// Schema for a single property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySchema {
    /// Property type
    pub property_type: PropertyType,

    /// Description
    pub description: Option<String>,

    /// Default value
    pub default: Option<serde_json::Value>,

    /// Whether this property is secret
    #[serde(default)]
    pub secret: bool,

    /// Whether this property forces replacement
    #[serde(default)]
    pub replace_on_change: bool,

    /// Deprecation message
    pub deprecated: Option<String>,
}

/// Property types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PropertyType {
    /// String type
    String,
    /// Integer type
    Integer,
    /// Number (float) type
    Number,
    /// Boolean type
    Boolean,
    /// Array type with element type
    Array(Box<PropertyType>),
    /// Object type with property schemas
    Object(HashMap<String, PropertySchema>),
    /// Union of types
    Union(Vec<PropertyType>),
    /// Reference to another resource
    Resource(String),
    /// Any type
    Any,
}

/// Provider schema containing all resource types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSchema {
    /// Provider name
    pub name: String,

    /// Provider version
    pub version: String,

    /// Description
    pub description: Option<String>,

    /// Resource schemas by type
    pub resources: HashMap<String, ResourceSchema>,

    /// Configuration schema
    pub config: Option<ResourceSchema>,
}

/// Result of a CRUD operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationResult {
    /// Whether the operation succeeded
    pub success: bool,

    /// The resource after the operation
    pub resource: Option<Resource>,

    /// Error message if failed
    pub error: Option<String>,

    /// Warnings
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl OperationResult {
    /// Create a success result
    pub fn success(resource: Resource) -> Self {
        Self {
            success: true,
            resource: Some(resource),
            error: None,
            warnings: vec![],
        }
    }

    /// Create a failure result
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            resource: None,
            error: Some(error.into()),
            warnings: vec![],
        }
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}

/// Check result for resource read operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Validated/normalized inputs
    pub inputs: PropertyValues,

    /// Validation failures
    #[serde(default)]
    pub failures: Vec<CheckFailure>,
}

/// A validation failure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckFailure {
    /// Property path (e.g., "bucket.tags.Name")
    pub property: String,

    /// Failure reason
    pub reason: String,
}

/// Diff result for update operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffResult {
    /// Changes to properties
    pub changes: HashMap<String, PropertyDiff>,

    /// Whether the resource needs replacement
    pub replace: bool,

    /// Properties that caused replacement
    #[serde(default)]
    pub replace_keys: Vec<String>,

    /// Stable keys (won't change)
    #[serde(default)]
    pub stable_keys: Vec<String>,
}

/// Diff for a single property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertyDiff {
    /// Type of change
    pub kind: DiffKind,

    /// Input key changed
    pub input_diff: bool,

    /// Output key changed
    pub output_diff: bool,
}

/// Kind of property diff
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffKind {
    /// Property added
    Add,
    /// Property removed
    Delete,
    /// Property updated
    Update,
    /// Property updated and requires replacement
    UpdateReplace,
}

/// Provider trait - defines the interface for resource providers
#[async_trait]
pub trait Provider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Get the provider version
    fn version(&self) -> &str;

    /// Get the provider schema
    async fn schema(&self) -> Result<ProviderSchema>;

    /// Configure the provider with the given configuration
    async fn configure(&mut self, config: ProviderConfig) -> Result<()>;

    /// Check and validate inputs before create/update
    async fn check(
        &self,
        resource_type: &ResourceType,
        inputs: PropertyValues,
    ) -> Result<CheckResult>;

    /// Diff the old and new inputs to determine what changes are needed
    async fn diff(
        &self,
        resource: &Resource,
        new_inputs: PropertyValues,
    ) -> Result<DiffResult>;

    /// Create a new resource
    async fn create(&self, resource: &Resource) -> Result<OperationResult>;

    /// Read the current state of a resource
    async fn read(&self, resource: &Resource) -> Result<OperationResult>;

    /// Update an existing resource
    async fn update(
        &self,
        resource: &Resource,
        new_inputs: PropertyValues,
    ) -> Result<OperationResult>;

    /// Delete a resource
    async fn delete(&self, resource: &Resource) -> Result<OperationResult>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_result() {
        let success = OperationResult::success(Resource::new(
            "test",
            ResourceType::new("aws", "s3", "Bucket"),
            "test-bucket",
            PropertyValues::new(),
        ));
        assert!(success.success);
        assert!(success.resource.is_some());

        let failure = OperationResult::failure("Something went wrong");
        assert!(!failure.success);
        assert!(failure.error.is_some());
    }
}
