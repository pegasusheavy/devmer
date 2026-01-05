//! Audit event types and definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::IpAddr;
use uuid::Uuid;

/// A single audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub id: Uuid,

    /// Event timestamp
    pub timestamp: DateTime<Utc>,

    /// Type of event
    pub event_type: EventType,

    /// Event severity
    pub severity: EventSeverity,

    /// Outcome of the event
    pub outcome: EventOutcome,

    /// Who performed the action
    pub actor: Actor,

    /// What was acted upon
    pub resource: Option<Resource>,

    /// Stack name
    pub stack: Option<String>,

    /// Project name
    pub project: Option<String>,

    /// Organization ID
    pub organization_id: Option<String>,

    /// Human-readable description
    pub description: String,

    /// Detailed message
    pub message: Option<String>,

    /// Additional metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,

    /// Request ID for correlation
    pub request_id: Option<String>,

    /// Session ID
    pub session_id: Option<String>,

    /// Duration in milliseconds (for operations)
    pub duration_ms: Option<u64>,

    /// Previous event hash (for chain integrity)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_hash: Option<String>,

    /// This event's hash
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
}

impl AuditEvent {
    /// Create a new audit event
    pub fn new(event_type: EventType, actor: Actor, description: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            event_type,
            severity: EventSeverity::Info,
            outcome: EventOutcome::Success,
            actor,
            resource: None,
            stack: None,
            project: None,
            organization_id: None,
            description: description.into(),
            message: None,
            metadata: HashMap::new(),
            request_id: None,
            session_id: None,
            duration_ms: None,
            previous_hash: None,
            hash: None,
        }
    }

    /// Set severity
    pub fn with_severity(mut self, severity: EventSeverity) -> Self {
        self.severity = severity;
        self
    }

    /// Set outcome
    pub fn with_outcome(mut self, outcome: EventOutcome) -> Self {
        self.outcome = outcome;
        self
    }

    /// Set resource
    pub fn with_resource(mut self, resource: Resource) -> Self {
        self.resource = Some(resource);
        self
    }

    /// Set stack
    pub fn with_stack(mut self, stack: impl Into<String>) -> Self {
        self.stack = Some(stack.into());
        self
    }

    /// Set project
    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// Set organization
    pub fn with_organization(mut self, org_id: impl Into<String>) -> Self {
        self.organization_id = Some(org_id.into());
        self
    }

    /// Set message
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.metadata.insert(key.into(), v);
        }
        self
    }

    /// Set request ID
    pub fn with_request_id(mut self, request_id: impl Into<String>) -> Self {
        self.request_id = Some(request_id.into());
        self
    }

    /// Set session ID
    pub fn with_session_id(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }

    /// Set duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = Some(duration_ms);
        self
    }

    // =========================================================================
    // Convenience constructors for common events
    // =========================================================================

    /// Deployment started event
    pub fn deployment_started(stack: &str, user: &str) -> Self {
        Self::new(
            EventType::DeploymentStarted,
            Actor::user(user),
            format!("Deployment started for stack '{}'", stack),
        )
        .with_stack(stack)
        .with_severity(EventSeverity::Info)
    }

    /// Deployment completed event
    pub fn deployment_completed(stack: &str, user: &str, duration_ms: u64) -> Self {
        Self::new(
            EventType::DeploymentCompleted,
            Actor::user(user),
            format!("Deployment completed for stack '{}'", stack),
        )
        .with_stack(stack)
        .with_duration(duration_ms)
        .with_severity(EventSeverity::Info)
    }

    /// Deployment failed event
    pub fn deployment_failed(stack: &str, user: &str, error: &str) -> Self {
        Self::new(
            EventType::DeploymentFailed,
            Actor::user(user),
            format!("Deployment failed for stack '{}'", stack),
        )
        .with_stack(stack)
        .with_message(error)
        .with_outcome(EventOutcome::Failure)
        .with_severity(EventSeverity::Error)
    }

    /// Resource created event
    pub fn resource_created(resource: Resource, user: &str) -> Self {
        let desc = format!("Created resource '{}' ({})", resource.name, resource.resource_type);
        Self::new(EventType::ResourceCreated, Actor::user(user), desc)
            .with_resource(resource)
            .with_severity(EventSeverity::Info)
    }

    /// Resource updated event
    pub fn resource_updated(resource: Resource, user: &str) -> Self {
        let desc = format!("Updated resource '{}' ({})", resource.name, resource.resource_type);
        Self::new(EventType::ResourceUpdated, Actor::user(user), desc)
            .with_resource(resource)
            .with_severity(EventSeverity::Info)
    }

    /// Resource deleted event
    pub fn resource_deleted(resource: Resource, user: &str) -> Self {
        let desc = format!("Deleted resource '{}' ({})", resource.name, resource.resource_type);
        Self::new(EventType::ResourceDeleted, Actor::user(user), desc)
            .with_resource(resource)
            .with_severity(EventSeverity::Warning)
    }

    /// Secret accessed event
    pub fn secret_accessed(secret_name: &str, user: &str) -> Self {
        Self::new(
            EventType::SecretAccessed,
            Actor::user(user),
            format!("Secret '{}' was accessed", secret_name),
        )
        .with_metadata("secret_name", secret_name)
        .with_severity(EventSeverity::Info)
    }

    /// Secret modified event
    pub fn secret_modified(secret_name: &str, user: &str) -> Self {
        Self::new(
            EventType::SecretModified,
            Actor::user(user),
            format!("Secret '{}' was modified", secret_name),
        )
        .with_metadata("secret_name", secret_name)
        .with_severity(EventSeverity::Warning)
    }

    /// Configuration changed event
    pub fn config_changed(key: &str, user: &str) -> Self {
        Self::new(
            EventType::ConfigChanged,
            Actor::user(user),
            format!("Configuration '{}' was changed", key),
        )
        .with_metadata("config_key", key)
        .with_severity(EventSeverity::Info)
    }

    /// Authentication event
    pub fn authentication(user: &str, success: bool, method: &str) -> Self {
        let event_type = if success {
            EventType::AuthenticationSuccess
        } else {
            EventType::AuthenticationFailure
        };
        let outcome = if success {
            EventOutcome::Success
        } else {
            EventOutcome::Failure
        };
        let severity = if success {
            EventSeverity::Info
        } else {
            EventSeverity::Warning
        };

        Self::new(
            event_type,
            Actor::user(user),
            format!(
                "Authentication {} for user '{}' via {}",
                if success { "succeeded" } else { "failed" },
                user,
                method
            ),
        )
        .with_outcome(outcome)
        .with_severity(severity)
        .with_metadata("auth_method", method)
    }

    /// Authorization event
    pub fn authorization(user: &str, action: &str, resource: &str, allowed: bool) -> Self {
        let event_type = if allowed {
            EventType::AuthorizationGranted
        } else {
            EventType::AuthorizationDenied
        };
        let outcome = if allowed {
            EventOutcome::Success
        } else {
            EventOutcome::Denied
        };
        let severity = if allowed {
            EventSeverity::Info
        } else {
            EventSeverity::Warning
        };

        Self::new(
            event_type,
            Actor::user(user),
            format!(
                "Authorization {} for action '{}' on '{}'",
                if allowed { "granted" } else { "denied" },
                action,
                resource
            ),
        )
        .with_outcome(outcome)
        .with_severity(severity)
        .with_metadata("action", action)
        .with_metadata("target_resource", resource)
    }

    /// Stack created event
    pub fn stack_created(stack: &str, user: &str) -> Self {
        Self::new(
            EventType::StackCreated,
            Actor::user(user),
            format!("Stack '{}' was created", stack),
        )
        .with_stack(stack)
        .with_severity(EventSeverity::Info)
    }

    /// Stack deleted event
    pub fn stack_deleted(stack: &str, user: &str) -> Self {
        Self::new(
            EventType::StackDeleted,
            Actor::user(user),
            format!("Stack '{}' was deleted", stack),
        )
        .with_stack(stack)
        .with_severity(EventSeverity::Warning)
    }

    /// State exported event
    pub fn state_exported(stack: &str, user: &str) -> Self {
        Self::new(
            EventType::StateExported,
            Actor::user(user),
            format!("State exported for stack '{}'", stack),
        )
        .with_stack(stack)
        .with_severity(EventSeverity::Info)
    }

    /// State imported event
    pub fn state_imported(stack: &str, user: &str) -> Self {
        Self::new(
            EventType::StateImported,
            Actor::user(user),
            format!("State imported for stack '{}'", stack),
        )
        .with_stack(stack)
        .with_severity(EventSeverity::Warning)
    }

    /// Policy violation event
    pub fn policy_violation(policy: &str, user: &str, details: &str) -> Self {
        Self::new(
            EventType::PolicyViolation,
            Actor::user(user),
            format!("Policy '{}' was violated", policy),
        )
        .with_message(details)
        .with_metadata("policy_name", policy)
        .with_outcome(EventOutcome::Denied)
        .with_severity(EventSeverity::Critical)
    }

    /// Approval requested event
    pub fn approval_requested(workflow: &str, user: &str, reason: &str) -> Self {
        Self::new(
            EventType::ApprovalRequested,
            Actor::user(user),
            format!("Approval requested for workflow '{}'", workflow),
        )
        .with_message(reason)
        .with_metadata("workflow_id", workflow)
        .with_severity(EventSeverity::Info)
    }

    /// Approval granted event
    pub fn approval_granted(workflow: &str, approver: &str) -> Self {
        Self::new(
            EventType::ApprovalGranted,
            Actor::user(approver),
            format!("Approval granted for workflow '{}'", workflow),
        )
        .with_metadata("workflow_id", workflow)
        .with_severity(EventSeverity::Info)
    }

    /// Approval denied event
    pub fn approval_denied(workflow: &str, approver: &str, reason: &str) -> Self {
        Self::new(
            EventType::ApprovalDenied,
            Actor::user(approver),
            format!("Approval denied for workflow '{}'", workflow),
        )
        .with_message(reason)
        .with_metadata("workflow_id", workflow)
        .with_outcome(EventOutcome::Denied)
        .with_severity(EventSeverity::Warning)
    }

    /// System event
    pub fn system_event(event_type: EventType, description: impl Into<String>) -> Self {
        Self::new(event_type, Actor::system(), description)
    }
}

