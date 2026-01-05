//! Runtime context for exposing services to scripting languages
//!
//! This module provides a context object that can be passed to script runtimes,
//! giving them access to configuration, state, and resource registration without
//! creating circular dependencies between the runtime and DI crates.

use crate::registry::{
    ComponentOptions, RegisteredComponent, RegisteredResource, RegisteredResourceOptions,
    ResourceRegistry, StackReference,
};
use async_trait::async_trait;
use devmer_core::types::{PropertyValue, PropertyValues};
use std::collections::HashMap;
use std::sync::Arc;

/// Service provider trait for configuration access
/// This is implemented by the DI layer and passed to the runtime
pub trait ConfigProvider: Send + Sync {
    /// Get a configuration value by key
    fn get(&self, key: &str) -> Option<String>;

    /// Get all configuration for a namespace
    fn get_namespace(&self, namespace: &str) -> HashMap<String, String>;

    /// Get the project name
    fn project_name(&self) -> &str;

    /// Get the current stack name
    fn stack_name(&self) -> &str;
}

/// Service provider trait for secrets access
#[async_trait]
pub trait SecretsProvider: Send + Sync {
    /// Get a secret value
    async fn get_secret(&self, name: &str) -> Result<Option<String>, String>;

    /// Check if a secret exists
    async fn has_secret(&self, name: &str) -> bool;
}

/// Service provider trait for resource operations
/// Scripts use this to register resources they want to create/manage
pub trait ResourceProvider: Send + Sync {
    /// Register a resource
    fn register_resource(
        &self,
        resource_type: &str,
        name: &str,
        inputs: PropertyValues,
        options: RegisteredResourceOptions,
    ) -> String; // Returns URN

    /// Get a resource by URN
    fn get_resource(&self, urn: &str) -> Option<RegisteredResource>;

    /// Export a stack output
    fn export_output(&self, name: &str, value: PropertyValue);

    /// Get the resource registry (for collecting results)
    fn registry(&self) -> &ResourceRegistry;

    // --- Component Operations ---

    /// Register a component resource
    fn register_component(
        &self,
        component_type: &str,
        name: &str,
        inputs: PropertyValues,
        options: ComponentOptions,
    ) -> String; // Returns URN

    /// Get a component by URN
    fn get_component(&self, urn: &str) -> Option<RegisteredComponent>;

    /// Enter a component context (child resources will be parented to this component)
    fn enter_component(&self, urn: &str);

    /// Exit the current component context
    fn exit_component(&self);

    /// Get the current component context URN
    fn current_component(&self) -> Option<String>;

    /// Set component outputs
    fn set_component_outputs(&self, urn: &str, outputs: PropertyValues);

    /// Get component outputs
    fn get_component_outputs(&self, urn: &str) -> Option<PropertyValues>;

    /// Get child resources of a component
    fn get_children(&self, parent_urn: &str) -> Vec<RegisteredResource>;

    // --- Stack References ---

    /// Register a reference to another stack's outputs
    fn register_stack_reference(&self, name: &str, stack_name: &str, output_key: Option<&str>);

    /// Get a stack reference
    fn get_stack_reference(&self, name: &str) -> Option<StackReference>;
}

/// Runtime context that provides access to services for scripts
/// 
/// This is the main interface that scripting languages use to interact
/// with the Devmer system. It abstracts away the DI container details.
pub struct RuntimeContext {
    /// Configuration provider
    config: Arc<dyn ConfigProvider>,

    /// Secrets provider (optional)
    secrets: Option<Arc<dyn SecretsProvider>>,

    /// Resource provider
    resources: Arc<dyn ResourceProvider>,

    /// Current stack name
    stack: String,

    /// Whether running in preview mode
    preview: bool,

    /// Custom variables accessible to scripts
    variables: HashMap<String, PropertyValue>,
}

impl RuntimeContext {
    /// Create a new runtime context
    pub fn new(
        config: Arc<dyn ConfigProvider>,
        resources: Arc<dyn ResourceProvider>,
        stack: &str,
        preview: bool,
    ) -> Self {
        Self {
            config,
            secrets: None,
            resources,
            stack: stack.to_string(),
            preview,
            variables: HashMap::new(),
        }
    }

