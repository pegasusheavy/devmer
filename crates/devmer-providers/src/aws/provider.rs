//! AWS Provider implementation

use super::config::AwsConfig;
use super::resources::{compute_diff, generate_outputs, validate_inputs};
use super::schemas::all_schemas;
use async_trait::async_trait;
use devmer_core::provider::{
    CheckFailure, CheckResult, DiffKind, DiffResult, OperationResult, PropertyDiff,
    PropertySchema, PropertyType, Provider, ProviderConfig, ProviderSchema, ResourceSchema,
};
use devmer_core::resource::{Resource, ResourceType};
use devmer_core::types::{PropertyValue, PropertyValues};
use devmer_core::Result;
use std::collections::HashMap;
use std::sync::RwLock;
use tracing::{debug, info, warn};

/// AWS Provider
///
/// Implements the Devmer Provider trait for AWS resources.
/// Currently uses mock implementations but can be extended to use
/// the real AWS SDK for production use.
pub struct AwsProvider {
    /// Provider configuration
    config: RwLock<AwsConfig>,

    /// Cached resource schemas
    schemas: HashMap<String, ResourceSchema>,

    /// Provider version
    version: String,
}

impl AwsProvider {
    /// Create a new AWS provider with default configuration
    pub fn new() -> Self {
        Self {
            config: RwLock::new(AwsConfig::from_env()),
            schemas: all_schemas(),
            version: "0.1.0".to_string(),
        }
    }

    /// Create a new AWS provider with specific configuration
    pub fn with_config(config: AwsConfig) -> Self {
        Self {
            config: RwLock::new(config),
            schemas: all_schemas(),
            version: "0.1.0".to_string(),
        }
    }

    /// Get the current region
    pub fn region(&self) -> String {
        self.config.read().unwrap().region.0.clone()
    }

    /// Check if credentials are available
    pub fn has_credentials(&self) -> bool {
        self.config.read().unwrap().credentials.is_available()
    }
}

impl Default for AwsProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Provider for AwsProvider {
    fn name(&self) -> &str {
        "aws"
    }

    fn version(&self) -> &str {
        &self.version
    }

    async fn schema(&self) -> Result<ProviderSchema> {
        // Configuration schema
        let mut config_props = HashMap::new();
        config_props.insert(
            "region".to_string(),
            PropertySchema {
                property_type: PropertyType::String,
                description: Some("The AWS region to use".to_string()),
                default: Some(serde_json::json!("us-east-1")),
                secret: false,
                replace_on_change: false,
                deprecated: None,
            },
        );
        config_props.insert(
            "accessKey".to_string(),
            PropertySchema {
                property_type: PropertyType::String,
                description: Some("AWS access key ID".to_string()),
                default: None,
                secret: true,
                replace_on_change: false,
                deprecated: None,
            },
        );
        config_props.insert(
            "secretKey".to_string(),
            PropertySchema {
                property_type: PropertyType::String,
                description: Some("AWS secret access key".to_string()),
                default: None,
                secret: true,
                replace_on_change: false,
                deprecated: None,
            },
        );
        config_props.insert(
            "profile".to_string(),
            PropertySchema {
                property_type: PropertyType::String,
                description: Some("AWS profile to use from credentials file".to_string()),
                default: None,
                secret: false,
                replace_on_change: false,
                deprecated: None,
            },
        );
        config_props.insert(
            "assumeRoleArn".to_string(),
            PropertySchema {
                property_type: PropertyType::String,
                description: Some("ARN of role to assume".to_string()),
                default: None,
                secret: false,
                replace_on_change: false,
                deprecated: None,
            },
        );
        config_props.insert(
            "defaultTags".to_string(),
            PropertySchema {
                property_type: PropertyType::Object(HashMap::new()),
                description: Some("Default tags to apply to all resources".to_string()),
                default: None,
                secret: false,
                replace_on_change: false,
                deprecated: None,
            },
        );

        let config_schema = ResourceSchema {
            resource_type: ResourceType::new("aws", "config", "Provider"),
            description: Some("AWS provider configuration".to_string()),
            input_properties: config_props,
            output_properties: HashMap::new(),
            required: vec![],
        };

        Ok(ProviderSchema {
            name: "aws".to_string(),
            version: self.version.clone(),
            description: Some(
                "The AWS provider for Devmer allows you to manage AWS infrastructure resources."
                    .to_string(),
            ),
            resources: self.schemas.clone(),
            config: Some(config_schema),
        })
    }

