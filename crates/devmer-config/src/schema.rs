//! Configuration schema types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Root configuration structure (Devmer.toml)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DevmerConfig {
    /// Project name
    pub name: String,

    /// Project description
    pub description: Option<String>,

    /// Runtime/SDK configuration
    #[serde(default)]
    pub runtime: RuntimeConfig,

    /// State backend configuration
    #[serde(default)]
    pub backend: BackendConfig,

    /// Secrets configuration
    #[serde(default)]
    pub secrets: SecretsConfig,

    /// Stack configurations
    #[serde(default)]
    pub stack: HashMap<String, StackConfig>,

    /// Provider configurations
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfigEntry>,

    /// Plugin paths
    #[serde(default)]
    pub plugins: Vec<PathBuf>,

    /// Template variables
    #[serde(default)]
    pub template: HashMap<String, toml::Value>,
}

/// Runtime configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Runtime name (python, typescript, go, rhai)
    pub name: Option<String>,

    /// JavaScript runtime (node, deno, bun)
    pub js_runtime: Option<String>,

    /// Entry point file
    pub main: Option<PathBuf>,

    /// Additional options
    #[serde(default)]
    pub options: HashMap<String, toml::Value>,
}

/// State backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendConfig {
    /// Backend type (local, s3, gcs, azure, postgres, etc.)
    #[serde(rename = "type")]
    pub backend_type: String,

    /// Backend URL or path
    pub url: Option<String>,

    /// S3-specific: bucket name
    pub bucket: Option<String>,

    /// S3-specific: key prefix
    pub prefix: Option<String>,

    /// S3-specific: region
    pub region: Option<String>,

    /// S3-specific: DynamoDB table for locking
    pub lock_table: Option<String>,

    /// PostgreSQL-specific: connection string
    pub connection_string: Option<String>,

    /// Encryption enabled
    #[serde(default)]
    pub encrypt: bool,

    /// Encryption key ID (for KMS)
    pub encryption_key: Option<String>,

    /// Additional backend-specific options
    #[serde(default)]
    pub options: HashMap<String, toml::Value>,
}

impl Default for BackendConfig {
    fn default() -> Self {
        Self {
            backend_type: "local".to_string(),
            url: None,
            bucket: None,
            prefix: None,
            region: None,
            lock_table: None,
            connection_string: None,
            encrypt: false,
            encryption_key: None,
            options: HashMap::new(),
        }
    }
}

/// Secrets configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    /// Secrets provider (passphrase, awskms, gcpkms, vault, age)
    pub provider: String,

    /// Provider-specific configuration
    #[serde(flatten)]
    pub config: SecretsProviderConfig,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            provider: "passphrase".to_string(),
            config: SecretsProviderConfig::default(),
        }
    }
}

/// Secrets provider configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecretsProviderConfig {
    /// AWS KMS key ID
    pub kms_key_id: Option<String>,

    /// Vault address
    pub vault_address: Option<String>,

    /// Vault mount path
    pub vault_mount: Option<String>,

    /// Age recipients
    pub age_recipients: Option<Vec<String>>,

    /// Age identity file
    pub age_identity: Option<PathBuf>,

    /// Additional options
    #[serde(default)]
    pub options: HashMap<String, toml::Value>,
}

/// Stack-specific configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StackConfig {
    /// Stack description
    pub description: Option<String>,

    /// Secrets provider override
    pub secrets_provider: Option<String>,

    /// Backend override
    pub backend: Option<BackendConfig>,

    /// Configuration values
    #[serde(default)]
    pub config: HashMap<String, toml::Value>,

    /// Environment variables to set
    #[serde(default)]
    pub environment: HashMap<String, String>,

    /// Tags applied to all resources
    #[serde(default)]
    pub tags: HashMap<String, String>,
}

/// Provider configuration entry
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProviderConfigEntry {
    /// Provider version constraint
    pub version: Option<String>,

    /// Plugin source
    pub source: Option<String>,

    /// Default configuration
    #[serde(default)]
    pub config: HashMap<String, toml::Value>,
}

impl DevmerConfig {
    /// Create a new empty configuration
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Get a configuration value by key path
    pub fn get(&self, key: &str) -> Option<String> {
        let parts: Vec<&str> = key.split('.').collect();

        match parts.as_slice() {
            ["name"] => Some(self.name.clone()),
            ["description"] => self.description.clone(),
            ["backend", "type"] => Some(self.backend.backend_type.clone()),
            ["backend", "bucket"] => self.backend.bucket.clone(),
            ["backend", "region"] => self.backend.region.clone(),
            ["secrets", "provider"] => Some(self.secrets.provider.clone()),
            _ => None,
        }
    }

    /// Get stack configuration
    pub fn get_stack(&self, name: &str) -> Option<&StackConfig> {
        self.stack.get(name)
    }

    /// Get all stack names
    pub fn stack_names(&self) -> Vec<&str> {
        self.stack.keys().map(|s| s.as_str()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DevmerConfig::new("test-project");
        assert_eq!(config.name, "test-project");
        assert_eq!(config.backend.backend_type, "local");
        assert_eq!(config.secrets.provider, "passphrase");
    }

    #[test]
    fn test_config_get() {
        let mut config = DevmerConfig::new("my-project");
        config.backend.bucket = Some("my-bucket".to_string());

        assert_eq!(config.get("name"), Some("my-project".to_string()));
        assert_eq!(config.get("backend.bucket"), Some("my-bucket".to_string()));
        assert_eq!(config.get("nonexistent"), None);
    }
}
