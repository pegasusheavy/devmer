//! Configuration for Cloudmer integration.

use serde::{Deserialize, Serialize};
use url::Url;

/// Default Cloudmer API base URL.
pub const DEFAULT_API_URL: &str = "https://api.cloudmer.app";

/// Default Cloudmer app URL.
pub const DEFAULT_APP_URL: &str = "https://cloudmer.app";

/// Configuration for connecting to Cloudmer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudmerConfig {
    /// API base URL.
    #[serde(default = "default_api_url")]
    pub api_url: String,

    /// App base URL (for generating links).
    #[serde(default = "default_app_url")]
    pub app_url: String,

    /// API token for authentication.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_token: Option<String>,

    /// Project ID in Cloudmer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,

    /// Organization ID in Cloudmer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,

    /// Enable automatic state sync after deployments.
    #[serde(default)]
    pub auto_sync: bool,

    /// Enable deployment notifications.
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,

    /// Timeout for API requests in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_api_url() -> String {
    DEFAULT_API_URL.to_string()
}

fn default_app_url() -> String {
    DEFAULT_APP_URL.to_string()
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    30
}

impl Default for CloudmerConfig {
    fn default() -> Self {
        Self {
            api_url: DEFAULT_API_URL.to_string(),
            app_url: DEFAULT_APP_URL.to_string(),
            api_token: None,
            project_id: None,
            organization_id: None,
            auto_sync: false,
            notifications_enabled: true,
            timeout_secs: 30,
        }
    }
}

impl CloudmerConfig {
    /// Create a new configuration with the given API token.
    pub fn with_token(token: impl Into<String>) -> Self {
        Self {
            api_token: Some(token.into()),
            ..Default::default()
        }
    }

    /// Set the project ID.
    pub fn with_project(mut self, project_id: impl Into<String>) -> Self {
        self.project_id = Some(project_id.into());
        self
    }

    /// Set the organization ID.
    pub fn with_organization(mut self, org_id: impl Into<String>) -> Self {
        self.organization_id = Some(org_id.into());
        self
    }

    /// Enable auto sync.
    pub fn with_auto_sync(mut self, enabled: bool) -> Self {
        self.auto_sync = enabled;
        self
    }

    /// Set custom API URL (for self-hosted or development).
    pub fn with_api_url(mut self, url: impl Into<String>) -> Self {
        self.api_url = url.into();
        self
    }

    /// Check if the configuration is valid for API calls.
    pub fn is_valid(&self) -> bool {
        self.api_token.is_some()
    }

    /// Check if project is linked.
    pub fn is_linked(&self) -> bool {
        self.project_id.is_some()
    }

    /// Build the full API URL for an endpoint.
    pub fn api_endpoint(&self, path: &str) -> String {
        format!("{}{}", self.api_url.trim_end_matches('/'), path)
    }

    /// Build the full app URL for a resource.
    pub fn app_link(&self, path: &str) -> String {
        format!("{}{}", self.app_url.trim_end_matches('/'), path)
    }

    /// Get the project dashboard URL.
    pub fn project_url(&self) -> Option<String> {
        self.project_id.as_ref().map(|id| {
            self.app_link(&format!("/projects/{}", id))
        })
    }

    /// Get the infrastructure visualization URL.
    pub fn visualization_url(&self) -> Option<String> {
        self.project_id.as_ref().map(|id| {
            self.app_link(&format!("/projects/{}/infrastructure", id))
        })
    }

    /// Validate the API URL.
    pub fn validate(&self) -> crate::error::Result<()> {
        Url::parse(&self.api_url).map_err(|e| {
            crate::error::CloudmerError::ConfigError(format!("invalid API URL: {}", e))
        })?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CloudmerConfig::default();
        assert_eq!(config.api_url, DEFAULT_API_URL);
        assert!(!config.is_valid());
        assert!(!config.is_linked());
    }

    #[test]
    fn test_config_with_token() {
        let config = CloudmerConfig::with_token("test-token")
            .with_project("proj-123")
            .with_organization("org-456");

        assert!(config.is_valid());
        assert!(config.is_linked());
        assert_eq!(config.project_id, Some("proj-123".to_string()));
    }

    #[test]
    fn test_api_endpoint() {
        let config = CloudmerConfig::default();
        assert_eq!(
            config.api_endpoint("/v1/projects"),
            "https://api.cloudmer.app/v1/projects"
        );
    }

    #[test]
    fn test_project_url() {
        let config = CloudmerConfig::with_token("token").with_project("proj-123");
        assert_eq!(
            config.project_url(),
            Some("https://cloudmer.app/projects/proj-123".to_string())
        );
    }
}
