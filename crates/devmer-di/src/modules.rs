//! Service implementations

use crate::interfaces::{
    ChangeType, ConfigService, DeployResult, DestroyResult, ExecutionService, PreviewResult,
    PropertyDiff, ProviderRegistryService, RefreshResult, ResourceChange, RuntimeService,
    StateService,
};
use devmer_config::DevmerConfig;
use devmer_core::engine::{PlanBuilder, ResourceOperation};
use devmer_core::provider::Provider;
use devmer_core::registry::ProviderRegistry;
use devmer_core::resource::Resource;
use devmer_core::state::StackState;
use devmer_core::types::{PropertyValue, PropertyValues};
use devmer_providers::{AwsProvider, MockProvider};
use devmer_runtime::context::{ConfigProvider, ResourceProvider, RuntimeContext};
use devmer_runtime::registry::{
    ComponentOptions, RegisteredComponent, RegisteredResource, RegisteredResourceOptions,
    ResourceRegistry, StackReference,
};
use devmer_runtime::runtime::{LanguageRuntime, RuntimeConfig, RuntimeKind};
use devmer_runtime::RhaiRuntime;
use devmer_state::backend::StateBackend;
use devmer_state::locking::{LockId, LockInfo, LockStatus};
use devmer_state::local::LocalBackend;
use shaku::Component;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock as StdRwLock};
use tokio::sync::RwLock;
use tracing::{debug, info};

/// Configuration service implementation
#[derive(Component)]
#[shaku(interface = ConfigService)]
pub struct ConfigServiceImpl {
    config: DevmerConfig,
}

impl ConfigService for ConfigServiceImpl {
    fn get(&self, key: &str) -> Option<String> {
        self.config.get(key)
    }

    fn config(&self) -> &DevmerConfig {
        &self.config
    }

    fn stack_names(&self) -> Vec<String> {
        self.config
            .stack_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    }
}

/// State service implementation using local backend
#[derive(Component)]
#[shaku(interface = StateService)]
pub struct StateServiceImpl {
    #[shaku(inject)]
    config_service: Arc<dyn ConfigService>,
    #[shaku(default)]
    backend: Option<Arc<RwLock<LocalBackend>>>,
}

impl StateServiceImpl {
    fn get_backend(&self) -> Arc<RwLock<LocalBackend>> {
        if let Some(ref backend) = self.backend {
            backend.clone()
        } else {
            // Create default local backend with state stored in .devmer directory
            let state_dir = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(".devmer");

            Arc::new(RwLock::new(LocalBackend::new(state_dir)))
        }
    }

    fn project_name(&self) -> &str {
        &self.config_service.config().name
    }
}