    /// Add a secrets provider
    pub fn with_secrets(mut self, secrets: Arc<dyn SecretsProvider>) -> Self {
        self.secrets = Some(secrets);
        self
    }

    /// Add a custom variable
    pub fn with_variable(mut self, name: impl Into<String>, value: PropertyValue) -> Self {
        self.variables.insert(name.into(), value);
        self
    }

    /// Get the current stack name
    pub fn stack(&self) -> &str {
        &self.stack
    }

    /// Check if running in preview mode
    pub fn is_preview(&self) -> bool {
        self.preview
    }

    /// Get the project name
    pub fn project_name(&self) -> &str {
        self.config.project_name()
    }

    // --- Configuration Access ---

    /// Get a configuration value
    pub fn config_get(&self, key: &str) -> Option<String> {
        self.config.get(key)
    }

    /// Get a configuration value with a default
    pub fn config_get_or(&self, key: &str, default: &str) -> String {
        self.config.get(key).unwrap_or_else(|| default.to_string())
    }

    /// Get configuration namespace
    pub fn config_namespace(&self, namespace: &str) -> HashMap<String, String> {
        self.config.get_namespace(namespace)
    }

    // --- Secrets Access ---

    /// Get a secret value (async)
    pub async fn secret_get(&self, name: &str) -> Result<Option<String>, String> {
        match &self.secrets {
            Some(provider) => provider.get_secret(name).await,
            None => Err("Secrets provider not configured".to_string()),
        }
    }

    /// Check if a secret exists
    pub async fn secret_exists(&self, name: &str) -> bool {
        match &self.secrets {
            Some(provider) => provider.has_secret(name).await,
            None => false,
        }
    }

    // --- Resource Operations ---

    /// Register a resource to be created
    pub fn resource(
        &self,
        resource_type: &str,
        name: &str,
        inputs: PropertyValues,
    ) -> String {
        self.resources.register_resource(
            resource_type,
            name,
            inputs,
            RegisteredResourceOptions::default(),
        )
    }

    /// Register a resource with options
    pub fn resource_with_options(
        &self,
        resource_type: &str,
        name: &str,
        inputs: PropertyValues,
        options: RegisteredResourceOptions,
    ) -> String {
        self.resources.register_resource(resource_type, name, inputs, options)
    }

    /// Get a registered resource
    pub fn get_resource(&self, urn: &str) -> Option<RegisteredResource> {
        self.resources.get_resource(urn)
    }

    /// Export a stack output
    pub fn output(&self, name: &str, value: PropertyValue) {
        self.resources.export_output(name, value);
    }

    /// Get the resource registry
    pub fn registry(&self) -> &ResourceRegistry {
        self.resources.registry()
    }

    // --- Component Operations ---

    /// Register a component resource
    pub fn component(
        &self,
        component_type: &str,
        name: &str,
        inputs: PropertyValues,
    ) -> String {
        self.resources.register_component(
            component_type,
            name,
            inputs,
            ComponentOptions::default(),
        )
    }

    /// Register a component with options
    pub fn component_with_options(
        &self,
        component_type: &str,
        name: &str,
        inputs: PropertyValues,
        options: ComponentOptions,
    ) -> String {
        self.resources.register_component(component_type, name, inputs, options)
    }

    /// Get a registered component
    pub fn get_component(&self, urn: &str) -> Option<RegisteredComponent> {
        self.resources.get_component(urn)
    }

    /// Enter a component context - child resources will be parented to this component
    pub fn enter_component(&self, urn: &str) {
        self.resources.enter_component(urn);
    }

    /// Exit the current component context
    pub fn exit_component(&self) {
        self.resources.exit_component();
    }

    /// Get the current component context
    pub fn current_component(&self) -> Option<String> {
        self.resources.current_component()
    }

    /// Set component outputs
    pub fn set_component_outputs(&self, urn: &str, outputs: PropertyValues) {
        self.resources.set_component_outputs(urn, outputs);
    }

    /// Get component outputs
    pub fn get_component_outputs(&self, urn: &str) -> Option<PropertyValues> {
        self.resources.get_component_outputs(urn)
    }