/// Type of audit event
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    // Deployment events
    DeploymentStarted,
    DeploymentCompleted,
    DeploymentFailed,
    DeploymentCancelled,
    PreviewExecuted,
    RefreshExecuted,
    DestroyStarted,
    DestroyCompleted,
    DestroyFailed,

    // Resource events
    ResourceCreated,
    ResourceUpdated,
    ResourceDeleted,
    ResourceReplaced,
    ResourceImported,
    ResourceDriftDetected,

    // State events
    StateRead,
    StateWritten,
    StateLocked,
    StateUnlocked,
    StateExported,
    StateImported,
    StateBackupCreated,

    // Secret events
    SecretAccessed,
    SecretModified,
    SecretDeleted,
    SecretRotated,
    EncryptionKeyRotated,

    // Configuration events
    ConfigChanged,
    ConfigDeleted,
    StackCreated,
    StackDeleted,
    StackSelected,

    // Authentication events
    AuthenticationSuccess,
    AuthenticationFailure,
    SessionStarted,
    SessionEnded,
    TokenGenerated,
    TokenRevoked,

    // Authorization events
    AuthorizationGranted,
    AuthorizationDenied,
    RoleAssigned,
    RoleRevoked,
    PermissionGranted,
    PermissionRevoked,

    // Policy events
    PolicyCreated,
    PolicyUpdated,
    PolicyDeleted,
    PolicyViolation,
    PolicyEvaluated,

    // Approval events
    ApprovalRequested,
    ApprovalGranted,
    ApprovalDenied,
    ApprovalExpired,

    // Organization events
    OrganizationCreated,
    OrganizationUpdated,
    OrganizationDeleted,
    TeamCreated,
    TeamUpdated,
    TeamDeleted,
    MemberAdded,
    MemberRemoved,

    // System events
    SystemStarted,
    SystemStopped,
    BackupCreated,
    BackupRestored,
    MaintenanceStarted,
    MaintenanceCompleted,
    AuditLogRotated,
    AuditLogExported,

    // Compliance events
    ComplianceCheckStarted,
    ComplianceCheckCompleted,
    ComplianceViolation,
    ReportGenerated,

    // Custom event
    Custom,
}

