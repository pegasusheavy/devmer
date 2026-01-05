//! Resource types and abstractions

use crate::types::PropertyValues;
use crate::{DevmerError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for a resource instance
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ResourceId(pub Uuid);

impl ResourceId {
    /// Create a new random resource ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create from an existing UUID
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Parse from string
    pub fn parse(s: &str) -> Result<Self> {
        Uuid::parse_str(s)
            .map(Self)
            .map_err(|e| DevmerError::InvalidUrn(format!("Invalid resource ID: {}", e)))
    }
}

impl Default for ResourceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ResourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Uniform Resource Name for a Devmer resource
///
/// Format: `urn:devmer:{stack}::{type}::{name}`
///
/// Internal representation caches separator positions for O(1) field access.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Urn {
    /// The full URN string
    raw: String,
    /// Byte index where stack ends (first "::" after prefix)
    stack_end: u16,
    /// Byte index where resource type ends (second "::")
    type_end: u16,
}

// Custom serde to maintain backwards compatibility
impl Serialize for Urn {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.raw)
    }
}

impl<'de> Deserialize<'de> for Urn {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Urn::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// Prefix for all URNs
const URN_PREFIX: &str = "urn:devmer:";
const URN_PREFIX_LEN: usize = 11; // "urn:devmer:".len()

impl Urn {
    /// Create a new URN
    pub fn new(stack: &str, resource_type: &str, name: &str) -> Self {
        // Pre-calculate total length to avoid reallocations
        let total_len = URN_PREFIX_LEN + stack.len() + 2 + resource_type.len() + 2 + name.len();
        let mut raw = String::with_capacity(total_len);
        raw.push_str(URN_PREFIX);
        raw.push_str(stack);
        let stack_end = raw.len();
        raw.push_str("::");
        raw.push_str(resource_type);
        let type_end = raw.len();
        raw.push_str("::");
        raw.push_str(name);

        Self {
            raw,
            stack_end: stack_end as u16,
            type_end: type_end as u16,
        }
    }

    /// Parse a URN from a string
    pub fn parse(s: &str) -> Result<Self> {
        if !s.starts_with(URN_PREFIX) {
            return Err(DevmerError::InvalidUrn(format!(
                "URN must start with 'urn:devmer:': {}",
                s
            )));
        }

        // Find separator positions using memchr for speed
        let after_prefix = &s[URN_PREFIX_LEN..];
        let first_sep = after_prefix.find("::").ok_or_else(|| {
            DevmerError::InvalidUrn(format!("URN must have at least 3 parts: {}", s))
        })?;
        let stack_end = URN_PREFIX_LEN + first_sep;

        let after_stack = &s[stack_end + 2..];
        let second_sep = after_stack.find("::").ok_or_else(|| {
            DevmerError::InvalidUrn(format!("URN must have at least 3 parts: {}", s))
        })?;
        let type_end = stack_end + 2 + second_sep;

        Ok(Self {
            raw: s.to_string(),
            stack_end: stack_end as u16,
            type_end: type_end as u16,
        })
    }

    /// Get the stack name from the URN - O(1)
    #[inline]
    pub fn stack(&self) -> &str {
        &self.raw[URN_PREFIX_LEN..self.stack_end as usize]
    }

    /// Get the resource type from the URN - O(1)
    #[inline]
    pub fn resource_type(&self) -> &str {
        &self.raw[(self.stack_end as usize + 2)..self.type_end as usize]
    }

    /// Get the resource name from the URN - O(1)
    #[inline]
    pub fn name(&self) -> &str {
        &self.raw[(self.type_end as usize + 2)..]
    }

    /// Get the full URN as a string
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl fmt::Display for Urn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl From<Urn> for String {
    fn from(urn: Urn) -> String {
        urn.raw
    }
}

/// Resource type identifier
///
/// Format: `{provider}:{module}:{type}` (e.g., `aws:s3:Bucket`)
///
/// Internal representation caches separator positions for O(1) field access.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResourceType {
    /// The full resource type string
    raw: String,
    /// Byte index of first ':' (end of provider)
    provider_end: u8,
    /// Byte index of second ':' (end of module)
    module_end: u8,
}

// Custom serde to maintain backwards compatibility
impl Serialize for ResourceType {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.raw)
    }
}

