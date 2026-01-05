//! Embedded Rhai runtime
//!
//! Provides an embedded Rhai scripting runtime that integrates with the
//! RuntimeContext to access configuration, secrets, and resource registration.

use crate::context::{ResourceProvider, RuntimeContext, SimpleConfigProvider, SimpleResourceProvider};
use crate::error::{Result, RuntimeError};
use crate::registry::ResourceRegistry;
use crate::runtime::{LanguageRuntime, RunResult, RuntimeConfig, RuntimeKind};
use async_trait::async_trait;
use devmer_core::types::{PropertyValue, PropertyValues};
use rhai::{Dynamic, Engine, EvalAltResult, ImmutableString, Map, Scope};
use std::sync::Arc;
use std::time::Instant;
use tracing::{debug, error, info, warn};

/// Embedded Rhai scripting runtime
pub struct RhaiRuntime {
    engine: Engine,
}

impl RhaiRuntime {
    /// Create a new Rhai runtime
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // Register basic utility functions that don't need context
        Self::register_utility_functions(&mut engine);

        Self { engine }
    }

    /// Register utility functions that don't need context
    fn register_utility_functions(engine: &mut Engine) {
        // Log functions
        engine.register_fn("log", |msg: ImmutableString| {
            info!("[rhai] {}", msg);
        });

        engine.register_fn("warn", |msg: ImmutableString| {
            warn!("[rhai] {}", msg);
        });

        engine.register_fn("error", |msg: ImmutableString| {
            error!("[rhai] {}", msg);
        });

        engine.register_fn("debug", |msg: ImmutableString| {
            debug!("[rhai] {}", msg);
        });

        // Type conversion helpers
        engine.register_fn("to_string", |value: Dynamic| -> ImmutableString {
            value.to_string().into()
        });

        engine.register_fn("to_int", |value: Dynamic| -> i64 {
            value.as_int().unwrap_or(0)
        });

        engine.register_fn("to_float", |value: Dynamic| -> f64 {
            value.as_float().unwrap_or(0.0)
        });

        engine.register_fn("to_bool", |value: Dynamic| -> bool {
            value.as_bool().unwrap_or(false)
        });
    }

    /// Create an engine configured with context-aware functions
    fn create_context_engine(ctx: Arc<RuntimeContext>) -> Engine {
        let mut engine = Engine::new();

        // Register basic utilities
        Self::register_utility_functions(&mut engine);

        // Register context-aware functions
        Self::register_context_functions(&mut engine, ctx);

        engine
    }

    /// Register functions that use the RuntimeContext
    fn register_context_functions(engine: &mut Engine, ctx: Arc<RuntimeContext>) {
        // --- Configuration Access ---

        // config(key) - get a config value
        let ctx_config = ctx.clone();
        engine.register_fn("config", move |key: ImmutableString| -> Dynamic {
            match ctx_config.config_get(&key) {
                Some(value) => Dynamic::from(value),
                None => Dynamic::UNIT,
            }
        });

        // config(key, default) - get a config value with default
        let ctx_config_default = ctx.clone();
        engine.register_fn(
            "config",
            move |key: ImmutableString, default: Dynamic| -> Dynamic {
                match ctx_config_default.config_get(&key) {
                    Some(value) => Dynamic::from(value),
                    None => default,
                }
            },
        );

        // get_config(namespace) - get all config for a namespace as a map
        let ctx_namespace = ctx.clone();
        engine.register_fn("get_config", move |namespace: ImmutableString| -> Map {
            let values = ctx_namespace.config_namespace(&namespace);
            values
                .into_iter()
                .map(|(k, v)| (k.into(), Dynamic::from(v)))
                .collect()
        });

        // --- Stack Information ---

        // stack() - get the current stack name
        let ctx_stack = ctx.clone();
        engine.register_fn("stack", move || -> ImmutableString {
            ctx_stack.stack().into()
        });

        // project() - get the project name
        let ctx_project = ctx.clone();
        engine.register_fn("project", move || -> ImmutableString {
            ctx_project.project_name().into()
        });

        // is_preview() - check if running in preview mode
        let ctx_preview = ctx.clone();
        engine.register_fn("is_preview", move || -> bool { ctx_preview.is_preview() });

        // --- Resource Registration ---

        // resource(type, name, inputs) - register a resource
        let ctx_resource = ctx.clone();
        engine.register_fn(
            "resource",
            move |resource_type: ImmutableString, name: ImmutableString, inputs: Map| -> ImmutableString {
                let inputs = map_to_property_values(&inputs);
                let urn = ctx_resource.resource(&resource_type, &name, inputs);
                info!(
                    resource_type = %resource_type,
                    name = %name,
                    urn = %urn,
                    "Registered resource"
                );
                urn.into()
            },
        );

        // resource_with_opts(type, name, inputs, options) - register with options
        let ctx_resource_opts = ctx.clone();
        engine.register_fn(
            "resource_with_opts",
            move |resource_type: ImmutableString,
                  name: ImmutableString,
                  inputs: Map,
                  opts: Map|
                  -> ImmutableString {
                let inputs = map_to_property_values(&inputs);
                let options = map_to_resource_options(&opts);
                let urn = ctx_resource_opts.resource_with_options(&resource_type, &name, inputs, options);
                info!(
                    resource_type = %resource_type,
                    name = %name,
                    urn = %urn,
                    "Registered resource with options"
                );
                urn.into()
            },
        );

        // get_resource(urn) - get a registered resource
        let ctx_get_resource = ctx.clone();
        engine.register_fn("get_resource", move |urn: ImmutableString| -> Dynamic {
            match ctx_get_resource.get_resource(&urn) {
                Some(resource) => {
                    let mut map = Map::new();
                    map.insert("urn".into(), Dynamic::from(resource.urn));
                    map.insert("type".into(), Dynamic::from(resource.resource_type));
                    map.insert("name".into(), Dynamic::from(resource.name));
                    map.insert("inputs".into(), property_values_to_dynamic(&resource.inputs));
                    Dynamic::from_map(map)
                }
                None => Dynamic::UNIT,
            }
        });

        // --- Component Registration ---

        // component(type, name, inputs) - register a component
        let ctx_component = ctx.clone();
        engine.register_fn(
            "component",
            move |component_type: ImmutableString, name: ImmutableString, inputs: Map| -> ImmutableString {
                let inputs = map_to_property_values(&inputs);
                let urn = ctx_component.component(&component_type, &name, inputs);
                info!(
                    component_type = %component_type,
                    name = %name,
                    urn = %urn,
                    "Registered component"
                );
                urn.into()
            },
        );

        // component_with_opts(type, name, inputs, options) - register component with options
        let ctx_component_opts = ctx.clone();
        engine.register_fn(
            "component_with_opts",
            move |component_type: ImmutableString,
                  name: ImmutableString,
                  inputs: Map,
                  opts: Map|
                  -> ImmutableString {
                let inputs = map_to_property_values(&inputs);
                let options = map_to_component_options(&opts);
                let urn = ctx_component_opts.component_with_options(&component_type, &name, inputs, options);
                info!(
                    component_type = %component_type,
                    name = %name,
                    urn = %urn,
                    "Registered component with options"
                );
                urn.into()
            },
        );

        // get_component(urn) - get a registered component
        let ctx_get_component = ctx.clone();
        engine.register_fn("get_component", move |urn: ImmutableString| -> Dynamic {
            match ctx_get_component.get_component(&urn) {
                Some(component) => {
                    let mut map = Map::new();
                    map.insert("urn".into(), Dynamic::from(component.urn));
                    map.insert("type".into(), Dynamic::from(component.component_type));
                    map.insert("name".into(), Dynamic::from(component.name));
                    map.insert("inputs".into(), property_values_to_dynamic(&component.inputs));
                    map.insert("outputs".into(), property_values_to_dynamic(&component.outputs));
                    map.insert("children".into(), Dynamic::from(component.children));
                    if let Some(ref parent) = component.parent {
                        map.insert("parent".into(), Dynamic::from(parent.clone()));
                    }
                    Dynamic::from_map(map)
                }
                None => Dynamic::UNIT,
            }
        });

        // enter_component(urn) - enter a component context
        let ctx_enter = ctx.clone();
        engine.register_fn("enter_component", move |urn: ImmutableString| {
            ctx_enter.enter_component(&urn);
            debug!(urn = %urn, "Entered component context");
        });

        // exit_component() - exit the current component context
        let ctx_exit = ctx.clone();
        engine.register_fn("exit_component", move || {
            ctx_exit.exit_component();
            debug!("Exited component context");
        });

        // current_component() - get the current component context
        let ctx_current = ctx.clone();
        engine.register_fn("current_component", move || -> Dynamic {
            match ctx_current.current_component() {
                Some(urn) => Dynamic::from(urn),
                None => Dynamic::UNIT,
            }
        });

        // set_component_outputs(urn, outputs) - set component outputs
        let ctx_set_comp_outputs = ctx.clone();
        engine.register_fn(
            "set_component_outputs",
            move |urn: ImmutableString, outputs: Map| {
                let outputs = map_to_property_values(&outputs);
                ctx_set_comp_outputs.set_component_outputs(&urn, outputs);
                debug!(urn = %urn, "Set component outputs");
            },
        );

        // get_component_outputs(urn) - get component outputs
        let ctx_get_comp_outputs = ctx.clone();
        engine.register_fn("get_component_outputs", move |urn: ImmutableString| -> Dynamic {
            match ctx_get_comp_outputs.get_component_outputs(&urn) {
                Some(outputs) => property_values_to_dynamic(&outputs),
                None => Dynamic::UNIT,
            }
        });

        // get_children(parent_urn) - get child resources of a component
        let ctx_children = ctx.clone();
        engine.register_fn("get_children", move |parent_urn: ImmutableString| -> Vec<Dynamic> {
            ctx_children
                .get_children(&parent_urn)
                .into_iter()
                .map(|r| {
                    let mut map = Map::new();
                    map.insert("urn".into(), Dynamic::from(r.urn));
                    map.insert("type".into(), Dynamic::from(r.resource_type));
                    map.insert("name".into(), Dynamic::from(r.name));
                    Dynamic::from_map(map)
                })
                .collect()
        });

        // --- Stack References ---

        // stack_ref(name, stack_name) - reference another stack's outputs
        let ctx_stack_ref = ctx.clone();
        engine.register_fn(
            "stack_ref",
            move |name: ImmutableString, stack_name: ImmutableString| {
                ctx_stack_ref.stack_reference(&name, &stack_name, None);
                debug!(name = %name, stack = %stack_name, "Registered stack reference");
            },
        );

        // stack_ref_output(name, stack_name, output_key) - reference specific output from another stack
        let ctx_stack_ref_output = ctx.clone();
        engine.register_fn(
            "stack_ref_output",
            move |name: ImmutableString, stack_name: ImmutableString, output_key: ImmutableString| {
                ctx_stack_ref_output.stack_reference(&name, &stack_name, Some(&output_key));
                debug!(name = %name, stack = %stack_name, output = %output_key, "Registered stack reference with output");
            },
        );

        // get_stack_ref(name) - get a stack reference
        let ctx_get_stack_ref = ctx.clone();
        engine.register_fn("get_stack_ref", move |name: ImmutableString| -> Dynamic {
            match ctx_get_stack_ref.get_stack_reference(&name) {
                Some(reference) => {
                    let mut map = Map::new();
                    map.insert("stack_name".into(), Dynamic::from(reference.stack_name));
                    if let Some(ref key) = reference.output_key {
                        map.insert("output_key".into(), Dynamic::from(key.clone()));
                    }
                    if let Some(ref value) = reference.cached_value {
                        map.insert("value".into(), json_to_dynamic(value));
                    }
                    Dynamic::from_map(map)
                }
                None => Dynamic::UNIT,
            }
        });

        // --- Stack Outputs ---

        // output(name, value) - export a stack output
        let ctx_output = ctx.clone();
        engine.register_fn("output", move |name: ImmutableString, value: Dynamic| {
            let prop_value = dynamic_to_property_value(&value);
            ctx_output.output(&name, prop_value);
            debug!(name = %name, "Exported stack output");
        });

        // set_output - alias for output
        let ctx_set_output = ctx.clone();
        engine.register_fn("set_output", move |name: ImmutableString, value: Dynamic| {
            let prop_value = dynamic_to_property_value(&value);
            ctx_set_output.output(&name, prop_value);
            debug!(name = %name, "Exported stack output");
        });

        // --- Variables ---

        // var(name) - get a context variable
        let ctx_var = ctx.clone();
        engine.register_fn("var", move |name: ImmutableString| -> Dynamic {
            match ctx_var.var(&name) {
                Some(value) => property_value_to_dynamic(value),
                None => Dynamic::UNIT,
            }
        });
    }

    /// Execute a Rhai script with context
    pub fn execute_with_context(
        &self,
        script: &str,
        ctx: Arc<RuntimeContext>,
    ) -> std::result::Result<(), Box<EvalAltResult>> {
        // Create engine with context-aware functions
        let engine = Self::create_context_engine(ctx.clone());

        // Create scope with constants
        let mut scope = Scope::new();
        scope.push_constant("STACK", ctx.stack().to_string());
        scope.push_constant("PROJECT", ctx.project_name().to_string());
        scope.push_constant("PREVIEW", ctx.is_preview());

        // Compile and run
        let ast = engine.compile(script)?;
        engine.run_ast_with_scope(&mut scope, &ast)?;

        Ok(())
    }

    /// Execute a Rhai script without context (uses simple providers)
    fn execute_simple(
        &self,
        script: &str,
        config: &RuntimeConfig,
    ) -> std::result::Result<ResourceRegistry, Box<EvalAltResult>> {
        // Create simple providers
        let config_provider = Arc::new(
            SimpleConfigProvider::new(&config.project_config.name, &config.stack)
        );
        let resource_provider = Arc::new(
            SimpleResourceProvider::new(&config.project_config.name, &config.stack)
        );

        // Create context
        let ctx = Arc::new(RuntimeContext::new(
            config_provider,
            resource_provider.clone(),
            &config.stack,
            config.preview,
        ));

        // Execute
        self.execute_with_context(script, ctx)?;

        // Return a clone of the registry
        Ok(resource_provider.registry().clone())
    }
}

