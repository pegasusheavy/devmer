//! Types for Cloudmer API communication.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A project in Cloudmer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    /// Unique project ID.
    pub id: String,
    /// Project name.
    pub name: String,
    /// Project description.
    pub description: Option<String>,
    /// Organization ID.
    pub organization_id: String,
    /// Cloud providers connected.
    pub providers: Vec<String>,
    /// Created timestamp.
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Infrastructure state to sync with Cloudmer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InfrastructureState {
    /// Stack name in Devmer.
    pub stack_name: String,
    /// Stack environment (e.g., "production", "staging").
    pub environment: Option<String>,
    /// Resources in the stack.
    pub resources: Vec<ResourceState>,
    /// Last deployment timestamp.
    pub last_deployed_at: Option<DateTime<Utc>>,
    /// Devmer version.
    pub devmer_version: String,
    /// Additional metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// State of a single resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceState {
    /// Resource URN.
    pub urn: String,
    /// Resource type.
    pub resource_type: String,
    /// Resource name.
    pub name: String,
    /// Cloud provider.
    pub provider: String,
    /// Region/location.
    pub region: Option<String>,
    /// Provider-specific resource ID.
    pub provider_id: Option<String>,
    /// Resource status.
    pub status: ResourceStatus,
    /// Resource properties.
    pub properties: HashMap<String, serde_json::Value>,
    /// Resource tags.
    pub tags: HashMap<String, String>,
    /// Dependencies (URNs).
    pub dependencies: Vec<String>,
    /// Created timestamp.
    pub created_at: Option<DateTime<Utc>>,
    /// Last modified timestamp.
    pub modified_at: Option<DateTime<Utc>>,
}

/// Status of a resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResourceStatus {
    /// Resource is active and healthy.
    Active,
    /// Resource is being created.
    Creating,
    /// Resource is being updated.
    Updating,
    /// Resource is being deleted.
    Deleting,
    /// Resource has been deleted.
    Deleted,
    /// Resource is in an error state.
    Error,
    /// Resource status is unknown.
    Unknown,
}

/// Deployment notification to send to Cloudmer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentNotification {
    /// Deployment ID (optional, generated if not provided).
    pub deployment_id: Option<String>,
    /// Stack name.
    pub stack_name: String,
    /// Deployment status.
    pub status: DeploymentStatus,
    /// Deployment operation type.
    pub operation: DeploymentOperation,
    /// Resources affected.
    pub resources_affected: ResourceChangeSummary,
    /// Started timestamp.
    pub started_at: DateTime<Utc>,
    /// Completed timestamp (if finished).
    pub completed_at: Option<DateTime<Utc>>,
    /// Duration in milliseconds.
    pub duration_ms: Option<u64>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Git commit SHA (if available).
    pub git_commit: Option<String>,
    /// Git branch (if available).
    pub git_branch: Option<String>,
    /// Triggered by (user or CI).
    pub triggered_by: Option<String>,
    /// Additional metadata.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Deployment status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentStatus {
    /// Deployment is pending.
    Pending,
    /// Deployment is in progress.
    InProgress,
    /// Deployment succeeded.
    Succeeded,
    /// Deployment failed.
    Failed,
    /// Deployment was cancelled.
    Cancelled,
    /// Deployment is being rolled back.
    RollingBack,
}

/// Type of deployment operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeploymentOperation {
    /// Create/update resources.
    Up,
    /// Preview changes.
    Preview,
    /// Destroy resources.
    Destroy,
    /// Refresh state.
    Refresh,
    /// Import resources.
    Import,
}

/// Summary of resource changes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceChangeSummary {
    /// Number of resources created.
    pub created: usize,
    /// Number of resources updated.
    pub updated: usize,
    /// Number of resources deleted.
    pub deleted: usize,
    /// Number of resources unchanged.
    pub unchanged: usize,
}

impl ResourceChangeSummary {
    /// Total resources affected (excluding unchanged).
    pub fn total_changed(&self) -> usize {
        self.created + self.updated + self.deleted
    }

    /// Total resources.
    pub fn total(&self) -> usize {
        self.created + self.updated + self.deleted + self.unchanged
    }
}

/// Response from Cloudmer API for state sync.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponse {
    /// Whether the sync was successful.
    pub success: bool,
    /// Sync ID for reference.
    pub sync_id: String,
    /// Number of resources synced.
    pub resources_synced: usize,
    /// Visualization URL.
    pub visualization_url: String,
    /// Any warnings during sync.
    pub warnings: Vec<String>,
}

/// Response from Cloudmer API for deployment notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeploymentResponse {
    /// Whether the notification was received.
    pub success: bool,
    /// Deployment ID in Cloudmer.
    pub deployment_id: String,
    /// URL to view the deployment.
    pub deployment_url: String,
}

/// User information from Cloudmer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    /// User ID.
    pub id: String,
    /// Email address.
    pub email: String,
    /// Display name.
    pub name: String,
    /// Organizations the user belongs to.
    pub organizations: Vec<OrganizationMembership>,
}

/// Organization membership.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrganizationMembership {
    /// Organization ID.
    pub organization_id: String,
    /// Organization name.
    pub organization_name: String,
    /// User's role in the organization.
    pub role: String,
}

/// Cost insights from Cloudmer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostInsights {
    /// Total monthly cost.
    pub total_monthly: f64,
    /// Cost by provider.
    pub by_provider: HashMap<String, f64>,
    /// Cost by resource type.
    pub by_resource_type: HashMap<String, f64>,
    /// Cost trend (percentage change from last month).
    pub trend_percentage: f64,
    /// Optimization recommendations.
    pub recommendations: Vec<CostRecommendation>,
}

/// Cost optimization recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CostRecommendation {
    /// Recommendation ID.
    pub id: String,
    /// Resource URN.
    pub resource_urn: String,
    /// Recommendation title.
    pub title: String,
    /// Recommendation description.
    pub description: String,
    /// Estimated monthly savings.
    pub estimated_savings: f64,
    /// Recommendation priority.
    pub priority: RecommendationPriority,
}

/// Priority of a recommendation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecommendationPriority {
    Low,
    Medium,
    High,
    Critical,
}
