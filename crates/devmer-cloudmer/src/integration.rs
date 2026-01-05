//! Cloudmer integration hooks and traits.
//!
//! This module defines the integration points between Devmer and Cloudmer.
//! All integrations are **optional** - Devmer works 100% standalone.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                          DEVMER (CLI)                               │
//! │                     Works 100% Standalone                           │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  devmer-core     │ devmer-state │ devmer-secrets │ devmer-audit    │
//! │  devmer-config   │ devmer-org   │ devmer-concurrency │ ...         │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │                    devmer-cloudmer (Optional)                       │
//! │              Hooks into core when CLOUDMER_TOKEN is set             │
//! └────────────────────────────────┬────────────────────────────────────┘
//!                                  │
//!                          (HTTPS API calls)
//!                                  │
//!                                  ▼
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                         CLOUDMER SERVICE                            │
//! │                    (cloudmer.app or self-hosted)                    │
//! ├─────────────────────────────────────────────────────────────────────┤
//! │  Visualization │ Cost Insights │ Team Collaboration │ Audit Logs   │
//! │  Distributed Locks │ Deployment History │ Alerts │ RBAC            │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ## What Works Without Cloudmer (Everything!)
//!
//! - ✅ Full infrastructure deployment (`devmer up`, `down`, `preview`)
//! - ✅ State management (local, S3, GCS, Azure, etc.)
//! - ✅ Secrets encryption (passphrase, KMS, Vault)
//! - ✅ Multi-language SDKs (Python, TypeScript, Go, Rhai)
//! - ✅ Provider ecosystem (AWS, GCP, Azure, etc.)
//! - ✅ Audit logging (local file, syslog)
//! - ✅ Single-user locking (via state backend)
//! - ✅ Organization/team management (local config)
//!
//! ## What Cloudmer Adds (Optional Enhancements)
//!
//! - 📊 **Infrastructure Visualization**: Interactive diagrams of your infra
//! - 💰 **Cost Insights**: Real-time cost tracking and optimization
//! - 👥 **Team Collaboration**: Multi-user coordination, comments
//! - 🔒 **Distributed Locking**: Enterprise multi-user concurrency control
//! - 📝 **Deployment History**: Full history with diffs and rollback
//! - 🔔 **Alerts & Notifications**: Slack, Teams, webhooks
//! - 📋 **Compliance Dashboards**: SOC2, HIPAA, PCI-DSS reports
//!
//! ## Usage
//!
//! ```rust,ignore
//! use devmer_cloudmer::{CloudmerHooks, HookContext};
//!
//! // Check if Cloudmer is configured
//! if CloudmerHooks::is_enabled() {
//!     let hooks = CloudmerHooks::from_env()?;
//!     
//!     // Before deployment
//!     hooks.on_deployment_start(&context).await?;
//!     
//!     // ... do deployment ...
//!     
//!     // After deployment
//!     hooks.on_deployment_complete(&context, &result).await?;
//! }
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::client::CloudmerClient;
use crate::config::CloudmerConfig;
use crate::error::Result;

/// Check if Cloudmer integration is enabled.
///
/// Returns true if CLOUDMER_TOKEN environment variable is set.
pub fn is_cloudmer_enabled() -> bool {
    std::env::var("CLOUDMER_TOKEN").is_ok()
}

/// Integration hooks for connecting Devmer to Cloudmer.
///
/// All methods are no-ops if Cloudmer is not configured.
pub struct CloudmerHooks {
    client: Option<Arc<CloudmerClient>>,
    config: CloudmerConfig,
}

impl CloudmerHooks {
    /// Create hooks from environment variables.
    ///
    /// Looks for:
    /// - `CLOUDMER_TOKEN` - API token (required for hooks to be active)
    /// - `CLOUDMER_PROJECT` - Project ID
    /// - `CLOUDMER_API_URL` - Custom API URL (for self-hosted)
    pub fn from_env() -> Result<Self> {
        let token = std::env::var("CLOUDMER_TOKEN").ok();
        let project_id = std::env::var("CLOUDMER_PROJECT").ok();
        let api_url = std::env::var("CLOUDMER_API_URL").ok();

        let mut config = CloudmerConfig::default();
        
        if let Some(token) = token {
            config.api_token = Some(token);
        }
        if let Some(project) = project_id {
            config.project_id = Some(project);
        }
        if let Some(url) = api_url {
            config.api_url = url;
        }

        let client = if config.is_valid() {
            Some(Arc::new(CloudmerClient::new(config.clone())?))
        } else {
            None
        };

        Ok(Self { client, config })
    }

