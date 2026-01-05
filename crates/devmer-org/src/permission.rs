//! Permission definitions

use serde::{Deserialize, Serialize};

/// Actions that can be performed
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    /// Create resources
    Create,
    /// Read/view resources
    Read,
    /// Update resources
    Update,
    /// Delete resources
    Delete,
    /// Deploy infrastructure
    Deploy,
    /// Preview changes
    Preview,
    /// Destroy infrastructure
    Destroy,
    /// Approve deployments
    Approve,
    /// Manage (create, update, delete)
    Manage,
    /// Full administrative access
    Admin,
}

impl Action {
    /// Check if this action implies another action
    pub fn implies(&self, other: &Action) -> bool {
        match self {
            Action::Admin => true, // Admin implies all
            Action::Manage => matches!(other, Action::Create | Action::Read | Action::Update | Action::Delete),
            _ => self == other,
        }
    }
}

/// Resource scopes for permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceScope {
    /// Organization-level
    Organization,
    /// Team-level
    Team,
    /// User-level
    User,
    /// Stack-level
    Stack,
    /// Project-level
    Project,
    /// Deployment-level
    Deployment,
    /// Secret-level
    Secret,
    /// Billing
    Billing,
    /// Security settings
    Security,
    /// Audit logs
    AuditLog,
    /// Specific stack pattern
    StackPattern(String),
    /// Specific project pattern
    ProjectPattern(String),
    /// Specific resource type pattern
    ResourceTypePattern(String),
}

/// A permission combining action and scope
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Permission {
    /// The action
    pub action: Action,
    /// The scope
    pub scope: ResourceScope,
    /// Optional conditions
    #[serde(default)]
    pub conditions: Vec<PermissionCondition>,
}

impl Permission {
    /// Create a new permission
    pub fn new(action: Action, scope: ResourceScope) -> Self {
        Self {
            action,
            scope,
            conditions: Vec::new(),
        }
    }

    /// Add a condition
    pub fn with_condition(mut self, condition: PermissionCondition) -> Self {
        self.conditions.push(condition);
        self
    }

    /// Create a stack permission with pattern
    pub fn stack_pattern(action: Action, pattern: impl Into<String>) -> Self {
        Self::new(action, ResourceScope::StackPattern(pattern.into()))
    }

    /// Create a resource type permission with pattern
    pub fn resource_type_pattern(action: Action, pattern: impl Into<String>) -> Self {
        Self::new(action, ResourceScope::ResourceTypePattern(pattern.into()))
    }

    /// Check if this permission matches a request
    pub fn matches(&self, action: &Action, scope: &ResourceScope) -> bool {
        // Check action (with implications)
        if !self.action.implies(action) {
            return false;
        }

        // Check scope
        match (&self.scope, scope) {
            // Exact match
            (a, b) if a == b => true,

            // Pattern matching for stacks
            (ResourceScope::StackPattern(_), ResourceScope::Stack) => true,
            (ResourceScope::StackPattern(pattern), ResourceScope::StackPattern(target)) => {
                pattern_matches(pattern, target)
            }

            // Pattern matching for projects
            (ResourceScope::ProjectPattern(_), ResourceScope::Project) => true,
            (ResourceScope::ProjectPattern(pattern), ResourceScope::ProjectPattern(target)) => {
                pattern_matches(pattern, target)
            }

            // Pattern matching for resource types
            (ResourceScope::ResourceTypePattern(pattern), ResourceScope::ResourceTypePattern(target)) => {
                pattern_matches(pattern, target)
            }

            // Organization scope implies all scopes within
            (ResourceScope::Organization, _) if self.action == Action::Admin => true,

            _ => false,
        }
    }
}

/// Conditions for permissions
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PermissionCondition {
    /// Time-based condition
    TimeWindow {
        /// Start hour (0-23)
        start_hour: u8,
        /// End hour (0-23)
        end_hour: u8,
        /// Timezone
        timezone: String,
    },
    /// IP address condition
    IpRange {
        /// CIDR ranges
        ranges: Vec<String>,
    },
    /// Environment condition
    Environment {
        /// Allowed environments
        allowed: Vec<String>,
    },
    /// Tag condition
    RequireTag {
        /// Tag key
        key: String,
        /// Tag value pattern
        value_pattern: Option<String>,
    },
    /// Approval required
    RequireApproval {
        /// Minimum approvers
        min_approvers: u32,
        /// Approver roles
        approver_roles: Vec<String>,
    },
}

/// Check if a glob pattern matches a string
fn pattern_matches(pattern: &str, target: &str) -> bool {
    // Simple glob matching
    if pattern == "*" {
        return true;
    }

    if pattern.contains('*') {
        // Handle wildcards
        let parts: Vec<&str> = pattern.split('*').collect();
        if parts.len() == 2 {
            let (prefix, suffix) = (parts[0], parts[1]);
            return target.starts_with(prefix) && target.ends_with(suffix);
        }
    }

    // Exact match
    pattern == target
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_implies() {
        assert!(Action::Admin.implies(&Action::Read));
        assert!(Action::Admin.implies(&Action::Deploy));
        assert!(Action::Manage.implies(&Action::Create));
        assert!(Action::Manage.implies(&Action::Delete));
        assert!(!Action::Read.implies(&Action::Deploy));
    }

    #[test]
    fn test_permission_matches() {
        let perm = Permission::new(Action::Deploy, ResourceScope::Stack);
        assert!(perm.matches(&Action::Deploy, &ResourceScope::Stack));
        assert!(!perm.matches(&Action::Deploy, &ResourceScope::Project));

        let admin_perm = Permission::new(Action::Admin, ResourceScope::Organization);
        assert!(admin_perm.matches(&Action::Deploy, &ResourceScope::Stack));
    }

    #[test]
    fn test_pattern_matching() {
        assert!(pattern_matches("*", "anything"));
        assert!(pattern_matches("marketing/*", "marketing/website"));
        assert!(pattern_matches("aws:s3:*", "aws:s3:Bucket"));
        assert!(!pattern_matches("marketing/*", "platform/api"));
    }

    #[test]
    fn test_stack_pattern_permission() {
        let perm = Permission::stack_pattern(Action::Deploy, "marketing/*");
        assert!(perm.matches(
            &Action::Deploy,
            &ResourceScope::StackPattern("marketing/website".to_string())
        ));
        assert!(!perm.matches(
            &Action::Deploy,
            &ResourceScope::StackPattern("platform/api".to_string())
        ));
    }
}