    /// Get child resources of a component
    pub fn get_children(&self, parent_urn: &str) -> Vec<RegisteredResource> {
        self.resources.get_children(parent_urn)
    }

    // --- Stack References ---

    /// Reference another stack's outputs
    pub fn stack_reference(&self, name: &str, stack_name: &str, output_key: Option<&str>) {
        self.resources.register_stack_reference(name, stack_name, output_key);
    }

    /// Get a stack reference
    pub fn get_stack_reference(&self, name: &str) -> Option<StackReference> {
        self.resources.get_stack_reference(name)
    }

    // --- Variables ---

    /// Get a custom variable
    pub fn var(&self, name: &str) -> Option<&PropertyValue> {
        self.variables.get(name)
    }

    /// Get a string variable
    pub fn var_str(&self, name: &str) -> Option<&str> {
        self.variables.get(name).and_then(|v| match v {
            PropertyValue::String(s) => Some(s.as_str()),
            _ => None,
        })
    }
}

/// Default implementations for providers used in testing or when
/// DI is not available

/// Simple in-memory config provider for testing
pub struct SimpleConfigProvider {
    project: String,
    stack: String,
    values: HashMap<String, String>,
}

impl SimpleConfigProvider {
    pub fn new(project: &str, stack: &str) -> Self {
        Self {
            project: project.to_string(),
            stack: stack.to_string(),
            values: HashMap::new(),
        }
    }

    pub fn with_value(mut self, key: &str, value: &str) -> Self {
        self.values.insert(key.to_string(), value.to_string());
        self
    }
}

impl ConfigProvider for SimpleConfigProvider {
    fn get(&self, key: &str) -> Option<String> {
        self.values.get(key).cloned()
    }

    fn get_namespace(&self, namespace: &str) -> HashMap<String, String> {
        let prefix = format!("{}:", namespace);
        self.values
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(k, v)| (k[prefix.len()..].to_string(), v.clone()))
            .collect()
    }

    fn project_name(&self) -> &str {
        &self.project
    }

    fn stack_name(&self) -> &str {
        &self.stack
    }
}

/// Simple resource provider that uses a ResourceRegistry
pub struct SimpleResourceProvider {
    registry: ResourceRegistry,
    project: String,
    stack: String,
}

impl SimpleResourceProvider {
    pub fn new(project: &str, stack: &str) -> Self {
        Self {
            registry: ResourceRegistry::new(),
            project: project.to_string(),
            stack: stack.to_string(),
        }
    }

    fn make_urn(&self, type_name: &str, name: &str) -> String {
        format!(
            "urn:devmer:{}::{}::{}::{}",
            self.stack, self.project, type_name, name
        )
    }
}

impl ResourceProvider for SimpleResourceProvider {
    fn register_resource(
        &self,
        resource_type: &str,
        name: &str,
        inputs: PropertyValues,
        options: RegisteredResourceOptions,
    ) -> String {
        let urn = self.make_urn(resource_type, name);

        let resource = RegisteredResource {
            urn: urn.clone(),
            resource_type: resource_type.to_string(),
            name: name.to_string(),
            inputs,
            options,
            parent: None,
            is_component: false,
        };

        self.registry.register_resource(resource);
        urn
    }

    fn get_resource(&self, urn: &str) -> Option<RegisteredResource> {
        self.registry.get_resource(urn)
    }

    fn export_output(&self, name: &str, value: PropertyValue) {
        let json_value = property_value_to_json(&value);
        self.registry.export(name, json_value);
    }

    fn registry(&self) -> &ResourceRegistry {
        &self.registry
    }

    // --- Component Operations ---

    fn register_component(
        &self,
        component_type: &str,
        name: &str,
        inputs: PropertyValues,
        options: ComponentOptions,
    ) -> String {
        let urn = self.make_urn(component_type, name);

        let component = RegisteredComponent {
            urn: urn.clone(),
            component_type: component_type.to_string(),
            name: name.to_string(),
            inputs,
            options,
            parent: None,
            children: vec![],
            outputs: PropertyValues::new(),
            state: HashMap::new(),
        };

        self.registry.register_component(component)
    }

    fn get_component(&self, urn: &str) -> Option<RegisteredComponent> {
        self.registry.get_component(urn)
    }

