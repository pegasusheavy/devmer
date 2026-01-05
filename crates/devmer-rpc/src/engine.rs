//! Engine service implementation
//!
//! The Engine service is called by language hosts to:
//! - Register resources and components
//! - Get configuration and secrets
//! - Log messages
//! - Read resource state

use crate::{LogRequest, LogSeverity, RegisterComponentRequest, RegisterComponentResponse, RegisterResourceRequest, RegisterResourceResponse};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};

/// Engine service trait for handling language host requests
#[async_trait]
pub trait EngineService: Send + Sync {
    /// Register a new resource
    async fn register_resource(
        &self,
        request: RegisterResourceRequest,
    ) -> Result<RegisterResourceResponse, EngineError>;

    /// Register a component resource
    async fn register_component(
        &self,
        request: RegisterComponentRequest,
    ) -> Result<RegisterComponentResponse, EngineError>;

    /// Set component outputs
    async fn register_component_outputs(
        &self,
        urn: String,
        outputs: serde_json::Value,
    ) -> Result<(), EngineError>;

    /// Get configuration value
    async fn get_config(&self, key: &str, namespace: Option<&str>) -> Result<Option<String>, EngineError>;

    /// Get secret value
    async fn get_secret(&self, key: &str, namespace: Option<&str>) -> Result<Option<Vec<u8>>, EngineError>;

    /// Log a message
    async fn log(&self, request: LogRequest) -> Result<(), EngineError>;

    /// Get root resource URN
    async fn get_root_resource(&self) -> Result<String, EngineError>;

    /// Get stack reference outputs
    async fn get_stack_reference(&self, name: &str) -> Result<serde_json::Value, EngineError>;
}

/// Engine errors
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Configuration not found: {0}")]
    ConfigNotFound(String),

    #[error("Secret not found: {0}")]
    SecretNotFound(String),

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Provider error: {0}")]
    ProviderError(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<EngineError> for Status {
    fn from(err: EngineError) -> Self {
        match err {
            EngineError::ResourceNotFound(msg) => Status::not_found(msg),
            EngineError::ConfigNotFound(msg) => Status::not_found(msg),
            EngineError::SecretNotFound(msg) => Status::not_found(msg),
            EngineError::InvalidRequest(msg) => Status::invalid_argument(msg),
            EngineError::ProviderError(msg) => Status::failed_precondition(msg),
            EngineError::Internal(msg) => Status::internal(msg),
        }
    }
}

/// gRPC server wrapper for Engine service
pub struct EngineServer<T: EngineService> {
    inner: Arc<T>,
}

impl<T: EngineService> EngineServer<T> {
    /// Create a new engine server
    pub fn new(service: T) -> Self {
        Self {
            inner: Arc::new(service),
        }
    }

    /// Get a reference to the inner service
    pub fn inner(&self) -> &T {
        &self.inner
    }
}

/// Event emitted by the engine during execution
#[derive(Debug, Clone)]
pub enum EngineEvent {
    /// Resource registration started
    ResourceRegistering {
        resource_type: String,
        name: String,
    },
    /// Resource registered successfully
    ResourceRegistered {
        urn: String,
        resource_type: String,
        name: String,
    },
    /// Component registered
    ComponentRegistered {
        urn: String,
        component_type: String,
        name: String,
    },
    /// Log message received
    Log {
        severity: LogSeverity,
        message: String,
        urn: Option<String>,
    },
    /// Error occurred
    Error {
        message: String,
        urn: Option<String>,
    },
}

/// Engine event sender
pub type EngineEventSender = mpsc::Sender<EngineEvent>;

/// Engine event receiver
pub type EngineEventReceiver = mpsc::Receiver<EngineEvent>;

/// Create an engine event channel
pub fn engine_event_channel(buffer: usize) -> (EngineEventSender, EngineEventReceiver) {
    mpsc::channel(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockEngineService;

    #[async_trait]
    impl EngineService for MockEngineService {
        async fn register_resource(
            &self,
            request: RegisterResourceRequest,
        ) -> Result<RegisterResourceResponse, EngineError> {
            Ok(RegisterResourceResponse {
                urn: format!("urn:devmer:test::{}::{}", request.resource_type, request.name),
                id: "test-id".to_string(),
                outputs: serde_json::json!({}),
                stable: true,
            })
        }

        async fn register_component(
            &self,
            request: RegisterComponentRequest,
        ) -> Result<RegisterComponentResponse, EngineError> {
            Ok(RegisterComponentResponse {
                urn: format!("urn:devmer:test::{}::{}", request.component_type, request.name),
            })
        }

        async fn register_component_outputs(
            &self,
            _urn: String,
            _outputs: serde_json::Value,
        ) -> Result<(), EngineError> {
            Ok(())
        }

        async fn get_config(&self, key: &str, _namespace: Option<&str>) -> Result<Option<String>, EngineError> {
            if key == "test" {
                Ok(Some("value".to_string()))
            } else {
                Ok(None)
            }
        }

        async fn get_secret(&self, _key: &str, _namespace: Option<&str>) -> Result<Option<Vec<u8>>, EngineError> {
            Ok(None)
        }

        async fn log(&self, _request: LogRequest) -> Result<(), EngineError> {
            Ok(())
        }

        async fn get_root_resource(&self) -> Result<String, EngineError> {
            Ok("urn:devmer:test::pulumi:pulumi:Stack::test".to_string())
        }

        async fn get_stack_reference(&self, _name: &str) -> Result<serde_json::Value, EngineError> {
            Ok(serde_json::json!({}))
        }
    }

    #[tokio::test]
    async fn test_register_resource() {
        let service = MockEngineService;
        let request = RegisterResourceRequest {
            resource_type: "aws:s3:Bucket".to_string(),
            name: "my-bucket".to_string(),
            inputs: serde_json::json!({"bucketName": "test"}),
            parent: None,
            provider: None,
            dependencies: vec![],
            protect: false,
            ignore_changes: vec![],
        };

        let response = service.register_resource(request).await.unwrap();
        assert!(response.urn.contains("aws:s3:Bucket"));
        assert!(response.urn.contains("my-bucket"));
    }
}