impl EventType {
    /// Get the category for this event type
    pub fn category(&self) -> &'static str {
        match self {
            Self::DeploymentStarted
            | Self::DeploymentCompleted
            | Self::DeploymentFailed
            | Self::DeploymentCancelled
            | Self::PreviewExecuted
            | Self::RefreshExecuted
            | Self::DestroyStarted
            | Self::DestroyCompleted
            | Self::DestroyFailed => "deployment",

            Self::ResourceCreated
            | Self::ResourceUpdated
            | Self::ResourceDeleted
            | Self::ResourceReplaced
            | Self::ResourceImported
            | Self::ResourceDriftDetected => "resource",

            Self::StateRead
            | Self::StateWritten
            | Self::StateLocked
            | Self::StateUnlocked
            | Self::StateExported
            | Self::StateImported
            | Self::StateBackupCreated => "state",

            Self::SecretAccessed
            | Self::SecretModified
            | Self::SecretDeleted
            | Self::SecretRotated
            | Self::EncryptionKeyRotated => "secret",

            Self::ConfigChanged
            | Self::ConfigDeleted
            | Self::StackCreated
            | Self::StackDeleted
            | Self::StackSelected => "configuration",

            Self::AuthenticationSuccess
            | Self::AuthenticationFailure
            | Self::SessionStarted
            | Self::SessionEnded
            | Self::TokenGenerated
            | Self::TokenRevoked => "authentication",

            Self::AuthorizationGranted
            | Self::AuthorizationDenied
            | Self::RoleAssigned
            | Self::RoleRevoked
            | Self::PermissionGranted
            | Self::PermissionRevoked => "authorization",

            Self::PolicyCreated
            | Self::PolicyUpdated
            | Self::PolicyDeleted
            | Self::PolicyViolation
            | Self::PolicyEvaluated => "policy",

            Self::ApprovalRequested
            | Self::ApprovalGranted
            | Self::ApprovalDenied
            | Self::ApprovalExpired => "approval",

            Self::OrganizationCreated
            | Self::OrganizationUpdated
            | Self::OrganizationDeleted
            | Self::TeamCreated
            | Self::TeamUpdated
            | Self::TeamDeleted
            | Self::MemberAdded
            | Self::MemberRemoved => "organization",

            Self::SystemStarted
            | Self::SystemStopped
            | Self::BackupCreated
            | Self::BackupRestored
            | Self::MaintenanceStarted
            | Self::MaintenanceCompleted
            | Self::AuditLogRotated
            | Self::AuditLogExported => "system",

            Self::ComplianceCheckStarted
            | Self::ComplianceCheckCompleted
            | Self::ComplianceViolation
            | Self::ReportGenerated => "compliance",

            Self::Custom => "custom",
        }
    }

    /// Check if this is a security-relevant event
    pub fn is_security_event(&self) -> bool {
        matches!(
            self,
            Self::AuthenticationSuccess
                | Self::AuthenticationFailure
                | Self::AuthorizationGranted
                | Self::AuthorizationDenied
                | Self::SecretAccessed
                | Self::SecretModified
                | Self::SecretRotated
                | Self::EncryptionKeyRotated
                | Self::PolicyViolation
                | Self::TokenGenerated
                | Self::TokenRevoked
                | Self::RoleAssigned
                | Self::RoleRevoked
        )
    }

    /// Check if this is a compliance-relevant event
    pub fn is_compliance_event(&self) -> bool {
        matches!(
            self,
            Self::ComplianceCheckStarted
                | Self::ComplianceCheckCompleted
                | Self::ComplianceViolation
                | Self::PolicyViolation
                | Self::PolicyEvaluated
                | Self::ApprovalGranted
                | Self::ApprovalDenied
                | Self::ReportGenerated
        )
    }
}