    async fn configure(&mut self, config: ProviderConfig) -> Result<()> {
        info!(provider = "aws", "Configuring AWS provider");

        let mut aws_config = self.config.write().unwrap();

        // Apply configuration values
        if let Some(region) = config.config.get("region") {
            if let Some(r) = region.as_str() {
                aws_config.region = super::config::AwsRegion(r.to_string());
                debug!(region = %r, "Set AWS region");
            }
        }

        if let Some(access_key) = config.config.get("accessKey") {
            if let Some(k) = access_key.as_str() {
                aws_config.credentials.access_key_id = Some(k.to_string());
            }
        }

        if let Some(secret_key) = config.config.get("secretKey") {
            if let Some(k) = secret_key.as_str() {
                aws_config.credentials.secret_access_key = Some(k.to_string());
            }
        }

        if let Some(profile) = config.config.get("profile") {
            if let Some(p) = profile.as_str() {
                aws_config.credentials.profile = Some(p.to_string());
            }
        }

        if let Some(role_arn) = config.config.get("assumeRoleArn") {
            if let Some(arn) = role_arn.as_str() {
                aws_config.credentials.assume_role_arn = Some(arn.to_string());
            }
        }

        if let Some(default_tags) = config.config.get("defaultTags") {
            if let PropertyValue::Object(tags) = default_tags {
                for (key, value) in tags {
                    if let Some(v) = value.as_str() {
                        aws_config.default_tags.insert(key.clone(), v.to_string());
                    }
                }
            }
        }

        // Validate credentials if not skipped
        if !aws_config.skip_credentials_validation && !aws_config.credentials.is_available() {
            warn!("No AWS credentials available. API calls will fail.");
        }

        Ok(())
    }

    async fn check(
        &self,
        resource_type: &ResourceType,
        inputs: PropertyValues,
    ) -> Result<CheckResult> {
        let type_str = resource_type.as_str();
        debug!(resource_type = %type_str, "Checking resource inputs");

        // Get schema for validation
        let schema = self.schemas.get(type_str);

        let mut normalized_inputs = inputs.clone();
        let mut failures = Vec::new();

        // Check required properties
        if let Some(schema) = schema {
            for required in &schema.required {
                if !normalized_inputs.contains_key(required) {
                    failures.push(CheckFailure {
                        property: required.clone(),
                        reason: format!("Required property '{}' is missing", required),
                    });
                }
            }

            // Apply defaults
            for (key, prop_schema) in &schema.input_properties {
                if !normalized_inputs.contains_key(key) {
                    if let Some(default) = &prop_schema.default {
                        normalized_inputs.insert(key.clone(), json_to_property_value(default));
                    }
                }
            }
        }

        // Resource-specific validation
        let validation_errors = validate_inputs(type_str, &normalized_inputs);
        for (property, reason) in validation_errors {
            failures.push(CheckFailure { property, reason });
        }

        // Add default tags
        let config = self.config.read().unwrap();
        if !config.default_tags.is_empty() && normalized_inputs.contains_key("tags") {
            if let Some(PropertyValue::Object(tags)) = normalized_inputs.get_mut("tags") {
                for (key, value) in &config.default_tags {
                    if !tags.contains_key(key) {
                        tags.insert(key.clone(), PropertyValue::String(value.clone()));
                    }
                }
            }
        } else if !config.default_tags.is_empty() {
            let tags: PropertyValues = config
                .default_tags
                .iter()
                .map(|(k, v)| (k.clone(), PropertyValue::String(v.clone())))
                .collect();
            normalized_inputs.insert("tags".to_string(), PropertyValue::Object(tags));
        }

        Ok(CheckResult {
            inputs: normalized_inputs,
            failures,
        })
    }

