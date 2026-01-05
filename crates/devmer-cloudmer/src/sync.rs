//! State synchronization with Cloudmer.

use std::collections::HashMap;

use chrono::Utc;

use crate::client::CloudmerClient;
use crate::error::Result;
use crate::types::{InfrastructureState, ResourceState, ResourceStatus, SyncResponse};

/// Builder for infrastructure state to sync.
#[derive(Debug, Default)]
pub struct StateSyncBuilder {
    stack_name: String,
    environment: Option<String>,
    resources: Vec<ResourceState>,
    metadata: HashMap<String, serde_json::Value>,
}

impl StateSyncBuilder {
    /// Create a new state sync builder.
    pub fn new(stack_name: impl Into<String>) -> Self {
        Self {
            stack_name: stack_name.into(),
            ..Default::default()
        }
    }

    /// Set the environment.
    pub fn environment(mut self, env: impl Into<String>) -> Self {
        self.environment = Some(env.into());
        self
    }

    /// Add a resource.
    pub fn add_resource(mut self, resource: ResourceState) -> Self {
        self.resources.push(resource);
        self
    }

    /// Add multiple resources.
    pub fn add_resources(mut self, resources: impl IntoIterator<Item = ResourceState>) -> Self {
        self.resources.extend(resources);
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl serde::Serialize) -> Self {
        self.metadata.insert(
            key.into(),
            serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
        );
        self
    }

    /// Build the infrastructure state.
    pub fn build(self) -> InfrastructureState {
        InfrastructureState {
            stack_name: self.stack_name,
            environment: self.environment,
            resources: self.resources,
            last_deployed_at: Some(Utc::now()),
            devmer_version: env!("CARGO_PKG_VERSION").to_string(),
            metadata: self.metadata,
        }
    }

    /// Sync the state to Cloudmer.
    pub async fn sync(self, client: &CloudmerClient) -> Result<SyncResponse> {
        let state = self.build();
        client.sync_state(&state).await
    }
}

/// Builder for resource state.
#[derive(Debug)]
pub struct ResourceStateBuilder {
    urn: String,
    resource_type: String,
    name: String,
    provider: String,
    region: Option<String>,
    provider_id: Option<String>,
    status: ResourceStatus,
    properties: HashMap<String, serde_json::Value>,
    tags: HashMap<String, String>,
    dependencies: Vec<String>,
}

impl ResourceStateBuilder {
    /// Create a new resource state builder.
    pub fn new(
        urn: impl Into<String>,
        resource_type: impl Into<String>,
        name: impl Into<String>,
        provider: impl Into<String>,
    ) -> Self {
        Self {
            urn: urn.into(),
            resource_type: resource_type.into(),
            name: name.into(),
            provider: provider.into(),
            region: None,
            provider_id: None,
            status: ResourceStatus::Active,
            properties: HashMap::new(),
            tags: HashMap::new(),
            dependencies: Vec::new(),
        }
    }

    /// Set the region.
    pub fn region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Set the provider-specific ID.
    pub fn provider_id(mut self, id: impl Into<String>) -> Self {
        self.provider_id = Some(id.into());
        self
    }

    /// Set the status.
    pub fn status(mut self, status: ResourceStatus) -> Self {
        self.status = status;
        self
    }

    /// Add a property.
    pub fn property(mut self, key: impl Into<String>, value: impl serde::Serialize) -> Self {
        self.properties.insert(
            key.into(),
            serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
        );
        self
    }

    /// Add a tag.
    pub fn tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Add a dependency.
    pub fn depends_on(mut self, urn: impl Into<String>) -> Self {
        self.dependencies.push(urn.into());
        self
    }

    /// Build the resource state.
    pub fn build(self) -> ResourceState {
        ResourceState {
            urn: self.urn,
            resource_type: self.resource_type,
            name: self.name,
            provider: self.provider,
            region: self.region,
            provider_id: self.provider_id,
            status: self.status,
            properties: self.properties,
            tags: self.tags,
            dependencies: self.dependencies,
            created_at: None,
            modified_at: Some(Utc::now()),
        }
    }
}