impl<'de> Deserialize<'de> for ResourceType {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        ResourceType::parse(&s).map_err(serde::de::Error::custom)
    }
}

impl ResourceType {
    /// Create a new resource type
    pub fn new(provider: &str, module: &str, type_name: &str) -> Self {
        // Pre-calculate total length to avoid reallocations
        let total_len = provider.len() + 1 + module.len() + 1 + type_name.len();
        let mut raw = String::with_capacity(total_len);
        raw.push_str(provider);
        let provider_end = raw.len();
        raw.push(':');
        raw.push_str(module);
        let module_end = raw.len();
        raw.push(':');
        raw.push_str(type_name);

        Self {
            raw,
            provider_end: provider_end as u8,
            module_end: module_end as u8,
        }
    }

    /// Parse from string
    pub fn parse(s: &str) -> Result<Self> {
        let first_colon = s.find(':').ok_or_else(|| {
            DevmerError::InvalidUrn(format!(
                "Resource type must have format 'provider:module:type': {}",
                s
            ))
        })?;

        let second_colon = s[first_colon + 1..].find(':').ok_or_else(|| {
            DevmerError::InvalidUrn(format!(
                "Resource type must have format 'provider:module:type': {}",
                s
            ))
        })?;

        // Ensure there's no fourth part
        let module_end = first_colon + 1 + second_colon;
        if s[module_end + 1..].contains(':') {
            return Err(DevmerError::InvalidUrn(format!(
                "Resource type must have format 'provider:module:type': {}",
                s
            )));
        }

        Ok(Self {
            raw: s.to_string(),
            provider_end: first_colon as u8,
            module_end: module_end as u8,
        })
    }

    /// Get the provider name - O(1)
    #[inline]
    pub fn provider(&self) -> &str {
        &self.raw[..self.provider_end as usize]
    }

    /// Get the module name - O(1)
    #[inline]
    pub fn module(&self) -> &str {
        &self.raw[(self.provider_end as usize + 1)..self.module_end as usize]
    }

    /// Get the type name - O(1)
    #[inline]
    pub fn type_name(&self) -> &str {
        &self.raw[(self.module_end as usize + 1)..]
    }

    /// Get the full type as a string
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.raw
    }
}

impl fmt::Display for ResourceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl std::str::FromStr for ResourceType {
    type Err = DevmerError;

    fn from_str(s: &str) -> Result<Self> {
        // Try to parse properly first
        if let Ok(rt) = Self::parse(s) {
            return Ok(rt);
        }
        // Fallback: accept malformed input for flexibility
        // Find positions or use defaults
        let first_colon = s.find(':').unwrap_or(s.len());
        let second_colon = if first_colon < s.len() {
            s[first_colon + 1..].find(':').map(|p| first_colon + 1 + p).unwrap_or(s.len())
        } else {
            s.len()
        };
        Ok(Self {
            raw: s.to_string(),
            provider_end: first_colon.min(255) as u8,
            module_end: second_colon.min(255) as u8,
        })
    }
}

impl Default for ResourceType {
    fn default() -> Self {
        Self {
            raw: "unknown:unknown:Unknown".to_string(),
            provider_end: 7,  // "unknown".len()
            module_end: 15,   // "unknown:unknown".len()
        }
    }
}

/// Options for creating a resource
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceOptions {
    /// Parent resource (for component resources)
    pub parent: Option<Urn>,

    /// Provider to use for this resource
    pub provider: Option<String>,

    /// Dependencies that must be created first
    pub depends_on: Vec<Urn>,

    /// Protect this resource from deletion
    pub protect: bool,

    /// Retain on delete (don't actually delete the cloud resource)
    pub retain_on_delete: bool,

    /// Custom timeout for operations (in seconds)
    pub custom_timeouts: Option<CustomTimeouts>,

    /// Ignore changes to specific properties
    pub ignore_changes: Vec<String>,

    /// Replace resource when these properties change
    pub replace_on_changes: Vec<String>,

    /// Additional aliases for import/rename
    pub aliases: Vec<Urn>,

    /// Delete before replace (default is replace-then-delete)
    pub delete_before_replace: bool,

    /// Import ID for importing existing resources
    pub import_id: Option<String>,
}