impl Default for RhaiRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LanguageRuntime for RhaiRuntime {
    fn kind(&self) -> RuntimeKind {
        RuntimeKind::Rhai
    }

    async fn is_available(&self) -> bool {
        true // Always available (embedded)
    }

    async fn version(&self) -> Result<String> {
        Ok(format!("rhai {}", env!("CARGO_PKG_VERSION")))
    }

    async fn run(&self, config: &RuntimeConfig) -> Result<RunResult> {
        let start = Instant::now();

        // Read the script file
        let script = tokio::fs::read_to_string(&config.entry_point)
            .await
            .map_err(|e| RuntimeError::InvalidProgram(e.to_string()))?;

        info!(
            entry = %config.entry_point.display(),
            "Running Rhai script"
        );

        // Execute synchronously (Rhai is not async)
        let config_clone = config.clone();
        let result = tokio::task::spawn_blocking(move || {
            let runtime = RhaiRuntime::new();
            runtime.execute_simple(&script, &config_clone)
        })
        .await
        .map_err(|e| RuntimeError::execution_failed(e.to_string()))?;

        let duration = start.elapsed();

        match result {
            Ok(registry) => Ok(RunResult {
                success: true,
                exit_code: Some(0),
                resources: registry,
                stdout: String::new(),
                stderr: String::new(),
                duration,
                errors: vec![],
            }),
            Err(e) => Ok(RunResult {
                success: false,
                exit_code: Some(1),
                resources: ResourceRegistry::new(),
                stdout: String::new(),
                stderr: e.to_string(),
                duration,
                errors: vec![e.to_string()],
            }),
        }
    }