/// Convert Devmer state to Cloudmer format.
pub fn convert_devmer_state(
    stack_name: &str,
    environment: Option<&str>,
    resources: &[serde_json::Value],
) -> InfrastructureState {
    let mut builder = StateSyncBuilder::new(stack_name);

    if let Some(env) = environment {
        builder = builder.environment(env);
    }

    for resource in resources {
        if let Some(resource_state) = convert_resource(resource) {
            builder = builder.add_resource(resource_state);
        }
    }

    builder.build()
}

/// Convert a single Devmer resource to Cloudmer format.
fn convert_resource(resource: &serde_json::Value) -> Option<ResourceState> {
    let urn = resource.get("urn")?.as_str()?;
    let resource_type = resource.get("type")?.as_str()?;
    let name = resource.get("name")?.as_str().unwrap_or("unknown");

    // Extract provider from resource type (e.g., "aws:s3:Bucket" -> "aws")
    let provider = resource_type.split(':').next().unwrap_or("unknown");

    let mut builder = ResourceStateBuilder::new(urn, resource_type, name, provider);

    // Extract region if present
    if let Some(region) = resource.get("region").and_then(|v| v.as_str()) {
        builder = builder.region(region);
    }

    // Extract provider ID if present
    if let Some(id) = resource.get("id").and_then(|v| v.as_str()) {
        builder = builder.provider_id(id);
    }

    // Extract properties
    if let Some(outputs) = resource.get("outputs").and_then(|v| v.as_object()) {
        for (key, value) in outputs {
            builder = builder.property(key, value.clone());
        }
    }

    // Extract tags
    if let Some(tags) = resource.get("tags").and_then(|v| v.as_object()) {
        for (key, value) in tags {
            if let Some(v) = value.as_str() {
                builder = builder.tag(key, v);
            }
        }
    }

    // Extract dependencies
    if let Some(deps) = resource.get("dependencies").and_then(|v| v.as_array()) {
        for dep in deps {
            if let Some(dep_urn) = dep.as_str() {
                builder = builder.depends_on(dep_urn);
            }
        }
    }

    Some(builder.build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_sync_builder() {
        let resource = ResourceStateBuilder::new(
            "urn:devmer:production::aws:s3:Bucket::my-bucket",
            "aws:s3:Bucket",
            "my-bucket",
            "aws",
        )
        .region("us-east-1")
        .provider_id("my-bucket-12345")
        .tag("environment", "production")
        .property("versioning", true)
        .build();

        let state = StateSyncBuilder::new("production")
            .environment("prod")
            .add_resource(resource)
            .with_metadata("git_commit", "abc123")
            .build();

        assert_eq!(state.stack_name, "production");
        assert_eq!(state.environment, Some("prod".to_string()));
        assert_eq!(state.resources.len(), 1);
    }

    #[test]
    fn test_convert_devmer_resource() {
        let resource = serde_json::json!({
            "urn": "urn:devmer:production::aws:s3:Bucket::my-bucket",
            "type": "aws:s3:Bucket",
            "name": "my-bucket",
            "region": "us-east-1",
            "id": "my-bucket-12345",
            "outputs": {
                "arn": "arn:aws:s3:::my-bucket",
                "bucketDomainName": "my-bucket.s3.amazonaws.com"
            },
            "tags": {
                "environment": "production",
                "team": "platform"
            },
            "dependencies": ["urn:devmer:production::aws:iam:Role::bucket-role"]
        });

        let state = convert_resource(&resource).unwrap();

        assert_eq!(state.urn, "urn:devmer:production::aws:s3:Bucket::my-bucket");
        assert_eq!(state.provider, "aws");
        assert_eq!(state.region, Some("us-east-1".to_string()));
        assert_eq!(state.tags.get("environment"), Some(&"production".to_string()));
        assert_eq!(state.dependencies.len(), 1);
    }
}
