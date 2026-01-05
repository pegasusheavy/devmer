//! Provider service and client
//!
//! The Provider service handles resource CRUD operations:
//! - Check: Validate inputs
//! - Diff: Compare old vs new state
//! - Create/Read/Update/Delete: Resource lifecycle

use async_trait::async_trait;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use tonic::Status;

/// Provider service trait
#[async_trait]
pub trait ProviderService: Send + Sync {
    /// Get provider schema
    async fn get_schema(&self, version: i32) -> Result<String, ProviderError>;

    /// Configure the provider
    async fn configure(&self, request: ConfigureRequest) -> Result<ConfigureResponse, ProviderError>;

    /// Check/validate resource inputs
    async fn check(&self, request: CheckRequest) -> Result<CheckResponse, ProviderError>;

    /// Diff resource state
    async fn diff(&self, request: DiffRequest) -> Result<DiffResponse, ProviderError>;

    /// Create a resource
    async fn create(&self, request: CreateRequest) -> Result<CreateResponse, ProviderError>;

    /// Read a resource
    async fn read(&self, request: ReadRequest) -> Result<ReadResponse, ProviderError>;

    /// Update a resource
    async fn update(&self, request: UpdateRequest) -> Result<UpdateResponse, ProviderError>;

    /// Delete a resource
    async fn delete(&self, request: DeleteRequest) -> Result<(), ProviderError>;

    /// Invoke a provider function
    async fn invoke(&self, request: InvokeRequest) -> Result<InvokeResponse, ProviderError>;
}

/// Provider configuration request
#[derive(Debug, Clone)]
pub struct ConfigureRequest {
    /// Configuration variables
    pub variables: HashMap<String, String>,
    /// Configuration arguments
    pub args: JsonValue,
    /// Accept secrets in inputs
    pub accept_secrets: bool,
    /// Accept resource references
    pub accept_resources: bool,
}

/// Provider configuration response
#[derive(Debug, Clone)]
pub struct ConfigureResponse {
    /// Provider accepts secrets
    pub accept_secrets: bool,
    /// Provider accepts resource references
    pub accept_resources: bool,
    /// Provider supports preview
    pub supports_preview: bool,
}

/// Check request
#[derive(Debug, Clone)]
pub struct CheckRequest {
    /// Resource URN
    pub urn: String,
    /// Previous inputs
    pub olds: JsonValue,
    /// New inputs
    pub news: JsonValue,
    /// Random seed for deterministic operations
    pub random_seed: Option<Vec<u8>>,
}

/// Check response
#[derive(Debug, Clone)]
pub struct CheckResponse {
    /// Validated/normalized inputs
    pub inputs: JsonValue,
    /// Validation failures
    pub failures: Vec<CheckFailure>,
}

/// Check failure
#[derive(Debug, Clone)]
pub struct CheckFailure {
    /// Property path
    pub property: String,
    /// Failure reason
    pub reason: String,
}

/// Diff request
#[derive(Debug, Clone)]
pub struct DiffRequest {
    /// Resource URN
    pub urn: String,
    /// Resource ID
    pub id: String,
    /// Old state
    pub olds: JsonValue,
    /// New desired state
    pub news: JsonValue,
    /// Properties to ignore
    pub ignore_changes: Vec<String>,
}

/// Diff response
#[derive(Debug, Clone)]
pub struct DiffResponse {
    /// Properties that require replacement
    pub replaces: Vec<String>,
    /// Properties that are stable
    pub stables: Vec<String>,
    /// Delete before replace
    pub delete_before_replace: bool,
    /// Overall change status
    pub changes: DiffChanges,
    /// Changed properties
    pub diffs: Vec<String>,
    /// Detailed property diffs
    pub detailed_diff: HashMap<String, PropertyDiff>,
}

/// Diff changes status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffChanges {
    /// No changes
    None,
    /// Some changes
    Some,
    /// Unknown (preview)
    Unknown,
}

/// Property diff details
#[derive(Debug, Clone)]
pub struct PropertyDiff {
    /// Kind of diff
    pub kind: DiffKind,
    /// Whether this is an input diff
    pub input_diff: bool,
}

/// Diff kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffKind {
    /// Property added
    Add,
    /// Property added, requires replacement
    AddReplace,
    /// Property deleted
    Delete,
    /// Property deleted, requires replacement
    DeleteReplace,
    /// Property updated
    Update,
    /// Property updated, requires replacement
    UpdateReplace,
}