/// Custom timeouts for resource operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomTimeouts {
    /// Create timeout in seconds
    pub create: Option<u64>,
    /// Update timeout in seconds
    pub update: Option<u64>,
    /// Delete timeout in seconds
    pub delete: Option<u64>,
}

/// Output from a resource property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceOutput {
    /// The URN of the resource
    pub urn: Urn,

    /// The property name
    pub property: String,

    /// Whether the value is known yet
    pub known: bool,

    /// Whether this is a secret value
    pub secret: bool,
}

/// Current state of a resource
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceState {
    /// Resource is pending creation
    Pending,
    /// Resource is being created
    Creating,
    /// Resource was created successfully
    Created,
    /// Resource is being updated
    Updating,
    /// Resource was updated successfully
    Updated,
    /// Resource is being deleted
    Deleting,
    /// Resource was deleted
    Deleted,
    /// Resource operation failed
    Failed,
}

impl Default for ResourceState {
    fn default() -> Self {
        Self::Pending
    }
}

/// A resource in the Devmer system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Unique identifier
    pub id: ResourceId,

    /// Uniform Resource Name
    pub urn: Urn,

    /// Resource type
    pub resource_type: ResourceType,

    /// Human-readable name
    pub name: String,

    /// Input properties
    pub inputs: PropertyValues,

    /// Output properties (populated after creation)
    pub outputs: PropertyValues,

    /// Resource options
    #[serde(default)]
    pub options: ResourceOptions,

    /// Current state
    #[serde(default)]
    pub state: ResourceState,

    /// Provider-specific ID (e.g., AWS ARN)
    pub provider_id: Option<String>,

    /// When the resource was created
    pub created_at: Option<DateTime<Utc>>,

    /// When the resource was last modified
    pub modified_at: Option<DateTime<Utc>>,

    /// Custom data from the provider
    #[serde(default)]
    pub custom: serde_json::Value,
}

impl Resource {
    /// Create a new resource
    pub fn new(
        stack: &str,
        resource_type: ResourceType,
        name: &str,
        inputs: PropertyValues,
    ) -> Self {
        let urn = Urn::new(stack, resource_type.as_str(), name);
        Self {
            id: ResourceId::new(),
            urn,
            resource_type,
            name: name.to_string(),
            inputs,
            outputs: PropertyValues::new(),
            options: ResourceOptions::default(),
            state: ResourceState::Pending,
            provider_id: None,
            created_at: None,
            modified_at: None,
            custom: serde_json::Value::Null,
        }
    }

    /// Create a new resource with options
    pub fn with_options(mut self, options: ResourceOptions) -> Self {
        self.options = options;
        self
    }

    /// Check if this resource is a component resource
    pub fn is_component(&self) -> bool {
        self.resource_type.provider() == "devmer"
            && self.resource_type.module() == "component"
    }

    /// Get a dependency reference to this resource's output
    pub fn output(&self, property: &str) -> ResourceOutput {
        ResourceOutput {
            urn: self.urn.clone(),
            property: property.to_string(),
            known: self.state == ResourceState::Created || self.state == ResourceState::Updated,
            secret: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_urn_parsing() {
        let urn = Urn::new("my-stack", "aws:s3:Bucket", "my-bucket");
        assert_eq!(urn.stack(), "my-stack");
        assert_eq!(urn.resource_type(), "aws:s3:Bucket");
        assert_eq!(urn.name(), "my-bucket");
    }

    #[test]
    fn test_resource_type_parsing() {
        let rt = ResourceType::new("aws", "s3", "Bucket");
        assert_eq!(rt.provider(), "aws");
        assert_eq!(rt.module(), "s3");
        assert_eq!(rt.type_name(), "Bucket");
    }

    #[test]
    fn test_resource_creation() {
        let resource = Resource::new(
            "test-stack",
            ResourceType::new("aws", "s3", "Bucket"),
            "my-bucket",
            PropertyValues::new(),
        );

        assert_eq!(resource.name, "my-bucket");
        assert_eq!(resource.state, ResourceState::Pending);
        assert!(resource.urn.as_str().contains("my-bucket"));
    }
}
