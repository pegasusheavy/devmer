//! Application container

use crate::interfaces::*;
use crate::modules::*;
use devmer_config::DevmerConfig;
use shaku::{module, HasComponent};
use std::sync::Arc;

// Define the main application module
module! {
    pub AppModule {
        components = [
            ConfigServiceImpl,
            StateServiceImpl,
            ProviderRegistryServiceImpl,
            RuntimeServiceImpl,
            ExecutionServiceImpl,
        ],
        providers = []
    }
}

/// Application container wrapping the shaku module
pub struct AppContainer {
    module: AppModule,
}

impl AppContainer {
    /// Create a new application container
    pub fn new(config: DevmerConfig) -> Self {
        let module = AppModule::builder()
            .with_component_parameters::<ConfigServiceImpl>(ConfigServiceImplParameters {
                config,
            })
            .with_component_parameters::<StateServiceImpl>(StateServiceImplParameters {
                backend: None,
            })
            .build();

        Self { module }
    }

    /// Get the config service
    pub fn config_service(&self) -> Arc<dyn ConfigService> {
        self.module.resolve()
    }

    /// Get the state service
    pub fn state_service(&self) -> Arc<dyn StateService> {
        self.module.resolve()
    }

    /// Get the execution service
    pub fn execution_service(&self) -> Arc<dyn ExecutionService> {
        self.module.resolve()
    }

    /// Get the provider registry service
    pub fn provider_registry(&self) -> Arc<dyn ProviderRegistryService> {
        self.module.resolve()
    }

    /// Get the underlying module
    pub fn module(&self) -> &AppModule {
        &self.module
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_creation() {
        let config = DevmerConfig::new("test-project");
        let container = AppContainer::new(config);

        let config_service = container.config_service();
        assert!(config_service.config().name == "test-project");
    }
}
