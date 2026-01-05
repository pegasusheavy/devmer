//! Approval workflows

use crate::user::UserId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Approval status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalStatus {
    /// Waiting for approval
    Pending,
    /// Approved
    Approved,
    /// Rejected
    Rejected,
    /// Expired
    Expired,
    /// Cancelled by requester
    Cancelled,
}

/// A single approval response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalResponse {
    /// Who responded
    pub user_id: UserId,

    /// Their decision
    pub decision: ApprovalDecision,

    /// Optional comment
    pub comment: Option<String>,

    /// When they responded
    pub responded_at: DateTime<Utc>,
}

/// Decision by an approver
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ApprovalDecision {
    /// Approved
    Approve,
    /// Rejected
    Reject,
    /// Request changes
    RequestChanges,
}

/// An approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// Unique ID
    pub id: Uuid,

    /// Organization ID
    pub organization_id: Uuid,

    /// Who requested
    pub requester_id: UserId,

    /// What's being approved
    pub request_type: ApprovalRequestType,

    /// Current status
    pub status: ApprovalStatus,

    /// Minimum approvals required
    pub required_approvals: u32,

    /// Who can approve
    pub eligible_approvers: Vec<UserId>,

    /// Responses received
    pub responses: Vec<ApprovalResponse>,

    /// Request title
    pub title: String,

    /// Request description
    pub description: Option<String>,

    /// Related metadata
    pub metadata: HashMap<String, String>,

    /// When created
    pub created_at: DateTime<Utc>,

    /// When it expires
    pub expires_at: Option<DateTime<Utc>>,

    /// When it was resolved
    pub resolved_at: Option<DateTime<Utc>>,
}

/// Type of approval request
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApprovalRequestType {
    /// Deployment approval
    Deployment {
        /// Stack being deployed
        stack: String,
        /// Environment
        environment: String,
        /// Preview/plan ID
        plan_id: Option<String>,
        /// Resources being changed
        resources_changed: u32,
        /// Resources being created
        resources_created: u32,
        /// Resources being deleted
        resources_deleted: u32,
    },
    /// Stack deletion
    StackDeletion {
        /// Stack being deleted
        stack: String,
        /// Resource count
        resource_count: u32,
    },
    /// Access request
    AccessRequest {
        /// Resource being accessed
        resource: String,
        /// Access level requested
        access_level: String,
    },
    /// Team membership
    TeamMembership {
        /// Team slug
        team: String,
        /// User being added
        user_id: UserId,
        /// Role requested
        role: String,
    },
    /// Custom approval
    Custom {
        /// Custom type name
        custom_type: String,
        /// Custom data
        data: HashMap<String, String>,
    },
}

impl ApprovalRequest {
    /// Create a new deployment approval request
    pub fn deployment(
        organization_id: Uuid,
        requester_id: UserId,
        stack: impl Into<String>,
        environment: impl Into<String>,
    ) -> Self {
        Self {
            id: Uuid::new_v4(),
            organization_id,
            requester_id,
            request_type: ApprovalRequestType::Deployment {
                stack: stack.into(),
                environment: environment.into(),
                plan_id: None,
                resources_changed: 0,
                resources_created: 0,
                resources_deleted: 0,
            },
            status: ApprovalStatus::Pending,
            required_approvals: 1,
            eligible_approvers: Vec::new(),
            responses: Vec::new(),
            title: String::new(),
            description: None,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            expires_at: None,
            resolved_at: None,
        }
    }

    /// Set title
    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Set required approvals
    pub fn with_required_approvals(mut self, count: u32) -> Self {
        self.required_approvals = count;
        self
    }

    /// Add eligible approver
    pub fn add_approver(&mut self, user_id: UserId) {
        if !self.eligible_approvers.contains(&user_id) {
            self.eligible_approvers.push(user_id);
        }
    }

    /// Set expiration
    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Check if user can approve
    pub fn can_approve(&self, user_id: &UserId) -> bool {
        // Can't approve your own request
        if &self.requester_id == user_id {
            return false;
        }

        // Must be eligible
        if !self.eligible_approvers.contains(user_id) {
            return false;
        }

        // Must not have already responded
        if self.responses.iter().any(|r| &r.user_id == user_id) {
            return false;
        }

        // Must be pending
        self.status == ApprovalStatus::Pending
    }

    /// Add approval response
    pub fn respond(&mut self, user_id: UserId, decision: ApprovalDecision, comment: Option<String>) -> bool {
        if !self.can_approve(&user_id) {
            return false;
        }

        self.responses.push(ApprovalResponse {
            user_id,
            decision,
            comment,
            responded_at: Utc::now(),
        });

        // Update status
        self.update_status();
        true
    }

    /// Update status based on responses
    fn update_status(&mut self) {
        // Check for rejections
        let rejections = self.responses.iter().filter(|r| r.decision == ApprovalDecision::Reject).count();
        if rejections > 0 {
            self.status = ApprovalStatus::Rejected;
            self.resolved_at = Some(Utc::now());
            return;
        }

        // Check for enough approvals
        let approvals = self.responses.iter().filter(|r| r.decision == ApprovalDecision::Approve).count();
        if approvals >= self.required_approvals as usize {
            self.status = ApprovalStatus::Approved;
            self.resolved_at = Some(Utc::now());
        }
    }

