//! Deployment notifications to Cloudmer.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::client::CloudmerClient;
use crate::error::Result;
use crate::types::{
    DeploymentNotification, DeploymentOperation, DeploymentResponse, DeploymentStatus,
    ResourceChangeSummary,
};

/// Builder for deployment notifications.
#[derive(Debug)]
pub struct DeploymentNotificationBuilder {
    stack_name: String,
    operation: DeploymentOperation,
    status: DeploymentStatus,
    started_at: DateTime<Utc>,
    completed_at: Option<DateTime<Utc>>,
    resources: ResourceChangeSummary,
    error: Option<String>,
    git_commit: Option<String>,
    git_branch: Option<String>,
    triggered_by: Option<String>,
    metadata: HashMap<String, serde_json::Value>,
}

impl DeploymentNotificationBuilder {
    /// Create a new notification builder.
    pub fn new(stack_name: impl Into<String>, operation: DeploymentOperation) -> Self {
        Self {
            stack_name: stack_name.into(),
            operation,
            status: DeploymentStatus::InProgress,
            started_at: Utc::now(),
            completed_at: None,
            resources: ResourceChangeSummary::default(),
            error: None,
            git_commit: None,
            git_branch: None,
            triggered_by: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a notification for a deployment start.
    pub fn started(stack_name: impl Into<String>, operation: DeploymentOperation) -> Self {
        Self::new(stack_name, operation).status(DeploymentStatus::InProgress)
    }

    /// Create a notification for a successful deployment.
    pub fn succeeded(stack_name: impl Into<String>, operation: DeploymentOperation) -> Self {
        let mut builder = Self::new(stack_name, operation);
        builder.status = DeploymentStatus::Succeeded;
        builder.completed_at = Some(Utc::now());
        builder
    }

    /// Create a notification for a failed deployment.
    pub fn failed(
        stack_name: impl Into<String>,
        operation: DeploymentOperation,
        error: impl Into<String>,
    ) -> Self {
        let mut builder = Self::new(stack_name, operation);
        builder.status = DeploymentStatus::Failed;
        builder.completed_at = Some(Utc::now());
        builder.error = Some(error.into());
        builder
    }

    /// Set the status.
    pub fn status(mut self, status: DeploymentStatus) -> Self {
        self.status = status;
        if matches!(status, DeploymentStatus::Succeeded | DeploymentStatus::Failed | DeploymentStatus::Cancelled) {
            self.completed_at = Some(Utc::now());
        }
        self
    }

    /// Set the start time.
    pub fn started_at(mut self, time: DateTime<Utc>) -> Self {
        self.started_at = time;
        self
    }

    /// Set the completion time.
    pub fn completed_at(mut self, time: DateTime<Utc>) -> Self {
        self.completed_at = Some(time);
        self
    }

    /// Set the resource change summary.
    pub fn resources(mut self, summary: ResourceChangeSummary) -> Self {
        self.resources = summary;
        self
    }

    /// Set resource counts directly.
    pub fn resource_counts(
        mut self,
        created: usize,
        updated: usize,
        deleted: usize,
        unchanged: usize,
    ) -> Self {
        self.resources = ResourceChangeSummary {
            created,
            updated,
            deleted,
            unchanged,
        };
        self
    }

    /// Set the error message.
    pub fn error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }

    /// Set the git commit SHA.
    pub fn git_commit(mut self, commit: impl Into<String>) -> Self {
        self.git_commit = Some(commit.into());
        self
    }

    /// Set the git branch.
    pub fn git_branch(mut self, branch: impl Into<String>) -> Self {
        self.git_branch = Some(branch.into());
        self
    }

    /// Set who triggered the deployment.
    pub fn triggered_by(mut self, user: impl Into<String>) -> Self {
        self.triggered_by = Some(user.into());
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl serde::Serialize) -> Self {
        self.metadata.insert(
            key.into(),
            serde_json::to_value(value).unwrap_or(serde_json::Value::Null),
        );
        self
    }

    /// Build the notification.
    pub fn build(self) -> DeploymentNotification {
        let duration_ms = self.completed_at.map(|end| {
            (end - self.started_at).num_milliseconds() as u64
        });

        DeploymentNotification {
            deployment_id: None,
            stack_name: self.stack_name,
            status: self.status,
            operation: self.operation,
            resources_affected: self.resources,
            started_at: self.started_at,
            completed_at: self.completed_at,
            duration_ms,
            error: self.error,
            git_commit: self.git_commit,
            git_branch: self.git_branch,
            triggered_by: self.triggered_by,
            metadata: self.metadata,
        }
    }

    /// Send the notification to Cloudmer.
    pub async fn send(self, client: &CloudmerClient) -> Result<DeploymentResponse> {
        let notification = self.build();
        client.notify_deployment(&notification).await
    }
}

/// Helper to capture git information from environment.
pub fn capture_git_info() -> (Option<String>, Option<String>) {
    let commit = std::env::var("GITHUB_SHA")
        .or_else(|_| std::env::var("GITLAB_CI_COMMIT_SHA"))
        .or_else(|_| std::env::var("BITBUCKET_COMMIT"))
        .or_else(|_| std::env::var("GIT_COMMIT"))
        .ok();

    let branch = std::env::var("GITHUB_REF_NAME")
        .or_else(|_| std::env::var("GITLAB_CI_COMMIT_REF_NAME"))
        .or_else(|_| std::env::var("BITBUCKET_BRANCH"))
        .or_else(|_| std::env::var("GIT_BRANCH"))
        .ok();

    (commit, branch)
}

/// Helper to capture the triggering user from environment.
pub fn capture_triggered_by() -> Option<String> {
    std::env::var("GITHUB_ACTOR")
        .or_else(|_| std::env::var("GITLAB_USER_LOGIN"))
        .or_else(|_| std::env::var("BITBUCKET_DEPLOYMENT_ENVIRONMENT"))
        .or_else(|_| std::env::var("USER"))
        .ok()
}

/// Create a notification with CI/CD context automatically captured.
pub fn notification_with_ci_context(
    stack_name: impl Into<String>,
    operation: DeploymentOperation,
) -> DeploymentNotificationBuilder {
    let (commit, branch) = capture_git_info();
    let triggered_by = capture_triggered_by();

    let mut builder = DeploymentNotificationBuilder::new(stack_name, operation);

    if let Some(commit) = commit {
        builder = builder.git_commit(commit);
    }
    if let Some(branch) = branch {
        builder = builder.git_branch(branch);
    }
    if let Some(user) = triggered_by {
        builder = builder.triggered_by(user);
    }

    // Capture CI system
    if std::env::var("GITHUB_ACTIONS").is_ok() {
        builder = builder.with_metadata("ci_system", "github-actions");
    } else if std::env::var("GITLAB_CI").is_ok() {
        builder = builder.with_metadata("ci_system", "gitlab-ci");
    } else if std::env::var("BITBUCKET_BUILD_NUMBER").is_ok() {
        builder = builder.with_metadata("ci_system", "bitbucket-pipelines");
    } else if std::env::var("JENKINS_URL").is_ok() {
        builder = builder.with_metadata("ci_system", "jenkins");
    } else if std::env::var("CIRCLECI").is_ok() {
        builder = builder.with_metadata("ci_system", "circleci");
    }

    builder
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_builder() {
        let notification = DeploymentNotificationBuilder::succeeded("production", DeploymentOperation::Up)
            .resource_counts(5, 2, 1, 10)
            .git_commit("abc123")
            .git_branch("main")
            .triggered_by("developer")
            .build();

        assert_eq!(notification.stack_name, "production");
        assert_eq!(notification.status, DeploymentStatus::Succeeded);
        assert_eq!(notification.operation, DeploymentOperation::Up);
        assert_eq!(notification.resources_affected.created, 5);
        assert_eq!(notification.git_commit, Some("abc123".to_string()));
        assert!(notification.completed_at.is_some());
        assert!(notification.duration_ms.is_some());
    }

    #[test]
    fn test_failed_notification() {
        let notification = DeploymentNotificationBuilder::failed(
            "staging",
            DeploymentOperation::Up,
            "Resource creation failed: quota exceeded",
        ).build();

        assert_eq!(notification.status, DeploymentStatus::Failed);
        assert!(notification.error.is_some());
        assert!(notification.error.unwrap().contains("quota exceeded"));
    }
}