    async fn install_dependencies(&self, _config: &RuntimeConfig) -> Result<()> {
        // Rhai has no external dependencies
        Ok(())
    }
}

/// Convert a Rhai Map to PropertyValues
fn map_to_property_values(map: &Map) -> PropertyValues {
    map.iter()
        .map(|(k, v)| (k.to_string(), dynamic_to_property_value(v)))
        .collect()
}

/// Convert a Rhai Map to resource options
fn map_to_resource_options(map: &Map) -> crate::registry::RegisteredResourceOptions {
    use crate::registry::RegisteredResourceOptions;

    let mut opts = RegisteredResourceOptions::default();

    if let Some(deps) = map.get("depends_on") {
        if let Some(arr) = deps.clone().try_cast::<Vec<Dynamic>>() {
            opts.depends_on = arr
                .iter()
                .filter_map(|v| v.clone().try_cast::<ImmutableString>())
                .map(|s| s.to_string())
                .collect();
        }
    }

    if let Some(protect) = map.get("protect") {
        opts.protect = protect.as_bool().unwrap_or(false);
    }

    if let Some(provider) = map.get("provider") {
        if let Some(s) = provider.clone().try_cast::<ImmutableString>() {
            opts.provider = Some(s.to_string());
        }
    }

    if let Some(ignore) = map.get("ignore_changes") {
        if let Some(arr) = ignore.clone().try_cast::<Vec<Dynamic>>() {
            opts.ignore_changes = arr
                .iter()
                .filter_map(|v| v.clone().try_cast::<ImmutableString>())
                .map(|s| s.to_string())
                .collect();
        }
    }

    if let Some(delete_before) = map.get("delete_before_replace") {
        opts.delete_before_replace = delete_before.as_bool().unwrap_or(false);
    }

    if let Some(aliases) = map.get("aliases") {
        if let Some(arr) = aliases.clone().try_cast::<Vec<Dynamic>>() {
            opts.aliases = arr
                .iter()
                .filter_map(|v| v.clone().try_cast::<ImmutableString>())
                .map(|s| s.to_string())
                .collect();
        }
    }

    if let Some(import_id) = map.get("import_id") {
        if let Some(s) = import_id.clone().try_cast::<ImmutableString>() {
            opts.import_id = Some(s.to_string());
        }
    }

    opts
}