/// Create request
#[derive(Debug, Clone)]
pub struct CreateRequest {
    /// Resource URN
    pub urn: String,
    /// Input properties
    pub properties: JsonValue,
    /// Timeout in seconds
    pub timeout: f64,
    /// Preview mode
    pub preview: bool,
}

/// Create response
#[derive(Debug, Clone)]
pub struct CreateResponse {
    /// Resource ID
    pub id: String,
    /// Output properties
    pub properties: JsonValue,
}

/// Read request
#[derive(Debug, Clone)]
pub struct ReadRequest {
    /// Resource URN
    pub urn: String,
    /// Resource ID
    pub id: String,
    /// Known inputs
    pub inputs: JsonValue,
    /// Known state
    pub state: JsonValue,
}

/// Read response
#[derive(Debug, Clone)]
pub struct ReadResponse {
    /// Resource ID (may have changed)
    pub id: String,
    /// Refreshed inputs
    pub inputs: JsonValue,
    /// Refreshed properties
    pub properties: JsonValue,
}

/// Update request
#[derive(Debug, Clone)]
pub struct UpdateRequest {
    /// Resource URN
    pub urn: String,
    /// Resource ID
    pub id: String,
    /// Old state
    pub olds: JsonValue,
    /// New desired state
    pub news: JsonValue,
    /// Timeout in seconds
    pub timeout: f64,
    /// Properties to ignore
    pub ignore_changes: Vec<String>,
    /// Preview mode
    pub preview: bool,
}

/// Update response
#[derive(Debug, Clone)]
pub struct UpdateResponse {
    /// Updated properties
    pub properties: JsonValue,
}

/// Delete request
#[derive(Debug, Clone)]
pub struct DeleteRequest {
    /// Resource URN
    pub urn: String,
    /// Resource ID
    pub id: String,
    /// Current properties
    pub properties: JsonValue,
    /// Timeout in seconds
    pub timeout: f64,
}

/// Invoke request
#[derive(Debug, Clone)]
pub struct InvokeRequest {
    /// Function token (e.g., "aws:ec2/getAmi:getAmi")
    pub token: String,
    /// Function arguments
    pub args: JsonValue,
}

/// Invoke response
#[derive(Debug, Clone)]
pub struct InvokeResponse {
    /// Return value
    pub return_value: JsonValue,
    /// Failures
    pub failures: Vec<CheckFailure>,
}

/// Provider errors
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("Configuration error: {0}")]
    ConfigurationError(String),

    #[error("Operation timeout")]
    Timeout,

    #[error("Provider not configured")]
    NotConfigured,

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<ProviderError> for Status {
    fn from(err: ProviderError) -> Self {
        match err {
            ProviderError::ResourceNotFound(msg) => Status::not_found(msg),
            ProviderError::ValidationFailed(msg) => Status::invalid_argument(msg),
            ProviderError::ConfigurationError(msg) => Status::failed_precondition(msg),
            ProviderError::Timeout => Status::deadline_exceeded("Operation timed out"),
            ProviderError::NotConfigured => Status::failed_precondition("Provider not configured"),
            ProviderError::UnsupportedOperation(msg) => Status::unimplemented(msg),
            ProviderError::Internal(msg) => Status::internal(msg),
        }
    }
}

/// Provider client for connecting to external providers
pub struct ProviderClient {
    /// Provider name
    name: String,
    /// Server address
    address: String,
}

impl ProviderClient {
    /// Create a new provider client
    pub fn new(name: impl Into<String>, address: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            address: address.into(),
        }
    }

    /// Get provider name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get server address
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Connect to the provider
    pub async fn connect(&self) -> Result<(), ProviderError> {
        // In a real implementation, this would establish a gRPC connection
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_response() {
        let response = DiffResponse {
            replaces: vec!["name".to_string()],
            stables: vec!["arn".to_string()],
            delete_before_replace: false,
            changes: DiffChanges::Some,
            diffs: vec!["name".to_string(), "tags".to_string()],
            detailed_diff: HashMap::new(),
        };

        assert_eq!(response.changes, DiffChanges::Some);
        assert!(response.replaces.contains(&"name".to_string()));
    }

    #[test]
    fn test_check_failure() {
        let failure = CheckFailure {
            property: "bucketName".to_string(),
            reason: "Bucket name must be lowercase".to_string(),
        };

        assert_eq!(failure.property, "bucketName");
    }
}
