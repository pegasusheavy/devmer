//! Access control and decision making

use crate::organization::Organization;
use crate::permission::{Action, ResourceScope};
use crate::policy::{PolicyEffect, ResourcePolicy};
use crate::role::Role;
use crate::team::Team;
use crate::user::{User, UserId};
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Result of an access check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessDecision {
    /// Whether access is allowed
    pub allowed: bool,

    /// Whether approval is required
    pub requires_approval: bool,

    /// Reason for the decision
    pub reason: String,

    /// Rules that matched
    pub matched_rules: Vec<String>,

    /// Conditions that must be met
    pub conditions: Vec<String>,
}

impl AccessDecision {
    /// Create an allow decision
    pub fn allow(reason: impl Into<String>) -> Self {
        Self {
            allowed: true,
            requires_approval: false,
            reason: reason.into(),
            matched_rules: Vec::new(),
            conditions: Vec::new(),
        }
    }

    /// Create a deny decision
    pub fn deny(reason: impl Into<String>) -> Self {
        Self {
            allowed: false,
            requires_approval: false,
            reason: reason.into(),
            matched_rules: Vec::new(),
            conditions: Vec::new(),
        }
    }

    /// Create a requires-approval decision
    pub fn requires_approval(reason: impl Into<String>) -> Self {
        Self {
            allowed: true,
            requires_approval: true,
            reason: reason.into(),
            matched_rules: Vec::new(),
            conditions: Vec::new(),
        }
    }

    /// Add matched rule
    pub fn with_matched_rule(mut self, rule: impl Into<String>) -> Self {
        self.matched_rules.push(rule.into());
        self
    }

    /// Add condition
    pub fn with_condition(mut self, condition: impl Into<String>) -> Self {
        self.conditions.push(condition.into());
        self
    }
}

/// Context for access checks
#[derive(Debug, Clone)]
pub struct AccessContext {
    /// User making the request
    pub user_id: UserId,

    /// Action being performed
    pub action: String,

    /// Target stack (if applicable)
    pub stack: Option<String>,

    /// Target project (if applicable)
    pub project: Option<String>,

    /// Resource type (if applicable)
    pub resource_type: Option<String>,

    /// Environment
    pub environment: Option<String>,

    /// Additional context
    pub metadata: HashMap<String, String>,
}

impl AccessContext {
    /// Create a new access context
    pub fn new(user_id: UserId, action: impl Into<String>) -> Self {
        Self {
            user_id,
            action: action.into(),
            stack: None,
            project: None,
            resource_type: None,
            environment: None,
            metadata: HashMap::new(),
        }
    }

    pub fn with_stack(mut self, stack: impl Into<String>) -> Self {
        self.stack = Some(stack.into());
        self
    }

    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    pub fn with_resource_type(mut self, resource_type: impl Into<String>) -> Self {
        self.resource_type = Some(resource_type.into());
        self
    }

    pub fn with_environment(mut self, environment: impl Into<String>) -> Self {
        self.environment = Some(environment.into());
        self
    }
}

/// Access checker for evaluating permissions
pub struct AccessChecker {
    /// Organization
    organization: Organization,

    /// Teams in the organization
    teams: HashMap<String, Team>,

    /// Users
    users: HashMap<UserId, User>,

    /// Roles
    roles: HashMap<String, Role>,

    /// User role assignments
    user_roles: HashMap<UserId, Vec<String>>,
}

impl AccessChecker {
    /// Create a new access checker
    pub fn new(organization: Organization) -> Self {
        Self {
            organization,
            teams: HashMap::new(),
            users: HashMap::new(),
            roles: HashMap::new(),
            user_roles: HashMap::new(),
        }
    }

    /// Add a team
    pub fn add_team(&mut self, team: Team) {
        self.teams.insert(team.slug.clone(), team);
    }

    /// Add a user
    pub fn add_user(&mut self, user: User) {
        self.users.insert(user.id.clone(), user);
    }

    /// Add a role
    pub fn add_role(&mut self, role: Role) {
        self.roles.insert(role.id.0.clone(), role);
    }