    /// Check if expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.map(|e| e < Utc::now()).unwrap_or(false)
    }

    /// Cancel the request
    pub fn cancel(&mut self) {
        if self.status == ApprovalStatus::Pending {
            self.status = ApprovalStatus::Cancelled;
            self.resolved_at = Some(Utc::now());
        }
    }

    /// Get approval count
    pub fn approval_count(&self) -> usize {
        self.responses.iter().filter(|r| r.decision == ApprovalDecision::Approve).count()
    }

    /// Get rejection count
    pub fn rejection_count(&self) -> usize {
        self.responses.iter().filter(|r| r.decision == ApprovalDecision::Reject).count()
    }

    /// Check if approved
    pub fn is_approved(&self) -> bool {
        self.status == ApprovalStatus::Approved
    }

    /// Check if rejected
    pub fn is_rejected(&self) -> bool {
        self.status == ApprovalStatus::Rejected
    }

    /// Check if pending
    pub fn is_pending(&self) -> bool {
        self.status == ApprovalStatus::Pending
    }
}

/// Approval workflow configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalWorkflow {
    /// Workflow name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Patterns that trigger this workflow
    pub triggers: Vec<ApprovalTrigger>,

    /// Required number of approvals
    pub required_approvals: u32,

    /// Who can approve
    pub approvers: ApproverConfig,

    /// Auto-expire after (hours)
    pub expire_hours: Option<u32>,

    /// Allow self-approval
    pub allow_self_approval: bool,

    /// Notify on request
    pub notify_on_request: bool,

    /// Notify on resolution
    pub notify_on_resolution: bool,
}

/// What triggers an approval workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApprovalTrigger {
    /// Stack pattern
    StackPattern { pattern: String },
    /// Environment
    Environment { name: String },
    /// Resource type pattern
    ResourceType { pattern: String },
    /// Action type
    Action { action: String },
    /// Resource count threshold
    ResourceCountThreshold { min_resources: u32 },
    /// Deletion
    AnyDeletion,
}

/// Who can approve
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ApproverConfig {
    /// Specific users
    Users { user_ids: Vec<UserId> },
    /// Team members with role
    TeamRole { team: String, min_role: String },
    /// Organization role
    OrgRole { role: String },
    /// Any team owner
    AnyTeamOwner,
    /// Stack owner
    StackOwner,
}

impl ApprovalWorkflow {
    /// Create a new workflow
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            triggers: Vec::new(),
            required_approvals: 1,
            approvers: ApproverConfig::AnyTeamOwner,
            expire_hours: Some(72),
            allow_self_approval: false,
            notify_on_request: true,
            notify_on_resolution: true,
        }
    }

    /// Add trigger
    pub fn add_trigger(&mut self, trigger: ApprovalTrigger) {
        self.triggers.push(trigger);
    }

    /// Check if workflow matches a deployment
    pub fn matches(
        &self,
        stack: &str,
        environment: Option<&str>,
        resource_types: &[&str],
        action: &str,
        resource_count: u32,
        has_deletions: bool,
    ) -> bool {
        for trigger in &self.triggers {
            let matched = match trigger {
                ApprovalTrigger::StackPattern { pattern } => glob_match(pattern, stack),
                ApprovalTrigger::Environment { name } => environment == Some(name.as_str()),
                ApprovalTrigger::ResourceType { pattern } => {
                    resource_types.iter().any(|rt| glob_match(pattern, rt))
                }
                ApprovalTrigger::Action { action: a } => action == a,
                ApprovalTrigger::ResourceCountThreshold { min_resources } => {
                    resource_count >= *min_resources
                }
                ApprovalTrigger::AnyDeletion => has_deletions,
            };

            if matched {
                return true;
            }
        }

        false
    }
}

/// Simple glob matching
fn glob_match(pattern: &str, value: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    if pattern.ends_with("/*") {
        let prefix = &pattern[..pattern.len() - 2];
        return value.starts_with(prefix);
    }
    pattern == value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_request() {
        let org_id = Uuid::new_v4();
        let requester = UserId::new();
        let approver1 = UserId::new();
        let approver2 = UserId::new();

        let mut request = ApprovalRequest::deployment(org_id, requester.clone(), "marketing/website", "production")
            .with_title("Deploy marketing website")
            .with_required_approvals(2);

        request.add_approver(approver1.clone());
        request.add_approver(approver2.clone());

        // Can't approve own request
        assert!(!request.can_approve(&requester));

        // First approval
        assert!(request.respond(approver1.clone(), ApprovalDecision::Approve, None));
        assert!(request.is_pending());
        assert_eq!(request.approval_count(), 1);

        // Can't approve twice
        assert!(!request.can_approve(&approver1));

        // Second approval
        assert!(request.respond(approver2.clone(), ApprovalDecision::Approve, None));
        assert!(request.is_approved());
        assert_eq!(request.approval_count(), 2);
    }

    #[test]
    fn test_approval_rejection() {
        let org_id = Uuid::new_v4();
        let requester = UserId::new();
        let approver = UserId::new();

        let mut request = ApprovalRequest::deployment(org_id, requester, "app/prod", "production");
        request.add_approver(approver.clone());

        request.respond(approver, ApprovalDecision::Reject, Some("Too risky".to_string()));
        assert!(request.is_rejected());
    }

    #[test]
    fn test_approval_workflow() {
        let mut workflow = ApprovalWorkflow::new("Production Deployments");
        workflow.add_trigger(ApprovalTrigger::Environment { name: "production".to_string() });
        workflow.add_trigger(ApprovalTrigger::AnyDeletion);
        workflow.required_approvals = 2;

        // Matches production
        assert!(workflow.matches("app/api", Some("production"), &[], "deploy", 5, false));

        // Matches deletions
        assert!(workflow.matches("app/api", Some("staging"), &[], "deploy", 5, true));

        // Doesn't match staging without deletions
        assert!(!workflow.matches("app/api", Some("staging"), &[], "deploy", 5, false));
    }
}
