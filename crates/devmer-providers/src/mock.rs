//! Mock provider for testing
//!
//! Simulates cloud operations without making real API calls.

use async_trait::async_trait;
use devmer_core::provider::{
    CheckFailure, CheckResult, DiffKind, DiffResult, OperationResult, PropertyDiff,
    PropertySchema, PropertyType, Provider, ProviderConfig, ProviderSchema, ResourceSchema,
};
use devmer_core::resource::{Resource, ResourceType};
use devmer_core::types::{PropertyValue, PropertyValues};
use devmer_core::{DevmerError, Result};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info};
use uuid::Uuid;

/// Mock provider for testing
pub struct MockProvider {
    /// Provider configuration
    config: ProviderConfig,

    /// Simulated resource state
    resources: Arc<RwLock<HashMap<String, MockResource>>>,

    /// Whether to simulate failures
    fail_rate: f32,

    /// Simulated latency in milliseconds
    latency_ms: u64,
}

#[derive(Debug, Clone)]
struct MockResource {
    id: String,
    resource_type: String,
    inputs: PropertyValues,
    outputs: PropertyValues,
}

impl MockProvider {
    /// Create a new mock provider with default config
    pub fn new() -> Self {
        Self {
            config: ProviderConfig::default(),
            resources: Arc::new(RwLock::new(HashMap::new())),
            fail_rate: 0.0,
            latency_ms: 0,
        }
    }

    /// Create a new mock provider with specific config
    pub fn with_config(config: ProviderConfig) -> Self {
        Self {
            config,
            resources: Arc::new(RwLock::new(HashMap::new())),
            fail_rate: 0.0,
            latency_ms: 0,
        }
    }

    /// Create with simulated failures
    pub fn with_fail_rate(mut self, rate: f32) -> Self {
        self.fail_rate = rate.clamp(0.0, 1.0);
        self
    }

    /// Create with simulated latency
    pub fn with_latency(mut self, ms: u64) -> Self {
        self.latency_ms = ms;
        self
    }

    /// Simulate latency
    async fn simulate_latency(&self) {
        if self.latency_ms > 0 {
            tokio::time::sleep(tokio::time::Duration::from_millis(self.latency_ms)).await;
        }
    }

    /// Check if operation should fail
    fn should_fail(&self) -> bool {
        if self.fail_rate > 0.0 {
            rand_simple() < self.fail_rate
        } else {
            false
        }
    }