    async fn diff(
        &self,
        resource: &Resource,
        new_inputs: PropertyValues,
    ) -> Result<DiffResult> {
        let type_str = resource.resource_type.as_str();
        debug!(
            resource_type = %type_str,
            urn = %resource.urn,
            "Computing diff"
        );

        let (changes_map, replace, replace_keys) =
            compute_diff(type_str, &resource.inputs, &new_inputs);

        let changes: HashMap<String, PropertyDiff> = changes_map
            .into_iter()
            .map(|(key, change_type)| {
                let kind = match change_type.as_str() {
                    "add" => DiffKind::Add,
                    "delete" => DiffKind::Delete,
                    "update" if replace_keys.contains(&key) => DiffKind::UpdateReplace,
                    _ => DiffKind::Update,
                };

                (
                    key,
                    PropertyDiff {
                        kind,
                        input_diff: true,
                        output_diff: false,
                    },
                )
            })
            .collect();

        // Stable keys are those that didn't change
        let all_keys: std::collections::HashSet<_> = resource
            .inputs
            .keys()
            .chain(new_inputs.keys())
            .collect();
        let stable_keys: Vec<String> = all_keys
            .into_iter()
            .filter(|k| !changes.contains_key(*k))
            .cloned()
            .collect();

        Ok(DiffResult {
            changes,
            replace,
            replace_keys,
            stable_keys,
        })
    }

    async fn create(&self, resource: &Resource) -> Result<OperationResult> {
        let type_str = resource.resource_type.as_str();
        info!(
            resource_type = %type_str,
            name = %resource.name,
            urn = %resource.urn,
            "Creating resource"
        );

        // In a real implementation, this would call AWS APIs
        // For now, generate mock outputs
        let outputs = generate_outputs(resource);

        // Create the result resource with outputs
        let mut result_resource = resource.clone();
        result_resource.outputs = outputs;

        Ok(OperationResult::success(result_resource))
    }

    async fn read(&self, resource: &Resource) -> Result<OperationResult> {
        let type_str = resource.resource_type.as_str();
        debug!(
            resource_type = %type_str,
            urn = %resource.urn,
            "Reading resource"
        );

        // In a real implementation, this would call AWS APIs to get current state
        // For now, return the resource as-is (simulating no drift)
        let outputs = if resource.outputs.is_empty() {
            generate_outputs(resource)
        } else {
            resource.outputs.clone()
        };

        let mut result_resource = resource.clone();
        result_resource.outputs = outputs;

        Ok(OperationResult::success(result_resource))
    }

    async fn update(
        &self,
        resource: &Resource,
        new_inputs: PropertyValues,
    ) -> Result<OperationResult> {
        let type_str = resource.resource_type.as_str();
        info!(
            resource_type = %type_str,
            name = %resource.name,
            urn = %resource.urn,
            "Updating resource"
        );

        // In a real implementation, this would call AWS update APIs
        let mut result_resource = resource.clone();
        result_resource.inputs = new_inputs;

        // Regenerate outputs (some might change based on inputs)
        result_resource.outputs = generate_outputs(&result_resource);

        Ok(OperationResult::success(result_resource))
    }

    async fn delete(&self, resource: &Resource) -> Result<OperationResult> {
        let type_str = resource.resource_type.as_str();
        info!(
            resource_type = %type_str,
            name = %resource.name,
            urn = %resource.urn,
            "Deleting resource"
        );

        // In a real implementation, this would call AWS delete APIs
        Ok(OperationResult::success(resource.clone()))
    }
}