#[async_trait::async_trait]
impl StateService for StateServiceImpl {
    async fn get_state(&self, stack: &str) -> anyhow::Result<Option<StackState>> {
        let backend = self.get_backend();
        let backend_guard = backend.read().await;
        let project = self.project_name();
        backend_guard.get_state(project, stack).await.map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn save_state(&self, stack: &str, state: &StackState) -> anyhow::Result<()> {
        let backend = self.get_backend();
        let backend_guard = backend.read().await;
        let project = self.project_name();
        backend_guard.save_state(project, stack, state).await.map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn delete_state(&self, stack: &str) -> anyhow::Result<()> {
        let backend = self.get_backend();
        let backend_guard = backend.read().await;
        let project = self.project_name();
        backend_guard.delete_state(project, stack).await.map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn list_stacks(&self) -> anyhow::Result<Vec<String>> {
        let backend = self.get_backend();
        let backend_guard = backend.read().await;
        let project = self.project_name();
        backend_guard.list_stacks(project).await.map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn lock(&self, stack: &str, info: LockInfo) -> anyhow::Result<LockId> {
        let backend = self.get_backend();
        let backend_guard = backend.read().await;
        let project = self.project_name();
        backend_guard.lock(project, stack, info).await.map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn unlock(&self, stack: &str, lock_id: &LockId) -> anyhow::Result<()> {
        let backend = self.get_backend();
        let backend_guard = backend.read().await;
        let project = self.project_name();
        backend_guard.unlock(project, stack, lock_id).await.map_err(|e| anyhow::anyhow!("{}", e))
    }

    async fn get_lock_status(&self, stack: &str) -> anyhow::Result<LockStatus> {
        let backend = self.get_backend();
        let backend_guard = backend.read().await;
        let project = self.project_name();
        backend_guard.get_lock_status(project, stack).await.map_err(|e| anyhow::anyhow!("{}", e))
    }
}

// --- Runtime Context Providers ---
// These implement the traits from devmer-runtime::context to expose DI services to scripts

/// Config provider that wraps ConfigService for script access
pub struct DiConfigProvider {
    config: DevmerConfig,
    stack: String,
}

impl DiConfigProvider {
    pub fn new(config: DevmerConfig, stack: &str) -> Self {
        Self {
            config,
            stack: stack.to_string(),
        }
    }
}

impl ConfigProvider for DiConfigProvider {
    fn get(&self, key: &str) -> Option<String> {
        self.config.get(key)
    }

    fn get_namespace(&self, namespace: &str) -> HashMap<String, String> {
        // Get all config values that start with the namespace
        let prefix = format!("{}:", namespace);
        let mut result = HashMap::new();

        // Check stack-specific config
        if let Some(stack_config) = self.config.stack.get(&self.stack) {
            for (k, v) in stack_config.config.iter() {
                if k.starts_with(&prefix) {
                    // Convert toml::Value to string
                    let value_str = match v {
                        toml::Value::String(s) => s.clone(),
                        toml::Value::Integer(i) => i.to_string(),
                        toml::Value::Float(f) => f.to_string(),
                        toml::Value::Boolean(b) => b.to_string(),
                        _ => v.to_string(),
                    };
                    result.insert(k[prefix.len()..].to_string(), value_str);
                }
            }
        }

        result
    }

    fn project_name(&self) -> &str {
        &self.config.name
    }

    fn stack_name(&self) -> &str {
        &self.stack
    }
}

/// Resource provider that collects resources and outputs
pub struct DiResourceProvider {
    registry: ResourceRegistry,
    project: String,
    stack: String,
}

impl DiResourceProvider {
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

impl ResourceProvider for DiResourceProvider {
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
        PropertyValue::Object(obj) => serde_json::Value::Object(
            obj.iter()
                .map(|(k, v)| (k.clone(), property_value_to_json(v)))
                .collect(),
        ),
        PropertyValue::Secret(_) => serde_json::Value::String("[SECRET]".to_string()),
        PropertyValue::OutputRef(output_ref) => {
            serde_json::json!({
                "$ref": output_ref.urn,
                "property": output_ref.property
            })
        }
    }
}

/// Provider registry service implementation
#[derive(Component)]
#[shaku(interface = ProviderRegistryService)]
pub struct ProviderRegistryServiceImpl {
    #[shaku(default)]
    registry: StdRwLock<ProviderRegistry>,
}

impl Default for ProviderRegistryServiceImpl {
    fn default() -> Self {
        let registry = ProviderRegistry::new();

        // Register built-in providers
        registry.register("aws", Arc::new(AwsProvider::new()));
        registry.register("mock", Arc::new(MockProvider::new()));

        // Register aliases
        registry.register_alias("amazon", "aws");

        Self {
            registry: StdRwLock::new(registry),
        }
    }
}

impl ProviderRegistryService for ProviderRegistryServiceImpl {
    fn get_provider(&self, name: &str) -> Option<Arc<dyn Provider>> {
        let registry = self.registry.read().unwrap();
        registry.get(name)
    }

    fn register_provider(&self, name: &str, provider: Arc<dyn Provider>) {
        let registry = self.registry.read().unwrap();
        registry.register(name, provider);
    }

    fn list_providers(&self) -> Vec<String> {
        let registry = self.registry.read().unwrap();
        registry.list()
    }
}

/// Runtime service implementation
#[derive(Component)]
#[shaku(interface = RuntimeService)]
pub struct RuntimeServiceImpl {
    #[shaku(inject)]
    config_service: Arc<dyn ConfigService>,
}

impl RuntimeServiceImpl {
    fn get_runtime_kind(&self) -> RuntimeKind {
        self.config_service
            .config()
            .runtime
            .name
            .as_deref()
            .and_then(RuntimeKind::from_str)
            .unwrap_or(RuntimeKind::Rhai)
    }

    fn create_runtime(&self) -> Box<dyn LanguageRuntime> {
        let kind = self.get_runtime_kind();
        match kind {
            RuntimeKind::Rhai => Box::new(RhaiRuntime::new()),
            // External runtimes will need LanguageHost
            _ => Box::new(RhaiRuntime::new()), // Fallback to Rhai for now
        }
    }

    /// Create a runtime context with proper DI providers
    pub fn create_context(&self, stack: &str, preview: bool) -> Arc<RuntimeContext> {
        let config = self.config_service.config().clone();
        let project = config.name.clone();

        let config_provider = Arc::new(DiConfigProvider::new(config, stack));
        let resource_provider = Arc::new(DiResourceProvider::new(&project, stack));

        Arc::new(RuntimeContext::new(
            config_provider,
            resource_provider,
            stack,
            preview,
        ))
    }
}

#[async_trait::async_trait]
impl RuntimeService for RuntimeServiceImpl {
    async fn run(&self, config: &RuntimeConfig) -> anyhow::Result<devmer_runtime::RunResult> {
        let runtime = self.create_runtime();
        runtime.run(config).await.map_err(|e| anyhow::anyhow!("{}", e))
    }

    fn runtime_kind(&self) -> RuntimeKind {
        self.get_runtime_kind()
    }

    async fn is_available(&self) -> bool {
        let runtime = self.create_runtime();
        runtime.is_available().await
    }

    async fn install_dependencies(&self, config: &RuntimeConfig) -> anyhow::Result<()> {
        let runtime = self.create_runtime();
        runtime.install_dependencies(config).await.map_err(|e| anyhow::anyhow!("{}", e))
    }
}

/// Execution service implementation
#[derive(Component)]
#[shaku(interface = ExecutionService)]
pub struct ExecutionServiceImpl {
    #[shaku(inject)]
    config_service: Arc<dyn ConfigService>,
    #[shaku(inject)]
    state_service: Arc<dyn StateService>,
    #[shaku(inject)]
    runtime_service: Arc<dyn RuntimeService>,
}

impl ExecutionServiceImpl {
    /// Run the infrastructure program and collect desired resources
    async fn run_program(&self, stack: &str, preview: bool) -> anyhow::Result<Vec<Resource>> {
        let config = self.config_service.config();

        // Get the entry point from config
        let working_dir = std::env::current_dir()?;
        let entry_point = config.runtime.main.as_ref()
            .map(|m| working_dir.join(m))
            .unwrap_or_else(|| working_dir.join(self.runtime_service.runtime_kind().default_entry_point()));

        // Check if entry point exists
        if !entry_point.exists() {
            debug!("Entry point {} not found, returning empty resources", entry_point.display());
            return Ok(vec![]);
        }

        // Build runtime config
        let runtime_config = RuntimeConfig::new(
            self.runtime_service.runtime_kind(),
            working_dir.clone(),
            stack,
        )
        .with_entry_point(entry_point)
        .with_preview(preview);

        // Run the program
        info!("Running infrastructure program for stack {}", stack);
        let run_result = self.runtime_service.run(&runtime_config).await?;

        if !run_result.success {
            let errors = run_result.errors.join(", ");
            return Err(anyhow::anyhow!("Program execution failed: {}", errors));
        }

        // Convert collected resources to core resources
        let resources = run_result.resources.to_core_resources(stack);
        info!("Collected {} resources from program", resources.len());

        Ok(resources)
    }
}

#[async_trait::async_trait]
impl ExecutionService for ExecutionServiceImpl {
    async fn preview(&self, stack: &str) -> anyhow::Result<PreviewResult> {
        // Get current state from backend
        let current_state = self.state_service.get_state(stack).await?;

        // Run the program to get desired resources
        let desired_resources = self.run_program(stack, true).await?;

        // Build the plan
        let mut plan_builder =
            PlanBuilder::new(stack).with_desired_resources(desired_resources);

        if let Some(state) = current_state {
            plan_builder = plan_builder.with_current_state(state);
        }

        let deployment_plan = plan_builder.build()?;

        // Convert to preview result
        let creates: Vec<ResourceChange> = deployment_plan
            .steps
            .iter()
            .filter(|s| s.operation == ResourceOperation::Create)
            .map(|s| ResourceChange {
                urn: s.urn.to_string(),
                resource_type: s.resource_type.clone(),
                name: s.name.clone(),
                change_type: ChangeType::Create,
                diffs: s
                    .diffs
                    .iter()
                    .map(|d| PropertyDiff {
                        path: d.path.clone(),
                        old_value: d.old_value.clone(),
                        new_value: d.new_value.clone(),
                    })
                    .collect(),
            })
            .collect();

        let updates: Vec<ResourceChange> = deployment_plan
            .steps
            .iter()
            .filter(|s| {
                matches!(
                    s.operation,
                    ResourceOperation::Update | ResourceOperation::Replace
                )
            })
            .map(|s| ResourceChange {
                urn: s.urn.to_string(),
                resource_type: s.resource_type.clone(),
                name: s.name.clone(),
                change_type: if s.operation == ResourceOperation::Replace {
                    ChangeType::Replace
                } else {
                    ChangeType::Update
                },
                diffs: s
                    .diffs
                    .iter()
                    .map(|d| PropertyDiff {
                        path: d.path.clone(),
                        old_value: d.old_value.clone(),
                        new_value: d.new_value.clone(),
                    })
                    .collect(),
            })
            .collect();

        let deletes: Vec<ResourceChange> = deployment_plan
            .steps
            .iter()
            .filter(|s| s.operation == ResourceOperation::Delete)
            .map(|s| ResourceChange {
                urn: s.urn.to_string(),
                resource_type: s.resource_type.clone(),
                name: s.name.clone(),
                change_type: ChangeType::Delete,
                diffs: vec![],
            })
            .collect();

        let same = deployment_plan
            .steps
            .iter()
            .filter(|s| s.operation == ResourceOperation::Same)
            .count();

        Ok(PreviewResult {
            stack: stack.to_string(),
            creates,
            updates,
            deletes,
            same,
        })
    }

    async fn deploy(&self, stack: &str, _auto_approve: bool) -> anyhow::Result<DeployResult> {
        let start = std::time::Instant::now();

        // Get current state
        let current_state = self.state_service.get_state(stack).await?;

        // Run the program to get desired resources
        let desired_resources = self.run_program(stack, false).await?;

        // Build deployment plan
        let mut plan_builder =
            PlanBuilder::new(stack).with_desired_resources(desired_resources.clone());

        if let Some(ref state) = current_state {
            plan_builder = plan_builder.with_current_state(state.clone());
        }

        let deployment_plan = plan_builder.build()?;

        // Count operations
        let creates = deployment_plan
            .steps
            .iter()
            .filter(|s| s.operation == ResourceOperation::Create)
            .count();
        let updates = deployment_plan
            .steps
            .iter()
            .filter(|s| matches!(s.operation, ResourceOperation::Update | ResourceOperation::Replace))
            .count();
        let deletes = deployment_plan
            .steps
            .iter()
            .filter(|s| s.operation == ResourceOperation::Delete)
            .count();

        // Update state with new resources
        let mut new_state = current_state.unwrap_or_else(|| StackState::new(stack));
        for resource in desired_resources {
            new_state.add_or_update_resource(resource);
        }
        self.state_service.save_state(stack, &new_state).await?;

        Ok(DeployResult {
            stack: stack.to_string(),
            success: true,
            resources_created: creates,
            resources_updated: updates,
            resources_deleted: deletes,
            errors: vec![],
            duration_secs: start.elapsed().as_secs_f64(),
        })
    }

    async fn destroy(&self, stack: &str, _auto_approve: bool) -> anyhow::Result<DestroyResult> {
        let start = std::time::Instant::now();

        // Get current state
        let current_state = self.state_service.get_state(stack).await?;

        let resources_to_destroy = current_state
            .as_ref()
            .map(|s| s.resource_count())
            .unwrap_or(0);

        // In real implementation:
        // 1. Load current state from backend
        // 2. Build plan to delete all resources
        // 3. Execute the plan
        // 4. Delete the state

        // Delete state file
        self.state_service.delete_state(stack).await?;

        Ok(DestroyResult {
            stack: stack.to_string(),
            success: true,
            resources_destroyed: resources_to_destroy,
            errors: vec![],
            duration_secs: start.elapsed().as_secs_f64(),
        })
    }

    async fn refresh(&self, stack: &str) -> anyhow::Result<RefreshResult> {
        // Get current state
        let current_state = self.state_service.get_state(stack).await?;

        let resources_refreshed = current_state
            .as_ref()
            .map(|s| s.resource_count())
            .unwrap_or(0);

        // In real implementation:
        // 1. Load current state from backend
        // 2. Read each resource from the cloud
        // 3. Update state with current values
        // 4. Save updated state

        Ok(RefreshResult {
            stack: stack.to_string(),
            success: true,
            resources_refreshed,
            drift_detected: 0,
        })
    }
}