/// Event severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventSeverity {
    /// Debug/trace level
    Debug,
    /// Informational
    Info,
    /// Warning - needs attention
    Warning,
    /// Error - operation failed
    Error,
    /// Critical - immediate attention required
    Critical,
}

impl EventSeverity {
    /// Get numeric value for comparison
    pub fn level(&self) -> u8 {
        match self {
            Self::Debug => 0,
            Self::Info => 1,
            Self::Warning => 2,
            Self::Error => 3,
            Self::Critical => 4,
        }
    }
}

impl std::fmt::Display for EventSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Debug => write!(f, "DEBUG"),
            Self::Info => write!(f, "INFO"),
            Self::Warning => write!(f, "WARNING"),
            Self::Error => write!(f, "ERROR"),
            Self::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Outcome of an event/operation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EventOutcome {
    /// Operation succeeded
    Success,
    /// Operation failed
    Failure,
    /// Operation was denied (authorization)
    Denied,
    /// Operation timed out
    Timeout,
    /// Operation was cancelled
    Cancelled,
    /// Unknown outcome
    Unknown,
}

/// Actor who performed the action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Actor {
    /// Actor type
    pub actor_type: ActorType,

    /// Actor ID (user ID, service name, etc.)
    pub id: String,

    /// Display name
    pub name: Option<String>,

    /// Email (for users)
    pub email: Option<String>,

    /// IP address
    pub ip_address: Option<IpAddr>,

    /// User agent (for web requests)
    pub user_agent: Option<String>,

    /// Additional attributes
    #[serde(default)]
    pub attributes: HashMap<String, String>,
}