/// Convert a Rhai Map to component options
fn map_to_component_options(map: &Map) -> crate::registry::ComponentOptions {
    use crate::registry::ComponentOptions;

    let mut opts = ComponentOptions::default();

    if let Some(deps) = map.get("depends_on") {
        if let Some(arr) = deps.clone().try_cast::<Vec<Dynamic>>() {
            opts.depends_on = arr
                .iter()
                .filter_map(|v| v.clone().try_cast::<ImmutableString>())
                .map(|s| s.to_string())
                .collect();
        }
    }

    if let Some(protect) = map.get("protect") {
        opts.protect = protect.as_bool().unwrap_or(false);
    }

    if let Some(providers) = map.get("providers") {
        if let Some(provider_map) = providers.clone().try_cast::<Map>() {
            opts.providers = provider_map
                .iter()
                .filter_map(|(k, v)| {
                    v.clone()
                        .try_cast::<ImmutableString>()
                        .map(|s| (k.to_string(), s.to_string()))
                })
                .collect();
        }
    }

    if let Some(aliases) = map.get("aliases") {
        if let Some(arr) = aliases.clone().try_cast::<Vec<Dynamic>>() {
            opts.aliases = arr
                .iter()
                .filter_map(|v| v.clone().try_cast::<ImmutableString>())
                .map(|s| s.to_string())
                .collect();
        }
    }

    opts
}