    /// Assign role to user
    pub fn assign_role(&mut self, user_id: &UserId, role_id: &str) {
        self.user_roles
            .entry(user_id.clone())
            .or_default()
            .push(role_id.to_string());
    }

    /// Check access
    pub fn check_access(&self, context: &AccessContext) -> AccessDecision {
        // Check if user exists and is active
        let _user = match self.users.get(&context.user_id) {
            Some(u) if u.is_active() => u,
            Some(_) => return AccessDecision::deny("User is not active"),
            None => return AccessDecision::deny("User not found"),
        };

        // Check if user is organization owner (full access)
        if self.organization.is_owner(&context.user_id) {
            return AccessDecision::allow("Organization owner has full access");
        }

        // Check if user is organization member
        if !self.organization.is_member(&context.user_id) {
            return AccessDecision::deny("User is not a member of this organization");
        }

        // Get user's teams
        let user_teams: Vec<&Team> = self
            .teams
            .values()
            .filter(|t| t.is_member(&context.user_id))
            .collect();

        // Check team-level permissions
        for team in &user_teams {
            // Check if team owns the stack
            if let Some(ref stack) = context.stack {
                if team.owns_stack(stack) {
                    // Check team role
                    if let Some(role) = team.get_member_role(&context.user_id) {
                        if !role.can_deploy() && context.action == "deploy" {
                            continue; // This team doesn't give deploy permission
                        }
                    }

                    // Check resource policy
                    if let Some(ref policy) = team.resource_policy {
                        let effect = policy.evaluate(
                            context.stack.as_deref(),
                            context.project.as_deref(),
                            context.resource_type.as_deref(),
                            Some(&context.action),
                            context.environment.as_deref(),
                        );

                        match effect {
                            PolicyEffect::Allow => {
                                return AccessDecision::allow(format!(
                                    "Allowed by team '{}' policy",
                                    team.name
                                ));
                            }
                            PolicyEffect::RequireApproval => {
                                return AccessDecision::requires_approval(format!(
                                    "Approval required by team '{}' policy",
                                    team.name
                                ));
                            }
                            PolicyEffect::Deny => {
                                // Check other teams
                                continue;
                            }
                        }
                    } else {
                        // No policy = allow for team members
                        return AccessDecision::allow(format!(
                            "Allowed as member of team '{}'",
                            team.name
                        ));
                    }
                }
            }
        }

        // Check role-based permissions
        if let Some(role_ids) = self.user_roles.get(&context.user_id) {
            let action = string_to_action(&context.action);
            let scope = context_to_scope(context);

            for role_id in role_ids {
                if let Some(role) = self.roles.get(role_id) {
                    if let (Some(action), Some(scope)) = (&action, &scope) {
                        if role.has_permission_for(*action, scope.clone()) {
                            return AccessDecision::allow(format!(
                                "Allowed by role '{}'",
                                role.name
                            ));
                        }
                    }
                }
            }
        }

        // Default deny
        AccessDecision::deny("No matching permissions found")
    }

    /// Check if user can deploy to a stack
    pub fn can_deploy(&self, user_id: &UserId, stack: &str, environment: Option<&str>) -> AccessDecision {
        let context = AccessContext::new(user_id.clone(), "deploy")
            .with_stack(stack);

        let context = if let Some(env) = environment {
            context.with_environment(env)
        } else {
            context
        };

        self.check_access(&context)
    }

    /// Check if user can create a resource type
    pub fn can_create_resource(&self, user_id: &UserId, stack: &str, resource_type: &str) -> AccessDecision {
        let context = AccessContext::new(user_id.clone(), "deploy")
            .with_stack(stack)
            .with_resource_type(resource_type);

        self.check_access(&context)
    }

    /// Get all stacks a user can access
    pub fn accessible_stacks(&self, user_id: &UserId) -> Vec<String> {
        let mut stacks = Vec::new();

        // Organization owners can access all
        if self.organization.is_owner(user_id) {
            for team in self.teams.values() {
                stacks.extend(team.owned_stacks.iter().cloned());
            }
            return stacks;
        }

        // Get stacks from user's teams
        for team in self.teams.values() {
            if team.is_member(user_id) {
                stacks.extend(team.owned_stacks.iter().cloned());
            }
        }

        stacks
    }
}