impl Actor {
    /// Create a user actor
    pub fn user(id: impl Into<String>) -> Self {
        Self {
            actor_type: ActorType::User,
            id: id.into(),
            name: None,
            email: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        }
    }

    /// Create a service actor
    pub fn service(name: impl Into<String>) -> Self {
        Self {
            actor_type: ActorType::Service,
            id: name.into(),
            name: None,
            email: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        }
    }

    /// Create a system actor
    pub fn system() -> Self {
        Self {
            actor_type: ActorType::System,
            id: "system".to_string(),
            name: Some("System".to_string()),
            email: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        }
    }

    /// Create an anonymous actor
    pub fn anonymous() -> Self {
        Self {
            actor_type: ActorType::Anonymous,
            id: "anonymous".to_string(),
            name: None,
            email: None,
            ip_address: None,
            user_agent: None,
            attributes: HashMap::new(),
        }
    }

    /// Set display name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set email
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.email = Some(email.into());
        self
    }

    /// Set IP address
    pub fn with_ip(mut self, ip: IpAddr) -> Self {
        self.ip_address = Some(ip);
        self
    }

    /// Set user agent
    pub fn with_user_agent(mut self, user_agent: impl Into<String>) -> Self {
        self.user_agent = Some(user_agent.into());
        self
    }

    /// Add attribute
    pub fn with_attribute(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.attributes.insert(key.into(), value.into());
        self
    }
}

/// Type of actor
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ActorType {
    /// Human user
    User,
    /// Automated service/bot
    Service,
    /// System process
    System,
    /// Anonymous/unauthenticated
    Anonymous,
    /// API key
    ApiKey,
}

/// Resource that was acted upon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    /// Resource type (e.g., "aws:s3:Bucket")
    pub resource_type: String,

    /// Resource name
    pub name: String,

    /// Resource URN
    pub urn: Option<String>,

    /// Provider
    pub provider: Option<String>,

    /// Region/location
    pub region: Option<String>,

    /// Additional identifiers
    #[serde(default)]
    pub identifiers: HashMap<String, String>,
}

impl Resource {
    /// Create a new resource
    pub fn new(resource_type: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            resource_type: resource_type.into(),
            name: name.into(),
            urn: None,
            provider: None,
            region: None,
            identifiers: HashMap::new(),
        }
    }

    /// Set URN
    pub fn with_urn(mut self, urn: impl Into<String>) -> Self {
        self.urn = Some(urn.into());
        self
    }

    /// Set provider
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    /// Set region
    pub fn with_region(mut self, region: impl Into<String>) -> Self {
        self.region = Some(region.into());
        self
    }

    /// Add identifier
    pub fn with_identifier(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.identifiers.insert(key.into(), value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_creation() {
        let event = AuditEvent::deployment_started("dev", "user@example.com")
            .with_project("my-project")
            .with_organization("org-123");

        assert_eq!(event.event_type, EventType::DeploymentStarted);
        assert_eq!(event.actor.id, "user@example.com");
        assert_eq!(event.stack, Some("dev".to_string()));
        assert_eq!(event.project, Some("my-project".to_string()));
        assert_eq!(event.severity, EventSeverity::Info);
    }

    #[test]
    fn test_event_categories() {
        assert_eq!(EventType::DeploymentStarted.category(), "deployment");
        assert_eq!(EventType::ResourceCreated.category(), "resource");
        assert_eq!(EventType::AuthenticationSuccess.category(), "authentication");
        assert_eq!(EventType::PolicyViolation.category(), "policy");
    }

    #[test]
    fn test_security_events() {
        assert!(EventType::AuthenticationFailure.is_security_event());
        assert!(EventType::SecretAccessed.is_security_event());
        assert!(!EventType::DeploymentStarted.is_security_event());
    }

    #[test]
    fn test_actor_types() {
        let user = Actor::user("user-123").with_email("user@example.com");
        assert_eq!(user.actor_type, ActorType::User);
        assert_eq!(user.email, Some("user@example.com".to_string()));

        let service = Actor::service("devmer-cli");
        assert_eq!(service.actor_type, ActorType::Service);

        let system = Actor::system();
        assert_eq!(system.actor_type, ActorType::System);
    }
}