    fn enter_component(&self, urn: &str) {
        self.registry.enter_component(urn);
    }

    fn exit_component(&self) {
        self.registry.exit_component();
    }

    fn current_component(&self) -> Option<String> {
        self.registry.current_component()
    }

    fn set_component_outputs(&self, urn: &str, outputs: PropertyValues) {
        self.registry.set_component_outputs(urn, outputs);
    }

    fn get_component_outputs(&self, urn: &str) -> Option<PropertyValues> {
        self.registry.get_component_outputs(urn)
    }

    fn get_children(&self, parent_urn: &str) -> Vec<RegisteredResource> {
        self.registry.get_children(parent_urn)
    }

    // --- Stack References ---

    fn register_stack_reference(&self, name: &str, stack_name: &str, output_key: Option<&str>) {
        let reference = StackReference {
            stack_name: stack_name.to_string(),
            output_key: output_key.map(|s| s.to_string()),
            cached_value: None,
        };
        self.registry.register_stack_reference(name, reference);
    }

    fn get_stack_reference(&self, name: &str) -> Option<StackReference> {
        self.registry.get_stack_reference(name)
    }
}

/// Convert PropertyValue to serde_json::Value
fn property_value_to_json(value: &PropertyValue) -> serde_json::Value {
    match value {
        PropertyValue::Null => serde_json::Value::Null,
        PropertyValue::Bool(b) => serde_json::Value::Bool(*b),
        PropertyValue::Int(i) => serde_json::json!(i),
        PropertyValue::Float(f) => serde_json::json!(f),
        PropertyValue::String(s) => serde_json::Value::String(s.clone()),
        PropertyValue::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(property_value_to_json).collect())
        }
        PropertyValue::Object(obj) => {
            serde_json::Value::Object(
                obj.iter()
                    .map(|(k, v)| (k.clone(), property_value_to_json(v)))
                    .collect(),
            )
        }
        PropertyValue::Secret(inner) => {
            // Secrets are redacted in JSON output
            serde_json::Value::String("[SECRET]".to_string())
        }
        PropertyValue::OutputRef(output_ref) => {
            serde_json::json!({
                "$ref": output_ref.urn,
                "property": output_ref.property
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_config_provider() {
        let provider = SimpleConfigProvider::new("test-project", "dev")
            .with_value("aws:region", "us-east-1")
            .with_value("aws:profile", "default")
            .with_value("app:name", "myapp");

        assert_eq!(provider.project_name(), "test-project");
        assert_eq!(provider.stack_name(), "dev");
        assert_eq!(provider.get("aws:region"), Some("us-east-1".to_string()));
        assert_eq!(provider.get("nonexistent"), None);

        let aws_namespace = provider.get_namespace("aws");
        assert_eq!(aws_namespace.get("region"), Some(&"us-east-1".to_string()));
        assert_eq!(aws_namespace.get("profile"), Some(&"default".to_string()));
    }

    #[test]
    fn test_simple_resource_provider() {
        let provider = SimpleResourceProvider::new("test-project", "dev");

        let mut inputs = PropertyValues::new();
        inputs.insert("name".to_string(), PropertyValue::String("my-bucket".to_string()));

        let urn = provider.register_resource(
            "aws:s3:Bucket",
            "my-bucket",
            inputs,
            RegisteredResourceOptions::default(),
        );

        assert!(urn.contains("aws:s3:Bucket"));
        assert!(urn.contains("my-bucket"));

        let resource = provider.get_resource(&urn);
        assert!(resource.is_some());
        assert_eq!(resource.unwrap().name, "my-bucket");
    }

    #[test]
    fn test_runtime_context() {
        let config = Arc::new(SimpleConfigProvider::new("test", "dev"));
        let resources = Arc::new(SimpleResourceProvider::new("test", "dev"));

        let ctx = RuntimeContext::new(config, resources, "dev", false)
            .with_variable("version", PropertyValue::String("1.0.0".to_string()));

        assert_eq!(ctx.stack(), "dev");
        assert!(!ctx.is_preview());
        assert_eq!(ctx.var_str("version"), Some("1.0.0"));
    }
}
