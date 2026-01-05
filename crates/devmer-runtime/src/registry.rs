//! Resource registry for collecting resources during program execution
//!
//! This module provides registration for:
//! - **Resources** - Individual cloud resources (S3 buckets, Lambda functions, etc.)
//! - **ComponentResources** - Reusable abstractions that group multiple resources
//! - **StackReferences** - References to outputs from other stacks

use devmer_core::resource::{Resource, ResourceOptions, ResourceType, Urn};
use devmer_core::types::PropertyValues;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Registry for collecting resources during program execution
#[derive(Debug, Clone, Default)]
pub struct ResourceRegistry {
    inner: Arc<RwLock<RegistryInner>>,
}

#[derive(Debug, Default)]
struct RegistryInner {
    /// Registered resources (including component children)
    resources: HashMap<String, RegisteredResource>,

    /// Registered components
    components: HashMap<String, RegisteredComponent>,

    /// Stack references
    stack_references: HashMap<String, StackReference>,

    /// Resource outputs (populated after creation)
    outputs: HashMap<String, PropertyValues>,

    /// Stack outputs (exports)
    stack_outputs: HashMap<String, serde_json::Value>,

    /// Configuration values
    config: HashMap<String, serde_json::Value>,

    /// Current component context (for nested resource registration)
    current_component: Option<String>,

    /// Component stack for nested components
    component_stack: Vec<String>,
}

/// A resource registered during program execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredResource {
    /// URN
    pub urn: String,

    /// Resource type
    pub resource_type: String,

    /// Logical name
    pub name: String,

    /// Input properties
    pub inputs: PropertyValues,

    /// Resource options
    pub options: RegisteredResourceOptions,

    /// Parent URN (for component resources)
    pub parent: Option<String>,

    /// Whether this is a component resource (vs. a cloud resource)
    #[serde(default)]
    pub is_component: bool,
}

/// A component resource that groups other resources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisteredComponent {
    /// URN
    pub urn: String,

    /// Component type (e.g., "my:module:WebService")
    pub component_type: String,

    /// Logical name
    pub name: String,

    /// Input properties (args passed to the component)
    pub inputs: PropertyValues,

    /// Component options
    pub options: ComponentOptions,

    /// Parent URN (for nested components)
    pub parent: Option<String>,

    /// Child resource URNs
    #[serde(default)]
    pub children: Vec<String>,

    /// Component outputs (exposed properties)
    #[serde(default)]
    pub outputs: PropertyValues,

    /// Component state (for stateful components)
    #[serde(default)]
    pub state: HashMap<String, serde_json::Value>,
}

/// Options for a registered resource
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegisteredResourceOptions {
    /// Dependencies
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Provider
    pub provider: Option<String>,

    /// Protect from deletion
    #[serde(default)]
    pub protect: bool,

    /// Ignore changes to properties
    #[serde(default)]
    pub ignore_changes: Vec<String>,

    /// Custom timeouts
    pub timeouts: Option<CustomTimeouts>,

    /// Delete before replace
    #[serde(default)]
    pub delete_before_replace: bool,

    /// Aliases (for renaming/refactoring)
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Custom resource ID (for imports)
    pub import_id: Option<String>,

    /// Version constraint for the provider
    pub provider_version: Option<String>,
}

/// Options for a component resource
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComponentOptions {
    /// Dependencies (wait for these before creating children)
    #[serde(default)]
    pub depends_on: Vec<String>,

    /// Protect all children from deletion
    #[serde(default)]
    pub protect: bool,

    /// Provider configurations for children
    #[serde(default)]
    pub providers: HashMap<String, String>,

    /// Aliases for the component
    #[serde(default)]
    pub aliases: Vec<String>,

    /// Transform function for child resources
    pub transformations: Option<Vec<String>>,
}

/// Reference to another stack's outputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackReference {
    /// Name/URN of the referenced stack
    pub stack_name: String,

    /// Output key to reference
    pub output_key: Option<String>,

    /// Cached output value
    pub cached_value: Option<serde_json::Value>,
}

/// Custom timeouts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTimeouts {
    pub create: Option<u64>,
    pub update: Option<u64>,
    pub delete: Option<u64>,
}