    /// Create hooks from explicit configuration.
    pub fn from_config(config: CloudmerConfig) -> Result<Self> {
        let client = if config.is_valid() {
            Some(Arc::new(CloudmerClient::new(config.clone())?))
        } else {
            None
        };

        Ok(Self { client, config })
    }

    /// Create disabled hooks (no-op).
    pub fn disabled() -> Self {
        Self {
            client: None,
            config: CloudmerConfig::default(),
        }
    }

    /// Check if hooks are active.
    pub fn is_active(&self) -> bool {
        self.client.is_some()
    }

    /// Get the underlying client if active.
    pub fn client(&self) -> Option<&CloudmerClient> {
        self.client.as_deref()
    }

    /// Get configuration.
    pub fn config(&self) -> &CloudmerConfig {
        &self.config
    }
}

/// Context passed to hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    /// Stack name.
    pub stack: String,
    /// Project name.
    pub project: Option<String>,
    /// Environment (dev, staging, prod).
    pub environment: Option<String>,
    /// User performing the action.
    pub user: Option<String>,
    /// Git commit SHA.
    pub git_commit: Option<String>,
    /// Git branch.
    pub git_branch: Option<String>,
    /// CI/CD system (if running in CI).
    pub ci_system: Option<String>,
}

impl HookContext {
    /// Create a new context.
    pub fn new(stack: impl Into<String>) -> Self {
        Self {
            stack: stack.into(),
            project: None,
            environment: None,
            user: None,
            git_commit: None,
            git_branch: None,
            ci_system: None,
        }
    }

    /// Set project.
    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// Set environment.
    pub fn with_environment(mut self, env: impl Into<String>) -> Self {
        self.environment = Some(env.into());
        self
    }

    /// Set user.
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.user = Some(user.into());
        self
    }

    /// Auto-detect git info from environment.
    pub fn with_git_info(mut self) -> Self {
        // Common CI environment variables
        self.git_commit = std::env::var("GITHUB_SHA")
            .or_else(|_| std::env::var("CI_COMMIT_SHA"))
            .or_else(|_| std::env::var("GIT_COMMIT"))
            .ok();

        self.git_branch = std::env::var("GITHUB_REF_NAME")
            .or_else(|_| std::env::var("CI_COMMIT_BRANCH"))
            .or_else(|_| std::env::var("GIT_BRANCH"))
            .ok();

        self.ci_system = if std::env::var("GITHUB_ACTIONS").is_ok() {
            Some("github-actions".to_string())
        } else if std::env::var("GITLAB_CI").is_ok() {
            Some("gitlab-ci".to_string())
        } else if std::env::var("CIRCLECI").is_ok() {
            Some("circleci".to_string())
        } else if std::env::var("JENKINS_URL").is_ok() {
            Some("jenkins".to_string())
        } else {
            None
        };

        self
    }
}

/// Trait for types that can be synced to Cloudmer.
///
/// Implement this for any data type that should be sent to Cloudmer
/// when the integration is enabled.
#[async_trait]
pub trait CloudmerSyncable: Send + Sync {
    /// The endpoint path for this resource type.
    fn endpoint() -> &'static str;

    /// Sync this resource to Cloudmer.
    async fn sync_to_cloudmer(&self, client: &CloudmerClient) -> Result<SyncResult>;
}

/// Result of a sync operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncResult {
    /// Whether the sync succeeded.
    pub success: bool,
    /// Cloudmer resource ID.
    pub cloudmer_id: Option<String>,
    /// Dashboard URL for this resource.
    pub dashboard_url: Option<String>,
    /// Message from Cloudmer.
    pub message: Option<String>,
}

impl SyncResult {
    /// Create a successful sync result.
    pub fn success(cloudmer_id: impl Into<String>) -> Self {
        Self {
            success: true,
            cloudmer_id: Some(cloudmer_id.into()),
            dashboard_url: None,
            message: None,
        }
    }