/// Convert string action to Action enum
fn string_to_action(action: &str) -> Option<Action> {
    match action {
        "create" => Some(Action::Create),
        "read" => Some(Action::Read),
        "update" => Some(Action::Update),
        "delete" => Some(Action::Delete),
        "deploy" => Some(Action::Deploy),
        "preview" => Some(Action::Preview),
        "destroy" => Some(Action::Destroy),
        "approve" => Some(Action::Approve),
        "manage" => Some(Action::Manage),
        "admin" => Some(Action::Admin),
        _ => None,
    }
}

/// Convert context to scope
fn context_to_scope(context: &AccessContext) -> Option<ResourceScope> {
    if context.stack.is_some() {
        Some(ResourceScope::Stack)
    } else if context.project.is_some() {
        Some(ResourceScope::Project)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::organization::OrganizationId;
    use crate::team::TeamRole;

    fn setup_test_checker() -> AccessChecker {
        let mut org = Organization::new("acme", "Acme Corp");
        let org_id = org.id.clone();
        
        let mut checker = AccessChecker::new(org);

        // Create marketing team
        let mut marketing = Team::new(OrganizationId::new(), "marketing", "Marketing Team");
        marketing.add_owned_stack("marketing/*");

        let marketing_policy = ResourcePolicy::builder()
            .allow_stacks(&["marketing/*"])
            .allow_resources(&["aws:s3:*", "aws:cloudfront:*", "aws:route53:*"])
            .deny_resources(&["aws:iam:*", "aws:kms:*"])
            .require_approval_for_environments(&["production"])
            .default_allow()
            .build();

        marketing.resource_policy = Some(marketing_policy);

        // Create platform team
        let mut platform = Team::new(OrganizationId::new(), "platform", "Platform Team");
        platform.add_owned_stack("platform/*");
        platform.add_owned_stack("shared/*");

        checker.add_team(marketing);
        checker.add_team(platform);

        checker
    }

    #[test]
    fn test_team_based_access() {
        let mut checker = setup_test_checker();

        let alice = User::new("alice@example.com", "alice", "Alice");
        let alice_id = alice.id.clone();
        
        // Add Alice to organization
        checker.organization.add_member(alice_id.clone());
        checker.add_user(alice);

        // Add Alice to marketing team
        checker.teams.get_mut("marketing").unwrap().add_member(
            alice_id.clone(),
            TeamRole::Member,
            None,
        );

        // Alice can deploy to marketing
        let decision = checker.can_deploy(&alice_id, "marketing/website", None);
        assert!(decision.allowed);

        // Alice cannot deploy to platform
        let decision = checker.can_deploy(&alice_id, "platform/api", None);
        assert!(!decision.allowed);

        // Alice cannot create IAM resources in marketing
        let decision = checker.can_create_resource(
            &alice_id,
            "marketing/website",
            "aws:iam:Role",
        );
        assert!(!decision.allowed);

        // Alice can create S3 buckets in marketing
        let decision = checker.can_create_resource(
            &alice_id,
            "marketing/website",
            "aws:s3:Bucket",
        );
        assert!(decision.allowed);
    }

    #[test]
    fn test_production_approval() {
        let mut checker = setup_test_checker();

        let bob = User::new("bob@example.com", "bob", "Bob");
        let bob_id = bob.id.clone();
        
        // Add Bob to organization
        checker.organization.add_member(bob_id.clone());
        checker.add_user(bob);

        checker.teams.get_mut("marketing").unwrap().add_member(
            bob_id.clone(),
            TeamRole::Member,
            None,
        );

        // Bob deploying to production requires approval
        let decision = checker.can_deploy(
            &bob_id,
            "marketing/website",
            Some("production"),
        );
        assert!(decision.allowed);
        assert!(decision.requires_approval);
    }
}