/// Convert JSON value to PropertyValue
fn json_to_property_value(value: &serde_json::Value) -> PropertyValue {
    match value {
        serde_json::Value::Null => PropertyValue::Null,
        serde_json::Value::Bool(b) => PropertyValue::Bool(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                PropertyValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                PropertyValue::Float(f)
            } else {
                PropertyValue::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => PropertyValue::String(s.clone()),
        serde_json::Value::Array(arr) => {
            PropertyValue::Array(arr.iter().map(json_to_property_value).collect())
        }
        serde_json::Value::Object(obj) => PropertyValue::Object(
            obj.iter()
                .map(|(k, v)| (k.clone(), json_to_property_value(v)))
                .collect(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_aws_provider_schema() {
        let provider = AwsProvider::new();
        let schema = provider.schema().await.unwrap();

        assert_eq!(schema.name, "aws");
        assert!(!schema.resources.is_empty());
        assert!(schema.resources.contains_key("aws:s3:Bucket"));
        assert!(schema.resources.contains_key("aws:lambda:Function"));
        assert!(schema.resources.contains_key("aws:iam:Role"));
    }

    #[tokio::test]
    async fn test_check_s3_bucket() {
        let provider = AwsProvider::new();

        let mut inputs = PropertyValues::new();
        inputs.insert("bucket".to_string(), PropertyValue::String("my-test-bucket".to_string()));

        let result = provider
            .check(&ResourceType::new("aws", "s3", "Bucket"), inputs)
            .await
            .unwrap();

        assert!(result.failures.is_empty());
        assert!(result.inputs.contains_key("bucket"));
    }

    #[tokio::test]
    async fn test_check_missing_required() {
        let provider = AwsProvider::new();

        let inputs = PropertyValues::new(); // Missing required 'bucket'

        let result = provider
            .check(&ResourceType::new("aws", "s3", "Bucket"), inputs)
            .await
            .unwrap();

        assert!(!result.failures.is_empty());
        assert!(result.failures.iter().any(|f| f.property == "bucket"));
    }

    #[tokio::test]
    async fn test_check_invalid_bucket_name() {
        let provider = AwsProvider::new();

        let mut inputs = PropertyValues::new();
        inputs.insert("bucket".to_string(), PropertyValue::String("ab".to_string())); // Too short

        let result = provider
            .check(&ResourceType::new("aws", "s3", "Bucket"), inputs)
            .await
            .unwrap();

        assert!(result.failures.iter().any(|f| f.property == "bucket"));
    }

    #[tokio::test]
    async fn test_create_resource() {
        let provider = AwsProvider::new();

        let mut inputs = PropertyValues::new();
        inputs.insert(
            "bucket".to_string(),
            PropertyValue::String("my-test-bucket".to_string()),
        );

        let resource = Resource::new(
            "test-project",
            ResourceType::new("aws", "s3", "Bucket"),
            "my-bucket",
            inputs,
        );

        let result = provider.create(&resource).await.unwrap();

        assert!(result.success);
        let created = result.resource.unwrap();
        assert!(created.outputs.contains_key("arn"));
        assert!(created.outputs.contains_key("bucketDomainName"));
    }

    #[tokio::test]
    async fn test_diff_no_changes() {
        let provider = AwsProvider::new();

        let mut inputs = PropertyValues::new();
        inputs.insert(
            "bucket".to_string(),
            PropertyValue::String("my-bucket".to_string()),
        );

        let resource = Resource::new(
            "test-project",
            ResourceType::new("aws", "s3", "Bucket"),
            "my-bucket",
            inputs.clone(),
        );

        let result = provider.diff(&resource, inputs).await.unwrap();

        assert!(result.changes.is_empty());
        assert!(!result.replace);
    }

    #[tokio::test]
    async fn test_diff_with_update() {
        let provider = AwsProvider::new();

        let mut old_inputs = PropertyValues::new();
        old_inputs.insert(
            "bucket".to_string(),
            PropertyValue::String("my-bucket".to_string()),
        );

        let resource = Resource::new(
            "test-project",
            ResourceType::new("aws", "s3", "Bucket"),
            "my-bucket",
            old_inputs,
        );

        let mut new_inputs = PropertyValues::new();
        new_inputs.insert(
            "bucket".to_string(),
            PropertyValue::String("my-bucket".to_string()),
        );
        new_inputs.insert(
            "acl".to_string(),
            PropertyValue::String("public-read".to_string()),
        );

        let result = provider.diff(&resource, new_inputs).await.unwrap();

        assert!(result.changes.contains_key("acl"));
        assert!(!result.replace); // acl doesn't force replacement
    }

    #[tokio::test]
    async fn test_diff_with_replacement() {
        let provider = AwsProvider::new();

        let mut old_inputs = PropertyValues::new();
        old_inputs.insert(
            "bucket".to_string(),
            PropertyValue::String("my-bucket".to_string()),
        );

        let resource = Resource::new(
            "test-project",
            ResourceType::new("aws", "s3", "Bucket"),
            "my-bucket",
            old_inputs,
        );

        let mut new_inputs = PropertyValues::new();
        new_inputs.insert(
            "bucket".to_string(),
            PropertyValue::String("new-bucket-name".to_string()),
        ); // Different name

        let result = provider.diff(&resource, new_inputs).await.unwrap();

        assert!(result.changes.contains_key("bucket"));
        assert!(result.replace); // bucket name forces replacement
        assert!(result.replace_keys.contains(&"bucket".to_string()));
    }
}