    /// Create a skipped sync result (Cloudmer not enabled).
    pub fn skipped() -> Self {
        Self {
            success: true,
            cloudmer_id: None,
            dashboard_url: None,
            message: Some("Cloudmer integration not enabled".to_string()),
        }
    }

    /// Create a failed sync result.
    pub fn failed(message: impl Into<String>) -> Self {
        Self {
            success: false,
            cloudmer_id: None,
            dashboard_url: None,
            message: Some(message.into()),
        }
    }

    /// Set dashboard URL.
    pub fn with_dashboard_url(mut self, url: impl Into<String>) -> Self {
        self.dashboard_url = Some(url.into());
        self
    }
}

/// Integration points that Cloudmer enhances.
///
/// This enum documents what optional features Cloudmer provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudmerFeature {
    /// Infrastructure visualization dashboard.
    Visualization,
    /// Deployment history and tracking.
    DeploymentHistory,
    /// Cost insights and optimization.
    CostInsights,
    /// Distributed locking for multi-user.
    DistributedLocking,
    /// Audit log aggregation.
    AuditAggregation,
    /// Compliance dashboards.
    ComplianceDashboards,
    /// Team collaboration features.
    TeamCollaboration,
    /// Alerts and notifications.
    Alerts,
}

impl CloudmerFeature {
    /// Get the description of what this feature provides.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Visualization => "Interactive infrastructure diagrams",
            Self::DeploymentHistory => "Full deployment history with diffs",
            Self::CostInsights => "Real-time cost tracking and optimization",
            Self::DistributedLocking => "Enterprise multi-user concurrency control",
            Self::AuditAggregation => "Centralized audit log dashboard",
            Self::ComplianceDashboards => "SOC2, HIPAA, PCI-DSS compliance reports",
            Self::TeamCollaboration => "Comments, approvals, team workflows",
            Self::Alerts => "Slack, Teams, and webhook notifications",
        }
    }

    /// Check if this feature works without Cloudmer.
    ///
    /// Returns true if there's a standalone alternative.
    pub fn has_standalone_alternative(&self) -> bool {
        match self {
            Self::Visualization => false, // Unique to Cloudmer
            Self::DeploymentHistory => true, // Local stack history
            Self::CostInsights => false, // Unique to Cloudmer
            Self::DistributedLocking => true, // devmer-concurrency
            Self::AuditAggregation => true, // Local file backend
            Self::ComplianceDashboards => true, // Local report generation
            Self::TeamCollaboration => false, // Unique to Cloudmer
            Self::Alerts => true, // Can use webhooks directly
        }
    }

    /// Get the standalone alternative description.
    pub fn standalone_alternative(&self) -> Option<&'static str> {
        match self {
            Self::Visualization => None,
            Self::DeploymentHistory => Some("Use 'devmer stack history' for local history"),
            Self::CostInsights => None,
            Self::DistributedLocking => Some("devmer-concurrency provides single-instance locking"),
            Self::AuditAggregation => Some("Use file-based audit backend with SIEM export"),
            Self::ComplianceDashboards => Some("Use 'devmer-audit' for local compliance reports"),
            Self::TeamCollaboration => None,
            Self::Alerts => Some("Configure webhooks in devmer.toml"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hooks_from_empty_config() {
        // Create hooks from empty config (no token)
        let hooks = CloudmerHooks::from_config(CloudmerConfig::default()).unwrap();
        assert!(!hooks.is_active());
    }

    #[test]
    fn test_disabled_hooks() {
        let hooks = CloudmerHooks::disabled();
        assert!(!hooks.is_active());
        assert!(hooks.client().is_none());
    }

    #[test]
    fn test_hook_context() {
        let context = HookContext::new("production")
            .with_project("my-project")
            .with_environment("prod")
            .with_user("alice@example.com");

        assert_eq!(context.stack, "production");
        assert_eq!(context.project, Some("my-project".to_string()));
    }

    #[test]
    fn test_feature_alternatives() {
        assert!(!CloudmerFeature::Visualization.has_standalone_alternative());
        assert!(CloudmerFeature::DistributedLocking.has_standalone_alternative());
        assert!(CloudmerFeature::AuditAggregation.has_standalone_alternative());
    }
}