/// Convert a serde_json::Value to a Rhai Dynamic
fn json_to_dynamic(value: &serde_json::Value) -> Dynamic {
    match value {
        serde_json::Value::Null => Dynamic::UNIT,
        serde_json::Value::Bool(b) => Dynamic::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Dynamic::from(i)
            } else if let Some(f) = n.as_f64() {
                Dynamic::from(f)
            } else {
                Dynamic::from(n.to_string())
            }
        }
        serde_json::Value::String(s) => Dynamic::from(s.clone()),
        serde_json::Value::Array(arr) => {
            Dynamic::from(arr.iter().map(json_to_dynamic).collect::<Vec<_>>())
        }
        serde_json::Value::Object(obj) => {
            let map: Map = obj
                .iter()
                .map(|(k, v)| (k.clone().into(), json_to_dynamic(v)))
                .collect();
            Dynamic::from_map(map)
        }
    }
}

/// Convert a Rhai Dynamic value to a PropertyValue
fn dynamic_to_property_value(value: &Dynamic) -> PropertyValue {
    if value.is_unit() {
        PropertyValue::Null
    } else if let Some(b) = value.clone().try_cast::<bool>() {
        PropertyValue::Bool(b)
    } else if let Some(i) = value.clone().try_cast::<i64>() {
        PropertyValue::Int(i)
    } else if let Some(f) = value.clone().try_cast::<f64>() {
        PropertyValue::Float(f)
    } else if let Some(s) = value.clone().try_cast::<ImmutableString>() {
        PropertyValue::String(s.to_string())
    } else if let Some(arr) = value.clone().try_cast::<Vec<Dynamic>>() {
        PropertyValue::Array(arr.iter().map(dynamic_to_property_value).collect())
    } else if let Some(map) = value.clone().try_cast::<Map>() {
        PropertyValue::Object(
            map.iter()
                .map(|(k, v)| (k.to_string(), dynamic_to_property_value(v)))
                .collect(),
        )
    } else {
        PropertyValue::String(value.to_string())
    }
}

