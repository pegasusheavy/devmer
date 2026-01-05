//! Role definitions

use crate::permission::Permission;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use uuid::Uuid;

/// Unique identifier for a role
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RoleId(pub String);

impl RoleId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

impl std::fmt::Display for RoleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Built-in roles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuiltinRole {
    /// Organization owner - full access
    OrgOwner,
    /// Organization admin - manage org settings and members
    OrgAdmin,
    /// Member - deploy within assigned teams
    Member,
    /// Viewer - read-only access
    Viewer,
    /// Billing admin - manage billing only
    BillingAdmin,
    /// Security admin - manage security settings
    SecurityAdmin,
    /// Deployment approver - can approve deployments
    DeploymentApprover,
}

impl BuiltinRole {
    /// Get role ID
    pub fn id(&self) -> RoleId {
        RoleId::new(match self {
            BuiltinRole::OrgOwner => "org:owner",
            BuiltinRole::OrgAdmin => "org:admin",
            BuiltinRole::Member => "org:member",
            BuiltinRole::Viewer => "org:viewer",
            BuiltinRole::BillingAdmin => "org:billing_admin",
            BuiltinRole::SecurityAdmin => "org:security_admin",
            BuiltinRole::DeploymentApprover => "org:deployment_approver",
        })
    }

    /// Get default permissions for this role
    pub fn default_permissions(&self) -> HashSet<Permission> {
        use crate::permission::{Action, ResourceScope};

        let mut perms = HashSet::new();

        match self {
            BuiltinRole::OrgOwner => {
                // Full access to everything
                perms.insert(Permission::new(Action::Admin, ResourceScope::Organization));
            }

            BuiltinRole::OrgAdmin => {
                perms.insert(Permission::new(Action::Read, ResourceScope::Organization));
                perms.insert(Permission::new(Action::Update, ResourceScope::Organization));
                perms.insert(Permission::new(Action::Create, ResourceScope::Team));
                perms.insert(Permission::new(Action::Read, ResourceScope::Team));
                perms.insert(Permission::new(Action::Update, ResourceScope::Team));
                perms.insert(Permission::new(Action::Delete, ResourceScope::Team));
                perms.insert(Permission::new(Action::Create, ResourceScope::User));
                perms.insert(Permission::new(Action::Read, ResourceScope::User));
                perms.insert(Permission::new(Action::Update, ResourceScope::User));
            }

            BuiltinRole::Member => {
                perms.insert(Permission::new(Action::Read, ResourceScope::Organization));
                perms.insert(Permission::new(Action::Read, ResourceScope::Team));
                perms.insert(Permission::new(Action::Read, ResourceScope::Stack));
                perms.insert(Permission::new(Action::Deploy, ResourceScope::Stack));
                perms.insert(Permission::new(Action::Preview, ResourceScope::Stack));
            }

            BuiltinRole::Viewer => {
                perms.insert(Permission::new(Action::Read, ResourceScope::Organization));
                perms.insert(Permission::new(Action::Read, ResourceScope::Team));
                perms.insert(Permission::new(Action::Read, ResourceScope::Stack));
                perms.insert(Permission::new(Action::Preview, ResourceScope::Stack));
            }

            BuiltinRole::BillingAdmin => {
                perms.insert(Permission::new(Action::Read, ResourceScope::Organization));
                perms.insert(Permission::new(Action::Admin, ResourceScope::Billing));
            }

            BuiltinRole::SecurityAdmin => {
                perms.insert(Permission::new(Action::Read, ResourceScope::Organization));
                perms.insert(Permission::new(Action::Admin, ResourceScope::Security));
                perms.insert(Permission::new(Action::Read, ResourceScope::AuditLog));
            }

            BuiltinRole::DeploymentApprover => {
                perms.insert(Permission::new(Action::Read, ResourceScope::Stack));
                perms.insert(Permission::new(Action::Approve, ResourceScope::Deployment));
            }
        }

        perms
    }
}

/// A role in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Role {
    /// Unique identifier
    pub id: RoleId,

    /// Display name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Permissions granted by this role
    pub permissions: HashSet<Permission>,

    /// Whether this is a built-in role
    pub builtin: bool,

    /// Organization this role belongs to (None for global roles)
    pub organization_id: Option<Uuid>,

    /// When created
    pub created_at: DateTime<Utc>,

    /// When last updated
    pub updated_at: DateTime<Utc>,
}

impl Role {
    /// Create a new custom role
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: RoleId::new(id),
            name: name.into(),
            description: None,
            permissions: HashSet::new(),
            builtin: false,
            organization_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Create from built-in role
    pub fn from_builtin(builtin: BuiltinRole) -> Self {
        let now = Utc::now();
        Self {
            id: builtin.id(),
            name: format!("{:?}", builtin),
            description: None,
            permissions: builtin.default_permissions(),
            builtin: true,
            organization_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Add permission
    pub fn add_permission(&mut self, permission: Permission) {
        self.permissions.insert(permission);
        self.updated_at = Utc::now();
    }

    /// Remove permission
    pub fn remove_permission(&mut self, permission: &Permission) -> bool {
        let removed = self.permissions.remove(permission);
        if removed {
            self.updated_at = Utc::now();
        }
        removed
    }

    /// Check if role has permission
    pub fn has_permission(&self, permission: &Permission) -> bool {
        self.permissions.contains(permission)
    }

    /// Check if role has any permission matching action and scope
    pub fn has_permission_for(
        &self,
        action: crate::permission::Action,
        scope: crate::permission::ResourceScope,
    ) -> bool {
        use crate::permission::Action;

        self.permissions.iter().any(|p| {
            // Admin action grants all actions on that scope
            if p.action == Action::Admin && p.scope == scope {
                return true;
            }
            // Exact match
            p.action == action && p.scope == scope
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permission::{Action, ResourceScope};

    #[test]
    fn test_builtin_role_permissions() {
        let owner = Role::from_builtin(BuiltinRole::OrgOwner);
        assert!(owner.has_permission_for(Action::Admin, ResourceScope::Organization));

        let member = Role::from_builtin(BuiltinRole::Member);
        assert!(member.has_permission_for(Action::Deploy, ResourceScope::Stack));
        assert!(!member.has_permission_for(Action::Admin, ResourceScope::Organization));

        let viewer = Role::from_builtin(BuiltinRole::Viewer);
        assert!(viewer.has_permission_for(Action::Read, ResourceScope::Stack));
        assert!(!viewer.has_permission_for(Action::Deploy, ResourceScope::Stack));
    }

    #[test]
    fn test_custom_role() {
        let mut role = Role::new("custom:deployer", "Custom Deployer")
            .with_description("Can deploy to staging only");

        role.add_permission(Permission::new(Action::Deploy, ResourceScope::Stack));
        role.add_permission(Permission::new(Action::Preview, ResourceScope::Stack));

        assert!(role.has_permission_for(Action::Deploy, ResourceScope::Stack));
        assert!(!role.has_permission_for(Action::Admin, ResourceScope::Organization));
    }
}