    /// Get resource schema for a type
    fn get_resource_schema(&self, resource_type: &ResourceType) -> ResourceSchema {
        let type_name = resource_type.type_name();

        let (description, input_properties, output_properties, required) = match type_name {
            "Bucket" => (
                "A mock storage bucket",
                vec![
                    (
                        "name",
                        PropertySchema {
                            property_type: PropertyType::String,
                            description: Some("Bucket name".to_string()),
                            default: None,
                            secret: false,
                            replace_on_change: true,
                            deprecated: None,
                        },
                    ),
                    (
                        "versioning",
                        PropertySchema {
                            property_type: PropertyType::Boolean,
                            description: Some("Enable versioning".to_string()),
                            default: Some(serde_json::Value::Bool(false)),
                            secret: false,
                            replace_on_change: false,
                            deprecated: None,
                        },
                    ),
                ],
                vec![
                    (
                        "arn",
                        PropertySchema {
                            property_type: PropertyType::String,
                            description: Some("Bucket ARN".to_string()),
                            default: None,
                            secret: false,
                            replace_on_change: false,
                            deprecated: None,
                        },
                    ),
                    (
                        "endpoint",
                        PropertySchema {
                            property_type: PropertyType::String,
                            description: Some("Bucket endpoint URL".to_string()),
                            default: None,
                            secret: false,
                            replace_on_change: false,
                            deprecated: None,
                        },
                    ),
                ],
                vec!["name"],
            ),
            "Function" => (
                "A mock serverless function",
                vec![
                    (
                        "name",
                        PropertySchema {
                            property_type: PropertyType::String,
                            description: Some("Function name".to_string()),
                            default: None,
                            secret: false,
                            replace_on_change: true,
                            deprecated: None,
                        },
                    ),
                    (
                        "runtime",
                        PropertySchema {
                            property_type: PropertyType::String,
                            description: Some("Function runtime".to_string()),
                            default: None,
                            secret: false,
                            replace_on_change: false,
                            deprecated: None,
                        },
                    ),
                    (
                        "memory",
                        PropertySchema {
                            property_type: PropertyType::Integer,
                            description: Some("Memory in MB".to_string()),
                            default: Some(serde_json::json!(128)),
                            secret: false,
                            replace_on_change: false,
                            deprecated: None,
                        },
                    ),
                ],
                vec![
                    (
                        "arn",
                        PropertySchema {
                            property_type: PropertyType::String,
                            description: Some("Function ARN".to_string()),
                            default: None,
                            secret: false,
                            replace_on_change: false,
                            deprecated: None,
                        },
                    ),
                    (
                        "invoke_url",
                        PropertySchema {
                            property_type: PropertyType::String,
                            description: Some("Function invocation URL".to_string()),
                            default: None,
                            secret: false,
                            replace_on_change: false,
                            deprecated: None,
                        },
                    ),
                ],
                vec!["name", "runtime"],
            ),
            "Instance" => (
                "A mock database instance",
                vec![
                    (
                        "name",
                        PropertySchema {
                            property_type: PropertyType::String,
                            description: Some("Database name".to_string()),
                            default: None,
                            secret: false,
                            replace_on_change: true,
                            deprecated: None,
                        },
                    ),
                    (
                        "engine",
                        PropertySchema {
                            property_type: PropertyType::String,
                            description: Some("Database engine".to_string()),
                            default: None,
                            secret: false,
                            replace_on_change: true,
                            deprecated: None,
                        },
                    ),
                    (
                        "size",
                        PropertySchema {
                            property_type: PropertyType::String,
                            description: Some("Instance size".to_string()),
                            default: Some(serde_json::json!("small")),
                            secret: false,
                            replace_on_change: false,
                            deprecated: None,
                        },
                    ),
                ],
                vec![
                    (
                        "endpoint",
                        PropertySchema {
                            property_type: PropertyType::String,
                            description: Some("Database endpoint".to_string()),
                            default: None,
                            secret: false,
                            replace_on_change: false,
                            deprecated: None,
                        },
                    ),
                    (
                        "port",
                        PropertySchema {
                            property_type: PropertyType::Integer,
                            description: Some("Database port".to_string()),
                            default: None,
                            secret: false,
                            replace_on_change: false,
                            deprecated: None,
                        },
                    ),
                ],
                vec!["name", "engine"],
            ),
            _ => (
                "Unknown resource type",
                vec![],
                vec![],
                vec![],
            ),
        };

        ResourceSchema {
            resource_type: resource_type.clone(),
            description: Some(description.to_string()),
            input_properties: input_properties
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
            output_properties: output_properties
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
            required: required.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    /// Generate mock outputs for a resource
    fn generate_outputs(&self, resource_type: &ResourceType, inputs: &PropertyValues) -> PropertyValues {
        let name = inputs
            .get("name")
            .and_then(|v| match v {
                PropertyValue::String(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "unnamed".to_string());

        let mut outputs = PropertyValues::new();
        let id = Uuid::new_v4().to_string();
        let type_name = resource_type.type_name();

        match type_name {
            "Bucket" => {
                outputs.insert(
                    "arn".to_string(),
                    PropertyValue::String(format!("arn:mock:s3:::{}_{}", name, id)),
                );
                outputs.insert(
                    "endpoint".to_string(),
                    PropertyValue::String(format!("https://{}.s3.mock.local", name)),
                );
            }
            "Function" => {
                outputs.insert(
                    "arn".to_string(),
                    PropertyValue::String(format!(
                        "arn:mock:lambda:us-east-1:123456789:function:{}",
                        name
                    )),
                );
                outputs.insert(
                    "invoke_url".to_string(),
                    PropertyValue::String(format!("https://lambda.mock.local/{}", name)),
                );
            }
            "Instance" => {
                let engine = inputs
                    .get("engine")
                    .and_then(|v| match v {
                        PropertyValue::String(s) => Some(s.clone()),
                        _ => None,
                    })
                    .unwrap_or_else(|| "postgres".to_string());

                let port = match engine.as_str() {
                    "mysql" => 3306,
                    "postgres" => 5432,
                    _ => 5432,
                };

                outputs.insert(
                    "endpoint".to_string(),
                    PropertyValue::String(format!("{}.db.mock.local", name)),
                );
                outputs.insert("port".to_string(), PropertyValue::Int(port));
            }
            _ => {}
        }

        outputs
    }
}

/// Simple random number generator for fail simulation
fn rand_simple() -> f32 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();
    (nanos % 1000) as f32 / 1000.0
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn name(&self) -> &str {
        "mock"
    }

    fn version(&self) -> &str {
        "0.1.0"
    }

    async fn schema(&self) -> Result<ProviderSchema> {
        let mut resources = HashMap::new();

        for (module, type_name) in [("storage", "Bucket"), ("compute", "Function"), ("database", "Instance")] {
            let rt = ResourceType::new("mock", module, type_name);
            let schema = self.get_resource_schema(&rt);
            resources.insert(format!("mock:{}:{}", module, type_name), schema);
        }

        Ok(ProviderSchema {
            name: "mock".to_string(),
            version: "0.1.0".to_string(),
            description: Some("Mock provider for testing".to_string()),
            resources,
            config: None,
        })
    }

    async fn configure(&mut self, config: ProviderConfig) -> Result<()> {
        self.config = config;
        info!("Configured mock provider");
        Ok(())
    }

    async fn check(
        &self,
        resource_type: &ResourceType,
        inputs: PropertyValues,
    ) -> Result<CheckResult> {
        let schema = self.get_resource_schema(resource_type);
        let mut failures = vec![];

        // Check required fields
        for required in &schema.required {
            if !inputs.contains_key(required) {
                failures.push(CheckFailure {
                    property: required.clone(),
                    reason: format!("Required property '{}' is missing", required),
                });
            }
        }

        Ok(CheckResult { inputs, failures })
    }

    async fn diff(&self, resource: &Resource, new_inputs: PropertyValues) -> Result<DiffResult> {
        let old_inputs = &resource.inputs;
        let mut changes = HashMap::new();
        let mut replace = false;
        let mut replace_keys = vec![];
        let mut stable_keys = vec![];

        let schema = self.get_resource_schema(&resource.resource_type);

        // Check for changed inputs
        for (key, new_value) in &new_inputs {
            let is_replace_key = schema
                .input_properties
                .get(key)
                .map(|p| p.replace_on_change)
                .unwrap_or(false);

            match old_inputs.get(key) {
                Some(old_value) if old_value != new_value => {
                    let kind = if is_replace_key {
                        replace = true;
                        replace_keys.push(key.clone());
                        DiffKind::UpdateReplace
                    } else {
                        DiffKind::Update
                    };
                    changes.insert(
                        key.clone(),
                        PropertyDiff {
                            kind,
                            input_diff: true,
                            output_diff: false,
                        },
                    );
                }
                None => {
                    changes.insert(
                        key.clone(),
                        PropertyDiff {
                            kind: DiffKind::Add,
                            input_diff: true,
                            output_diff: false,
                        },
                    );
                }
                Some(_) => {
                    stable_keys.push(key.clone());
                }
            }
        }

        // Check for deleted inputs
        for key in old_inputs.keys() {
            if !new_inputs.contains_key(key) {
                changes.insert(
                    key.clone(),
                    PropertyDiff {
                        kind: DiffKind::Delete,
                        input_diff: true,
                        output_diff: false,
                    },
                );
            }
        }

        Ok(DiffResult {
            changes,
            replace,
            replace_keys,
            stable_keys,
        })
    }

    async fn create(&self, resource: &Resource) -> Result<OperationResult> {
        self.simulate_latency().await;

        if self.should_fail() {
            return Ok(OperationResult::failure("Simulated failure during create"));
        }

        let outputs = self.generate_outputs(&resource.resource_type, &resource.inputs);

        let mock_resource = MockResource {
            id: resource.id.to_string(),
            resource_type: resource.resource_type.as_str().to_string(),
            inputs: resource.inputs.clone(),
            outputs: outputs.clone(),
        };

        self.resources
            .write()
            .unwrap()
            .insert(resource.id.to_string(), mock_resource);

        debug!(
            "Created mock resource {} ({}) with id {}",
            resource.name,
            resource.resource_type.as_str(),
            resource.id
        );

        let mut updated_resource = resource.clone();
        updated_resource.outputs = outputs;

        Ok(OperationResult::success(updated_resource))
    }

    async fn read(&self, resource: &Resource) -> Result<OperationResult> {
        self.simulate_latency().await;

        if self.should_fail() {
            return Ok(OperationResult::failure("Simulated failure during read"));
        }

        let resources = self.resources.read().unwrap();
        match resources.get(&resource.id.to_string()) {
            Some(mock_resource) => {
                let mut updated_resource = resource.clone();
                updated_resource.outputs = mock_resource.outputs.clone();
                Ok(OperationResult::success(updated_resource))
            }
            None => Ok(OperationResult::failure("Resource not found")),
        }
    }

    async fn update(
        &self,
        resource: &Resource,
        new_inputs: PropertyValues,
    ) -> Result<OperationResult> {
        self.simulate_latency().await;

        if self.should_fail() {
            return Ok(OperationResult::failure("Simulated failure during update"));
        }

        let outputs = self.generate_outputs(&resource.resource_type, &new_inputs);

        let mut resources = self.resources.write().unwrap();
        if let Some(mock_resource) = resources.get_mut(&resource.id.to_string()) {
            mock_resource.inputs = new_inputs.clone();
            mock_resource.outputs = outputs.clone();
        }

        debug!(
            "Updated mock resource {} ({})",
            resource.name,
            resource.resource_type.as_str()
        );

        let mut updated_resource = resource.clone();
        updated_resource.inputs = new_inputs;
        updated_resource.outputs = outputs;

        Ok(OperationResult::success(updated_resource))
    }

    async fn delete(&self, resource: &Resource) -> Result<OperationResult> {
        self.simulate_latency().await;

        if self.should_fail() {
            return Ok(OperationResult::failure("Simulated failure during delete"));
        }

        self.resources
            .write()
            .unwrap()
            .remove(&resource.id.to_string());

        debug!(
            "Deleted mock resource {} ({})",
            resource.name,
            resource.resource_type.as_str()
        );

        Ok(OperationResult::success(resource.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_resource(name: &str, module: &str, type_name: &str, inputs: PropertyValues) -> Resource {
        Resource::new(
            "test-stack",
            ResourceType::new("mock", module, type_name),
            name,
            inputs,
        )
    }

    #[tokio::test]
    async fn test_create_bucket() {
        let provider = MockProvider::new();

        let mut inputs = PropertyValues::new();
        inputs.insert(
            "name".to_string(),
            PropertyValue::String("test-bucket".to_string()),
        );
        inputs.insert("versioning".to_string(), PropertyValue::Bool(true));

        let resource = create_test_resource("my-bucket", "storage", "Bucket", inputs);
        let result = provider.create(&resource).await.unwrap();

        assert!(result.success);
        assert!(result.resource.is_some());
        let created = result.resource.unwrap();
        assert!(created.outputs.contains_key("arn"));
        assert!(created.outputs.contains_key("endpoint"));
    }

    #[tokio::test]
    async fn test_create_function() {
        let provider = MockProvider::new();

        let mut inputs = PropertyValues::new();
        inputs.insert(
            "name".to_string(),
            PropertyValue::String("my-func".to_string()),
        );
        inputs.insert(
            "runtime".to_string(),
            PropertyValue::String("nodejs18.x".to_string()),
        );
        inputs.insert("memory".to_string(), PropertyValue::Int(256));

        let resource = create_test_resource("my-function", "compute", "Function", inputs);
        let result = provider.create(&resource).await.unwrap();

        assert!(result.success);
        let created = result.resource.unwrap();
        assert!(created.outputs.contains_key("arn"));
        assert!(created.outputs.contains_key("invoke_url"));
    }

    #[tokio::test]
    async fn test_update_resource() {
        let provider = MockProvider::new();

        let mut inputs = PropertyValues::new();
        inputs.insert(
            "name".to_string(),
            PropertyValue::String("test-db".to_string()),
        );
        inputs.insert(
            "engine".to_string(),
            PropertyValue::String("postgres".to_string()),
        );

        let resource = create_test_resource("my-db", "database", "Instance", inputs.clone());
        let create_result = provider.create(&resource).await.unwrap();
        assert!(create_result.success);

        // Update with new size
        let mut new_inputs = inputs.clone();
        new_inputs.insert(
            "size".to_string(),
            PropertyValue::String("large".to_string()),
        );

        let update_result = provider.update(&resource, new_inputs).await.unwrap();
        assert!(update_result.success);
        let updated = update_result.resource.unwrap();
        assert!(updated.outputs.contains_key("endpoint"));
    }

    #[tokio::test]
    async fn test_diff() {
        let provider = MockProvider::new();

        let mut old_inputs = PropertyValues::new();
        old_inputs.insert(
            "name".to_string(),
            PropertyValue::String("bucket".to_string()),
        );
        old_inputs.insert("versioning".to_string(), PropertyValue::Bool(false));

        let resource = create_test_resource("my-bucket", "storage", "Bucket", old_inputs);

        let mut new_inputs = PropertyValues::new();
        new_inputs.insert(
            "name".to_string(),
            PropertyValue::String("bucket".to_string()),
        );
        new_inputs.insert("versioning".to_string(), PropertyValue::Bool(true));

        let diff_result = provider.diff(&resource, new_inputs).await.unwrap();

        assert!(diff_result.changes.contains_key("versioning"));
        assert!(!diff_result.changes.contains_key("name"));
        assert!(diff_result.stable_keys.contains(&"name".to_string()));
    }

    #[tokio::test]
    async fn test_check_required_fields() {
        let provider = MockProvider::new();

        // Missing required field
        let inputs = PropertyValues::new();
        let resource_type = ResourceType::new("mock", "storage", "Bucket");

        let check_result = provider.check(&resource_type, inputs).await.unwrap();
        assert!(!check_result.failures.is_empty());
        assert!(check_result
            .failures
            .iter()
            .any(|f| f.property == "name"));
    }

    #[tokio::test]
    async fn test_delete_resource() {
        let provider = MockProvider::new();

        let mut inputs = PropertyValues::new();
        inputs.insert(
            "name".to_string(),
            PropertyValue::String("to-delete".to_string()),
        );

        let resource = create_test_resource("my-bucket", "storage", "Bucket", inputs);
        let create_result = provider.create(&resource).await.unwrap();
        assert!(create_result.success);

        // Delete it
        let delete_result = provider.delete(&resource).await.unwrap();
        assert!(delete_result.success);

        // Verify it's gone
        let read_result = provider.read(&resource).await.unwrap();
        assert!(!read_result.success);
    }

    #[tokio::test]
    async fn test_schema() {
        let provider = MockProvider::new();
        let schema = provider.schema().await.unwrap();

        assert_eq!(schema.name, "mock");
        assert!(schema.resources.contains_key("mock:storage:Bucket"));
        assert!(schema.resources.contains_key("mock:compute:Function"));
        assert!(schema.resources.contains_key("mock:database:Instance"));
    }
}