/// Convert a PropertyValue to a Rhai Dynamic
fn property_value_to_dynamic(value: &PropertyValue) -> Dynamic {
    match value {
        PropertyValue::Null => Dynamic::UNIT,
        PropertyValue::Bool(b) => Dynamic::from(*b),
        PropertyValue::Int(i) => Dynamic::from(*i),
        PropertyValue::Float(f) => Dynamic::from(*f),
        PropertyValue::String(s) => Dynamic::from(s.clone()),
        PropertyValue::Array(arr) => {
            Dynamic::from(arr.iter().map(property_value_to_dynamic).collect::<Vec<_>>())
        }
        PropertyValue::Object(obj) => {
            let map: Map = obj
                .iter()
                .map(|(k, v)| (k.clone().into(), property_value_to_dynamic(v)))
                .collect();
            Dynamic::from_map(map)
        }
        PropertyValue::Secret(_) => Dynamic::from("[SECRET]"),
        PropertyValue::OutputRef(output_ref) => {
            Dynamic::from(format!("${{{}:{}}}", output_ref.urn, output_ref.property))
        }
    }
}

/// Convert PropertyValues to a Rhai Dynamic Map
fn property_values_to_dynamic(values: &PropertyValues) -> Dynamic {
    let map: Map = values
        .iter()
        .map(|(k, v)| (k.clone().into(), property_value_to_dynamic(v)))
        .collect();
    Dynamic::from_map(map)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::fs;

    #[tokio::test]
    async fn test_rhai_runtime_available() {
        let runtime = RhaiRuntime::new();
        assert!(runtime.is_available().await);
    }

    #[tokio::test]
    async fn test_rhai_simple_script() {
        let runtime = RhaiRuntime::new();
        let temp = TempDir::new().unwrap();

        fs::write(
            temp.path().join("main.rhai"),
            r#"
let x = 1 + 2;
let msg = "Result: " + x;
log(msg);
"#,
        )
        .await
        .unwrap();

        let config = RuntimeConfig::new(RuntimeKind::Rhai, temp.path().to_path_buf(), "test");

        let result = runtime.run(&config).await.unwrap();
        if !result.success {
            eprintln!("Errors: {:?}", result.errors);
            eprintln!("Stderr: {}", result.stderr);
        }
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_rhai_resource_registration() {
        let runtime = RhaiRuntime::new();
        let temp = TempDir::new().unwrap();

        fs::write(
            temp.path().join("main.rhai"),
            r#"
// Register a resource
let bucket_inputs = #{
    name: "my-bucket",
    versioning: true
};

let urn = resource("aws:s3:Bucket", "my-bucket", bucket_inputs);
log("Created resource: " + urn);

// Export outputs
output("bucket_urn", urn);
output("bucket_name", "my-bucket");
"#,
        )
        .await
        .unwrap();

        let config = RuntimeConfig::new(RuntimeKind::Rhai, temp.path().to_path_buf(), "test");

        let result = runtime.run(&config).await.unwrap();
        if !result.success {
            eprintln!("Errors: {:?}", result.errors);
            eprintln!("Stderr: {}", result.stderr);
        }
        assert!(result.success);
        assert_eq!(result.resources.len(), 1);
    }

    #[tokio::test]
    async fn test_rhai_config_access() {
        let runtime = RhaiRuntime::new();
        let temp = TempDir::new().unwrap();

        fs::write(
            temp.path().join("main.rhai"),
            r#"
// Access stack info
let s = stack();
let p = project();
let preview = is_preview();

log("Stack: " + s);
log("Project: " + p);
log("Preview: " + preview);
"#,
        )
        .await
        .unwrap();

        let config = RuntimeConfig::new(RuntimeKind::Rhai, temp.path().to_path_buf(), "dev")
            .with_preview(true);

        let result = runtime.run(&config).await.unwrap();
        assert!(result.success);
    }

    #[test]
    fn test_execute_with_context() {
        let config_provider = Arc::new(
            SimpleConfigProvider::new("test-project", "dev")
                .with_value("aws:region", "us-east-1"),
        );
        let resource_provider = Arc::new(SimpleResourceProvider::new("test-project", "dev"));

        let ctx = Arc::new(RuntimeContext::new(
            config_provider,
            resource_provider.clone(),
            "dev",
            false,
        ));

        let runtime = RhaiRuntime::new();
        let result = runtime.execute_with_context(
            r#"
let region = config("aws:region", "us-west-2");
log("Region: " + region);
output("region", region);
"#,
            ctx,
        );

        assert!(result.is_ok());

        // Check the output was recorded
        let outputs = resource_provider.registry().stack_outputs();
        assert!(outputs.contains_key("region"));
    }
}
