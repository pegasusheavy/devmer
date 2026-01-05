//! # devmer-cloudmer
//!
//! **Optional** integration with [Cloudmer](https://cloudmer.app) - a multi-cloud
//! infrastructure visualization, auditing, and cost optimization platform.
//!
//! ## ⚠️ Important: Devmer Works 100% Without Cloudmer
//!
//! Cloudmer integration is **completely optional**. Devmer is a fully standalone
//! IaC tool that works perfectly without any external services:
//!
//! | Feature | Without Cloudmer | With Cloudmer |
//! |---------|-----------------|---------------|
//! | Deploy infrastructure | ✅ Full support | ✅ + Visualization |
//! | State management | ✅ S3/GCS/Azure/Local | ✅ + Dashboard |
//! | Secrets encryption | ✅ Passphrase/KMS/Vault | ✅ Same |
//! | Multi-language SDKs | ✅ Python/TS/Go/Rhai | ✅ Same |
//! | Single-user locking | ✅ State backend | ✅ Same |
//! | Multi-user locking | ✅ devmer-concurrency | ✅ + Distributed |
//! | Audit logging | ✅ File/Syslog | ✅ + Dashboard |
//! | Compliance reports | ✅ Local generation | ✅ + Dashboard |
//! | Cost tracking | ❌ | ✅ Cost insights |
//! | Team collaboration | ❌ | ✅ Comments, approvals |
//!
//! ## When to Use Cloudmer
//!
//! Consider Cloudmer if you need:
//!
//! - 📊 **Infrastructure Visualization**: Interactive diagrams across clouds
//! - 💰 **Cost Insights**: Real-time cost tracking and optimization tips
//! - 👥 **Team Collaboration**: Multi-user coordination, comments, approvals
//! - 🔒 **Enterprise Locking**: Distributed locks across multiple machines
//! - 📋 **Compliance Dashboards**: Visual SOC2/HIPAA/PCI-DSS reports
//!
//! ## Getting Started
//!
//! 1. Create a free account at [cloudmer.app](https://cloudmer.app)
//! 2. Generate an API token from your account settings
//! 3. Set environment variable: `export CLOUDMER_TOKEN=your-token`
//!
//! That's it! Devmer will automatically sync deployments to Cloudmer.
//!
//! ## Example
//!
//! ```rust,ignore
//! use devmer_cloudmer::{
//!     CloudmerClient, CloudmerConfig,
//!     StateSyncBuilder, ResourceStateBuilder, ResourceStatus,
//!     DeploymentNotificationBuilder, DeploymentOperation,
//! };
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create client with your API token
//!     let client = CloudmerClient::from_token("your-api-token")?
//!         .with_project("proj-123");
//!
//!     // Sync infrastructure state
//!     let resource = ResourceStateBuilder::new(
//!         "urn:devmer:prod::aws:s3:Bucket::my-bucket",
//!         "aws:s3:Bucket",
//!         "my-bucket",
//!         "aws",
//!     )
//!     .region("us-east-1")
//!     .provider_id("my-bucket-12345")
//!     .status(ResourceStatus::Active)
//!     .tag("environment", "production")
//!     .build();
//!
//!     let response = StateSyncBuilder::new("production")
//!         .environment("prod")
//!         .add_resource(resource)
//!         .sync(&client)
//!         .await?;
//!
//!     println!("View your infrastructure: {}", response.visualization_url);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Integration Hooks
//!
//! Use `CloudmerHooks` to integrate Cloudmer into your workflow:
//!
//! ```rust,ignore
//! use devmer_cloudmer::{CloudmerHooks, is_cloudmer_enabled};
//!
//! // Check if Cloudmer is configured (CLOUDMER_TOKEN is set)
//! if is_cloudmer_enabled() {
//!     let hooks = CloudmerHooks::from_env()?;
//!     
//!     // Hooks are active, sync will happen
//!     hooks.on_deployment_complete(&context, &result).await?;
//! } else {
//!     // No Cloudmer - everything still works locally
//! }
//! ```
//!
//! ## CLI Commands
//!
//! ```bash
//! # Login to Cloudmer
//! devmer cloudmer login
//!
//! # Link current stack to a Cloudmer project
//! devmer cloudmer link
//!
//! # Sync current state
//! devmer cloudmer sync
//!
//! # View infrastructure in browser
//! devmer cloudmer open
//! ```
//!
//! ## Configuration
//!
//! Add to your `devmer.toml`:
//!
//! ```toml
//! [cloudmer]
//! api_token = "${CLOUDMER_TOKEN}"  # or set CLOUDMER_TOKEN env var
//! project_id = "proj-123"
//! auto_sync = true  # Sync after every deployment
//! notifications_enabled = true
//! ```
//!
//! ## Privacy
//!
//! - Only metadata is sent to Cloudmer (resource types, IDs, tags)
//! - No secrets or sensitive property values are transmitted
//! - All data is encrypted in transit and at rest
//! - See [cloudmer.app/privacy](https://cloudmer.app/privacy) for details

pub mod client;
pub mod config;
pub mod error;
pub mod integration;
pub mod locking;
pub mod notifications;
pub mod sync;
pub mod types;

// Re-export main types
pub use client::{CloudmerClient, DeviceAuthResponse, TokenResponse};
pub use config::CloudmerConfig;
pub use error::{CloudmerError, Result};

// Integration hooks
pub use integration::{
    is_cloudmer_enabled, CloudmerFeature, CloudmerHooks, CloudmerSyncable, HookContext, SyncResult,
};

// Locking
pub use locking::{
    AcquireLockRequest, AcquireLockResponse, CloudmerLock, CloudmerLockingClient,
    LockStatusResponse, QueuedLockRequest,
};

// Notifications
pub use notifications::{
    capture_git_info, capture_triggered_by, notification_with_ci_context,
    DeploymentNotificationBuilder,
};

// State sync
pub use sync::{convert_devmer_state, ResourceStateBuilder, StateSyncBuilder};

// Types
pub use types::{
    CostInsights, CostRecommendation, DeploymentNotification, DeploymentOperation,
    DeploymentResponse, DeploymentStatus, InfrastructureState, OrganizationMembership,
    Project, RecommendationPriority, ResourceChangeSummary, ResourceState, ResourceStatus,
    SyncResponse, User,
};