impl ResourceRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    // --- Resource Registration ---

    /// Register a resource
    pub fn register_resource(&self, resource: RegisteredResource) {
        let mut inner = self.inner.write().unwrap();

        // If there's a current component context, set it as parent
        let mut resource = resource;
        if resource.parent.is_none() {
            if let Some(parent_urn) = inner.current_component.clone() {
                resource.parent = Some(parent_urn.clone());

                // Add to parent's children list
                if let Some(component) = inner.components.get_mut(&parent_urn) {
                    component.children.push(resource.urn.clone());
                }
            }
        }

        inner.resources.insert(resource.urn.clone(), resource);
    }

    /// Get a registered resource
    pub fn get_resource(&self, urn: &str) -> Option<RegisteredResource> {
        let inner = self.inner.read().unwrap();
        inner.resources.get(urn).cloned()
    }

    /// Get all registered resources
    pub fn resources(&self) -> Vec<RegisteredResource> {
        let inner = self.inner.read().unwrap();
        inner.resources.values().cloned().collect()
    }

    /// Get child resources of a component
    pub fn get_children(&self, parent_urn: &str) -> Vec<RegisteredResource> {
        let inner = self.inner.read().unwrap();
        inner
            .resources
            .values()
            .filter(|r| r.parent.as_deref() == Some(parent_urn))
            .cloned()
            .collect()
    }

    // --- Component Registration ---

    /// Register a component resource
    pub fn register_component(&self, component: RegisteredComponent) -> String {
        let mut inner = self.inner.write().unwrap();
        let urn = component.urn.clone();

        // If there's a current component context, set it as parent
        let mut component = component;
        if component.parent.is_none() {
            if let Some(parent_urn) = inner.current_component.clone() {
                component.parent = Some(parent_urn.clone());

                // Add to parent's children list
                if let Some(parent) = inner.components.get_mut(&parent_urn) {
                    parent.children.push(urn.clone());
                }
            }
        }

        inner.components.insert(urn.clone(), component);
        urn
    }

    /// Get a registered component
    pub fn get_component(&self, urn: &str) -> Option<RegisteredComponent> {
        let inner = self.inner.read().unwrap();
        inner.components.get(urn).cloned()
    }

    /// Get all registered components
    pub fn components(&self) -> Vec<RegisteredComponent> {
        let inner = self.inner.read().unwrap();
        inner.components.values().cloned().collect()
    }

    /// Enter a component context (resources created will be children of this component)
    pub fn enter_component(&self, urn: &str) {
        let mut inner = self.inner.write().unwrap();
        if let Some(current) = inner.current_component.take() {
            inner.component_stack.push(current);
        }
        inner.current_component = Some(urn.to_string());
    }

    /// Exit the current component context
    pub fn exit_component(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.current_component = inner.component_stack.pop();
    }

    /// Get the current component context
    pub fn current_component(&self) -> Option<String> {
        let inner = self.inner.read().unwrap();
        inner.current_component.clone()
    }

    /// Set component outputs
    pub fn set_component_outputs(&self, urn: &str, outputs: PropertyValues) {
        let mut inner = self.inner.write().unwrap();
        if let Some(component) = inner.components.get_mut(urn) {
            component.outputs = outputs;
        }
    }

    /// Get component outputs
    pub fn get_component_outputs(&self, urn: &str) -> Option<PropertyValues> {
        let inner = self.inner.read().unwrap();
        inner.components.get(urn).map(|c| c.outputs.clone())
    }

    // --- Stack References ---

    /// Register a stack reference
    pub fn register_stack_reference(&self, name: &str, reference: StackReference) {
        let mut inner = self.inner.write().unwrap();
        inner.stack_references.insert(name.to_string(), reference);
    }

    /// Get a stack reference
    pub fn get_stack_reference(&self, name: &str) -> Option<StackReference> {
        let inner = self.inner.read().unwrap();
        inner.stack_references.get(name).cloned()
    }

    /// Get all stack references
    pub fn stack_references(&self) -> HashMap<String, StackReference> {
        let inner = self.inner.read().unwrap();
        inner.stack_references.clone()
    }

    // --- Outputs ---

    /// Set outputs for a resource
    pub fn set_outputs(&self, urn: &str, outputs: PropertyValues) {
        let mut inner = self.inner.write().unwrap();
        inner.outputs.insert(urn.to_string(), outputs);
    }

    /// Get outputs for a resource
    pub fn get_outputs(&self, urn: &str) -> Option<PropertyValues> {
        let inner = self.inner.read().unwrap();
        inner.outputs.get(urn).cloned()
    }

    /// Export a stack output
    pub fn export(&self, name: &str, value: serde_json::Value) {
        let mut inner = self.inner.write().unwrap();
        inner.stack_outputs.insert(name.to_string(), value);
    }

    /// Get all stack outputs
    pub fn stack_outputs(&self) -> HashMap<String, serde_json::Value> {
        let inner = self.inner.read().unwrap();
        inner.stack_outputs.clone()
    }

    // --- Configuration ---

    /// Set a configuration value
    pub fn set_config(&self, key: &str, value: serde_json::Value) {
        let mut inner = self.inner.write().unwrap();
        inner.config.insert(key.to_string(), value);
    }

    /// Get a configuration value
    pub fn get_config(&self, key: &str) -> Option<serde_json::Value> {
        let inner = self.inner.read().unwrap();
        inner.config.get(key).cloned()
    }

    // --- Conversion ---

    /// Convert registered resources to core resources
    pub fn to_core_resources(&self, stack: &str) -> Vec<Resource> {
        let inner = self.inner.read().unwrap();

        inner
            .resources
            .values()
            .filter(|r| !r.is_component) // Only include non-component resources
            .map(|reg| {
                let resource_type = ResourceType::parse(&reg.resource_type)
                    .unwrap_or_else(|_| ResourceType::new("unknown", "unknown", "Unknown"));

                let mut resource = Resource::new(stack, resource_type, &reg.name, reg.inputs.clone());

                // Parse and set the URN
                if let Ok(urn) = Urn::parse(&reg.urn) {
                    resource.urn = urn;
                }

                // Set outputs if available
                if let Some(outputs) = inner.outputs.get(&reg.urn) {
                    resource.outputs = outputs.clone();
                }

                // Set options
                resource.options = ResourceOptions {
                    depends_on: reg
                        .options
                        .depends_on
                        .iter()
                        .filter_map(|s| Urn::parse(s).ok())
                        .collect(),
                    protect: reg.options.protect,
                    ignore_changes: reg.options.ignore_changes.clone(),
                    parent: reg.parent.as_ref().and_then(|s| Urn::parse(s).ok()),
                    delete_before_replace: reg.options.delete_before_replace,
                    aliases: reg
                        .options
                        .aliases
                        .iter()
                        .filter_map(|s| Urn::parse(s).ok())
                        .collect(),
                    ..Default::default()
                };

                resource
            })
            .collect()
    }

    // --- Statistics ---

    /// Get the number of registered resources (excluding components)
    pub fn len(&self) -> usize {
        let inner = self.inner.read().unwrap();
        inner.resources.values().filter(|r| !r.is_component).count()
    }

    /// Get the number of registered components
    pub fn component_count(&self) -> usize {
        let inner = self.inner.read().unwrap();
        inner.components.len()
    }

    /// Get the total count (resources + components)
    pub fn total_count(&self) -> usize {
        let inner = self.inner.read().unwrap();
        inner.resources.len() + inner.components.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0 && self.component_count() == 0
    }

    /// Clear all registrations
    pub fn clear(&self) {
        let mut inner = self.inner.write().unwrap();
        inner.resources.clear();
        inner.components.clear();
        inner.stack_references.clear();
        inner.outputs.clear();
        inner.stack_outputs.clear();
        inner.current_component = None;
        inner.component_stack.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use devmer_core::types::PropertyValue;

    #[test]
    fn test_resource_registration() {
        let registry = ResourceRegistry::new();

        let resource = RegisteredResource {
            urn: "urn:devmer:test::project::aws:s3:Bucket::my-bucket".to_string(),
            resource_type: "aws:s3:Bucket".to_string(),
            name: "my-bucket".to_string(),
            inputs: PropertyValues::new(),
            options: RegisteredResourceOptions::default(),
            parent: None,
            is_component: false,
        };

        registry.register_resource(resource);
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_component_registration() {
        let registry = ResourceRegistry::new();

        let component = RegisteredComponent {
            urn: "urn:devmer:test::project::my:module:WebService::api".to_string(),
            component_type: "my:module:WebService".to_string(),
            name: "api".to_string(),
            inputs: PropertyValues::new(),
            options: ComponentOptions::default(),
            parent: None,
            children: vec![],
            outputs: PropertyValues::new(),
            state: HashMap::new(),
        };

        let urn = registry.register_component(component);
        assert_eq!(registry.component_count(), 1);
        assert!(registry.get_component(&urn).is_some());
    }

    #[test]
    fn test_component_context() {
        let registry = ResourceRegistry::new();

        // Register a component
        let component = RegisteredComponent {
            urn: "urn:devmer:test::project::my:module:WebService::api".to_string(),
            component_type: "my:module:WebService".to_string(),
            name: "api".to_string(),
            inputs: PropertyValues::new(),
            options: ComponentOptions::default(),
            parent: None,
            children: vec![],
            outputs: PropertyValues::new(),
            state: HashMap::new(),
        };
        let component_urn = registry.register_component(component);

        // Enter component context
        registry.enter_component(&component_urn);

        // Register a child resource
        let resource = RegisteredResource {
            urn: "urn:devmer:test::project::aws:s3:Bucket::child-bucket".to_string(),
            resource_type: "aws:s3:Bucket".to_string(),
            name: "child-bucket".to_string(),
            inputs: PropertyValues::new(),
            options: RegisteredResourceOptions::default(),
            parent: None, // Will be set automatically
            is_component: false,
        };
        registry.register_resource(resource);

        // Exit component context
        registry.exit_component();

        // Check that the resource has the component as parent
        let registered = registry.get_resource("urn:devmer:test::project::aws:s3:Bucket::child-bucket").unwrap();
        assert_eq!(registered.parent, Some(component_urn.clone()));

        // Check that the component has the resource as child
        let component = registry.get_component(&component_urn).unwrap();
        assert!(component.children.contains(&"urn:devmer:test::project::aws:s3:Bucket::child-bucket".to_string()));
    }

    #[test]
    fn test_nested_components() {
        let registry = ResourceRegistry::new();

        // Register parent component
        let parent = RegisteredComponent {
            urn: "urn:devmer:test::project::my:module:App::main".to_string(),
            component_type: "my:module:App".to_string(),
            name: "main".to_string(),
            inputs: PropertyValues::new(),
            options: ComponentOptions::default(),
            parent: None,
            children: vec![],
            outputs: PropertyValues::new(),
            state: HashMap::new(),
        };
        let parent_urn = registry.register_component(parent);
        registry.enter_component(&parent_urn);

        // Register child component
        let child = RegisteredComponent {
            urn: "urn:devmer:test::project::my:module:Database::db".to_string(),
            component_type: "my:module:Database".to_string(),
            name: "db".to_string(),
            inputs: PropertyValues::new(),
            options: ComponentOptions::default(),
            parent: None, // Will be set automatically
            children: vec![],
            outputs: PropertyValues::new(),
            state: HashMap::new(),
        };
        let child_urn = registry.register_component(child);
        registry.enter_component(&child_urn);

        // Register resource in child component
        let resource = RegisteredResource {
            urn: "urn:devmer:test::project::aws:rds:Instance::instance".to_string(),
            resource_type: "aws:rds:Instance".to_string(),
            name: "instance".to_string(),
            inputs: PropertyValues::new(),
            options: RegisteredResourceOptions::default(),
            parent: None,
            is_component: false,
        };
        registry.register_resource(resource);

        // Exit both contexts
        registry.exit_component();
        registry.exit_component();

        // Verify hierarchy
        let child_component = registry.get_component(&child_urn).unwrap();
        assert_eq!(child_component.parent, Some(parent_urn.clone()));
        assert!(child_component.children.contains(&"urn:devmer:test::project::aws:rds:Instance::instance".to_string()));

        let parent_component = registry.get_component(&parent_urn).unwrap();
        assert!(parent_component.children.contains(&child_urn));
    }

    #[test]
    fn test_component_outputs() {
        let registry = ResourceRegistry::new();

        let component = RegisteredComponent {
            urn: "urn:devmer:test::project::my:module:WebService::api".to_string(),
            component_type: "my:module:WebService".to_string(),
            name: "api".to_string(),
            inputs: PropertyValues::new(),
            options: ComponentOptions::default(),
            parent: None,
            children: vec![],
            outputs: PropertyValues::new(),
            state: HashMap::new(),
        };
        let urn = registry.register_component(component);

        // Set component outputs
        let mut outputs = PropertyValues::new();
        outputs.insert("endpoint".to_string(), PropertyValue::String("https://api.example.com".to_string()));
        outputs.insert("api_key".to_string(), PropertyValue::String("secret-key".to_string()));

        registry.set_component_outputs(&urn, outputs);

        // Get component outputs
        let retrieved_outputs = registry.get_component_outputs(&urn).unwrap();
        assert!(retrieved_outputs.contains_key("endpoint"));
        assert!(retrieved_outputs.contains_key("api_key"));
    }

    #[test]
    fn test_stack_reference() {
        let registry = ResourceRegistry::new();

        let reference = StackReference {
            stack_name: "organization/project/production".to_string(),
            output_key: Some("vpc_id".to_string()),
            cached_value: Some(serde_json::json!("vpc-12345")),
        };

        registry.register_stack_reference("prod-vpc", reference);

        let retrieved = registry.get_stack_reference("prod-vpc").unwrap();
        assert_eq!(retrieved.stack_name, "organization/project/production");
        assert_eq!(retrieved.output_key, Some("vpc_id".to_string()));
    }
}
