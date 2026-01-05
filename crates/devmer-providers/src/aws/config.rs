//! AWS Provider configuration

use serde::{Deserialize, Serialize};
use std::env;

/// AWS Region
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AwsRegion(pub String);

impl Default for AwsRegion {
    fn default() -> Self {
        Self("us-east-1".to_string())
    }
}

impl From<&str> for AwsRegion {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

impl AsRef<str> for AwsRegion {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// AWS Credentials
#[derive(Debug, Clone, Default)]
pub struct AwsCredentials {
    /// Access key ID
    pub access_key_id: Option<String>,

    /// Secret access key
    pub secret_access_key: Option<String>,

    /// Session token (for temporary credentials)
    pub session_token: Option<String>,

    /// Profile name from ~/.aws/credentials
    pub profile: Option<String>,

    /// Role ARN to assume
    pub assume_role_arn: Option<String>,

    /// External ID for role assumption
    pub external_id: Option<String>,
}

impl AwsCredentials {
    /// Create credentials from environment variables
    pub fn from_env() -> Self {
        Self {
            access_key_id: env::var("AWS_ACCESS_KEY_ID").ok(),
            secret_access_key: env::var("AWS_SECRET_ACCESS_KEY").ok(),
            session_token: env::var("AWS_SESSION_TOKEN").ok(),
            profile: env::var("AWS_PROFILE").ok(),
            assume_role_arn: env::var("AWS_ROLE_ARN").ok(),
            external_id: env::var("AWS_EXTERNAL_ID").ok(),
        }
    }

    /// Check if credentials are available
    pub fn is_available(&self) -> bool {
        // Either explicit credentials or profile-based
        (self.access_key_id.is_some() && self.secret_access_key.is_some())
            || self.profile.is_some()
            || self.assume_role_arn.is_some()
    }
}

/// AWS Provider configuration
#[derive(Debug, Clone, Default)]
pub struct AwsConfig {
    /// AWS region
    pub region: AwsRegion,

    /// Credentials
    pub credentials: AwsCredentials,

    /// Default tags to apply to all resources
    pub default_tags: std::collections::HashMap<String, String>,

    /// Skip credential validation
    pub skip_credentials_validation: bool,

    /// Skip requesting account ID
    pub skip_requesting_account_id: bool,

    /// Allowed account IDs (for safety)
    pub allowed_account_ids: Vec<String>,

    /// Forbidden account IDs (for safety)
    pub forbidden_account_ids: Vec<String>,

    /// Custom endpoint URLs for services
    pub endpoints: std::collections::HashMap<String, String>,

    /// Maximum number of retries
    pub max_retries: Option<u32>,

    /// S3 force path style (for MinIO compatibility)
    pub s3_force_path_style: bool,
}

impl AwsConfig {
    /// Create configuration from environment
    pub fn from_env() -> Self {
        Self {
            region: env::var("AWS_REGION")
                .or_else(|_| env::var("AWS_DEFAULT_REGION"))
                .map(|r| AwsRegion(r))
                .unwrap_or_default(),
            credentials: AwsCredentials::from_env(),
            ..Default::default()
        }
    }

    /// Builder-style: set region
    pub fn with_region(mut self, region: impl Into<AwsRegion>) -> Self {
        self.region = region.into();
        self
    }

    /// Builder-style: set credentials
    pub fn with_credentials(mut self, credentials: AwsCredentials) -> Self {
        self.credentials = credentials;
        self
    }

    /// Builder-style: add default tag
    pub fn with_default_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.default_tags.insert(key.into(), value.into());
        self
    }

    /// Builder-style: set custom endpoint
    pub fn with_endpoint(mut self, service: impl Into<String>, url: impl Into<String>) -> Self {
        self.endpoints.insert(service.into(), url.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_default() {
        let region = AwsRegion::default();
        assert_eq!(region.0, "us-east-1");
    }

    #[test]
    fn test_credentials_from_env() {
        // Test with no env vars
        let creds = AwsCredentials::default();
        assert!(!creds.is_available());
    }

    #[test]
    fn test_config_builder() {
        let config = AwsConfig::default()
            .with_region("eu-west-1")
            .with_default_tag("Environment", "test")
            .with_endpoint("s3", "http://localhost:9000");

        assert_eq!(config.region.0, "eu-west-1");
        assert_eq!(config.default_tags.get("Environment"), Some(&"test".to_string()));
        assert_eq!(
            config.endpoints.get("s3"),
            Some(&"http://localhost:9000".to_string())
        );
    }
}
