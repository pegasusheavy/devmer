//! Provider registry for managing cloud providers

use crate::provider::{Provider, ProviderConfig, ProviderSchema};
use crate::Result;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tracing::{debug, info};

/// Registry for managing cloud provider instances
#[derive(Clone)]
pub struct ProviderRegistry {
    inner: Arc<RwLock<RegistryInner>>,
}

struct RegistryInner {
    /// Registered providers by name
    providers: HashMap<String, Arc<dyn Provider>>,

    /// Provider schemas by name
    schemas: HashMap<String, ProviderSchema>,

    /// Aliases for providers
    aliases: HashMap<String, String>,
}

impl ProviderRegistry {
    /// Create a new empty provider registry
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(RegistryInner {
                providers: HashMap::new(),
                schemas: HashMap::new(),
                aliases: HashMap::new(),
            })),
        }
    }

    /// Register a provider
    pub fn register(&self, name: impl Into<String>, provider: Arc<dyn Provider>) {
        let name = name.into();
        info!(provider = %name, "Registering provider");

        let mut inner = self.inner.write().unwrap();
        inner.providers.insert(name, provider);
    }

    /// Register an alias for a provider
    pub fn register_alias(&self, alias: impl Into<String>, target: impl Into<String>) {
        let alias = alias.into();
        let target = target.into();
        debug!(alias = %alias, target = %target, "Registering provider alias");

        let mut inner = self.inner.write().unwrap();
        inner.aliases.insert(alias, target);
    }

    /// Get a provider by name (resolves aliases)
    pub fn get(&self, name: &str) -> Option<Arc<dyn Provider>> {
        let inner = self.inner.read().unwrap();

        // Check for alias
        let resolved_name = inner.aliases.get(name).map(|s| s.as_str()).unwrap_or(name);

        inner.providers.get(resolved_name).cloned()
    }

    /// Get a provider schema by name
    pub fn get_schema(&self, name: &str) -> Option<ProviderSchema> {
        let inner = self.inner.read().unwrap();
        let resolved_name = inner.aliases.get(name).map(|s| s.as_str()).unwrap_or(name);
        inner.schemas.get(resolved_name).cloned()
    }

    /// Cache a provider schema
    pub fn cache_schema(&self, name: impl Into<String>, schema: ProviderSchema) {
        let mut inner = self.inner.write().unwrap();
        inner.schemas.insert(name.into(), schema);
    }

    /// List all registered provider names
    pub fn list(&self) -> Vec<String> {
        let inner = self.inner.read().unwrap();
        inner.providers.keys().cloned().collect()
    }

    /// List all aliases
    pub fn list_aliases(&self) -> Vec<(String, String)> {
        let inner = self.inner.read().unwrap();
        inner
            .aliases
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }

    /// Check if a provider is registered
    pub fn has(&self, name: &str) -> bool {
        let inner = self.inner.read().unwrap();
        let resolved_name = inner.aliases.get(name).map(|s| s.as_str()).unwrap_or(name);
        inner.providers.contains_key(resolved_name)
    }

    /// Remove a provider
    pub fn remove(&self, name: &str) -> Option<Arc<dyn Provider>> {
        let mut inner = self.inner.write().unwrap();
        inner.providers.remove(name)
    }

    /// Get the count of registered providers
    pub fn len(&self) -> usize {
        let inner = self.inner.read().unwrap();
        inner.providers.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Factory trait for creating provider instances
#[async_trait::async_trait]
pub trait ProviderFactory: Send + Sync {
    /// Get the provider name this factory creates
    fn name(&self) -> &str;

    /// Create a new provider instance with the given configuration
    async fn create(&self, config: ProviderConfig) -> Result<Arc<dyn Provider>>;

    /// Check if this provider is available (e.g., has required credentials)
    async fn is_available(&self) -> bool {
        true
    }
}

/// Registry of provider factories
#[derive(Default)]
pub struct ProviderFactoryRegistry {
    factories: HashMap<String, Arc<dyn ProviderFactory>>,
}

impl ProviderFactoryRegistry {
    /// Create a new empty factory registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a provider factory
    pub fn register(&mut self, factory: Arc<dyn ProviderFactory>) {
        let name = factory.name().to_string();
        self.factories.insert(name, factory);
    }

    /// Get a factory by provider name
    pub fn get(&self, name: &str) -> Option<Arc<dyn ProviderFactory>> {
        self.factories.get(name).cloned()
    }

    /// List all registered factory names
    pub fn list(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }

    /// Create a provider using the factory
    pub async fn create_provider(&self, config: ProviderConfig) -> Result<Arc<dyn Provider>> {
        let factory = self
            .factories
            .get(&config.name)
            .ok_or_else(|| crate::DevmerError::ProviderNotFound(config.name.clone()))?;

        factory.create(config).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::{CheckResult, DiffResult, OperationResult, ResourceSchema};
    use crate::resource::ResourceType;
    use crate::types::PropertyValues;
    use async_trait::async_trait;

    struct MockProvider {
        name: String,
    }

    #[async_trait]
    impl Provider for MockProvider {
        fn name(&self) -> &str {
            &self.name
        }

        fn version(&self) -> &str {
            "0.1.0"
        }

        async fn schema(&self) -> Result<ProviderSchema> {
            Ok(ProviderSchema {
                name: self.name.clone(),
                version: "0.1.0".to_string(),
                description: None,
                resources: HashMap::new(),
                config: None,
            })
        }

        async fn configure(&mut self, _config: ProviderConfig) -> Result<()> {
            Ok(())
        }

        async fn check(
            &self,
            _resource_type: &ResourceType,
            inputs: PropertyValues,
        ) -> Result<CheckResult> {
            Ok(CheckResult {
                inputs,
                failures: vec![],
            })
        }

        async fn diff(
            &self,
            _resource: &crate::Resource,
            _new_inputs: PropertyValues,
        ) -> Result<DiffResult> {
            Ok(DiffResult {
                changes: HashMap::new(),
                replace: false,
                replace_keys: vec![],
                stable_keys: vec![],
            })
        }

        async fn create(&self, resource: &crate::Resource) -> Result<OperationResult> {
            Ok(OperationResult::success(resource.clone()))
        }

        async fn read(&self, resource: &crate::Resource) -> Result<OperationResult> {
            Ok(OperationResult::success(resource.clone()))
        }

        async fn update(
            &self,
            resource: &crate::Resource,
            _new_inputs: PropertyValues,
        ) -> Result<OperationResult> {
            Ok(OperationResult::success(resource.clone()))
        }

        async fn delete(&self, resource: &crate::Resource) -> Result<OperationResult> {
            Ok(OperationResult::success(resource.clone()))
        }
    }

    #[test]
    fn test_provider_registry() {
        let registry = ProviderRegistry::new();

        let provider = Arc::new(MockProvider {
            name: "test".to_string(),
        });

        registry.register("test", provider);

        assert!(registry.has("test"));
        assert!(!registry.has("nonexistent"));

        let retrieved = registry.get("test");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name(), "test");
    }

    #[test]
    fn test_provider_aliases() {
        let registry = ProviderRegistry::new();

        let provider = Arc::new(MockProvider {
            name: "aws".to_string(),
        });

        registry.register("aws", provider);
        registry.register_alias("amazon", "aws");

        assert!(registry.has("aws"));
        assert!(registry.has("amazon"));

        let via_alias = registry.get("amazon");
        assert!(via_alias.is_some());
        assert_eq!(via_alias.unwrap().name(), "aws");
    }
}
