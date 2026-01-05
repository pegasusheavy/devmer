//! Team management

use crate::organization::OrganizationId;
use crate::policy::ResourcePolicy;
use crate::role::RoleId;
use crate::user::UserId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Unique identifier for a team
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TeamId(pub Uuid);

impl TeamId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn parse(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }
}

impl Default for TeamId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TeamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Team membership with role
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamMembership {
    /// User ID
    pub user_id: UserId,

    /// Role within the team
    pub role: TeamRole,

    /// When joined
    pub joined_at: DateTime<Utc>,

    /// Who added them
    pub added_by: Option<UserId>,
}

/// Role within a team
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TeamRole {
    /// Team owner - can manage team
    Owner,
    /// Team maintainer - can deploy and manage resources
    Maintainer,
    /// Team member - can deploy within policy
    Member,
    /// Viewer - read-only access
    Viewer,
}

impl Default for TeamRole {
    fn default() -> Self {
        Self::Member
    }
}

impl TeamRole {
    /// Check if this role can manage team members
    pub fn can_manage_members(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Maintainer)
    }

    /// Check if this role can deploy
    pub fn can_deploy(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Maintainer | TeamRole::Member)
    }

    /// Check if this role can approve deployments
    pub fn can_approve(&self) -> bool {
        matches!(self, TeamRole::Owner | TeamRole::Maintainer)
    }
}

/// A team within an organization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Team {
    /// Unique identifier
    pub id: TeamId,

    /// Organization this team belongs to
    pub organization_id: OrganizationId,

    /// URL-safe slug
    pub slug: String,

    /// Display name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Team members with their roles
    pub members: HashMap<UserId, TeamMembership>,

    /// Resource policy - what this team can deploy
    pub resource_policy: Option<ResourcePolicy>,

    /// Stacks owned by this team
    pub owned_stacks: HashSet<String>,

    /// Projects owned by this team
    pub owned_projects: HashSet<String>,

    /// Parent team (for nested teams)
    pub parent_team: Option<TeamId>,

    /// Child teams
    pub child_teams: HashSet<TeamId>,

    /// Custom metadata
    pub metadata: HashMap<String, String>,

    /// When created
    pub created_at: DateTime<Utc>,

    /// When last updated
    pub updated_at: DateTime<Utc>,

    /// Whether team is active
    pub active: bool,
}

impl Team {
    /// Create a new team
    pub fn new(
        organization_id: OrganizationId,
        slug: impl Into<String>,
        name: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: TeamId::new(),
            organization_id,
            slug: slug.into(),
            name: name.into(),
            description: None,
            members: HashMap::new(),
            resource_policy: None,
            owned_stacks: HashSet::new(),
            owned_projects: HashSet::new(),
            parent_team: None,
            child_teams: HashSet::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            active: true,
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set resource policy
    pub fn with_resource_policy(mut self, policy: ResourcePolicy) -> Self {
        self.resource_policy = Some(policy);
        self
    }

    /// Add a member
    pub fn add_member(&mut self, user_id: UserId, role: TeamRole, added_by: Option<UserId>) {
        let membership = TeamMembership {
            user_id: user_id.clone(),
            role,
            joined_at: Utc::now(),
            added_by,
        };
        self.members.insert(user_id, membership);
        self.updated_at = Utc::now();
    }

    /// Remove a member
    pub fn remove_member(&mut self, user_id: &UserId) -> Option<TeamMembership> {
        let removed = self.members.remove(user_id);
        if removed.is_some() {
            self.updated_at = Utc::now();
        }
        removed
    }

    /// Update member role
    pub fn update_member_role(&mut self, user_id: &UserId, role: TeamRole) -> bool {
        if let Some(membership) = self.members.get_mut(user_id) {
            membership.role = role;
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Get member role
    pub fn get_member_role(&self, user_id: &UserId) -> Option<TeamRole> {
        self.members.get(user_id).map(|m| m.role)
    }

    /// Check if user is a member
    pub fn is_member(&self, user_id: &UserId) -> bool {
        self.members.contains_key(user_id)
    }

    /// Check if user is an owner
    pub fn is_owner(&self, user_id: &UserId) -> bool {
        self.members
            .get(user_id)
            .map(|m| m.role == TeamRole::Owner)
            .unwrap_or(false)
    }

    /// Get all owners
    pub fn owners(&self) -> Vec<&UserId> {
        self.members
            .iter()
            .filter(|(_, m)| m.role == TeamRole::Owner)
            .map(|(id, _)| id)
            .collect()
    }

    /// Add owned stack
    pub fn add_owned_stack(&mut self, stack_path: impl Into<String>) {
        self.owned_stacks.insert(stack_path.into());
        self.updated_at = Utc::now();
    }

    /// Remove owned stack
    pub fn remove_owned_stack(&mut self, stack_path: &str) -> bool {
        let removed = self.owned_stacks.remove(stack_path);
        if removed {
            self.updated_at = Utc::now();
        }
        removed
    }

    /// Add owned project
    pub fn add_owned_project(&mut self, project: impl Into<String>) {
        self.owned_projects.insert(project.into());
        self.updated_at = Utc::now();
    }

    /// Check if team owns a stack
    pub fn owns_stack(&self, stack_path: &str) -> bool {
        // Check exact match
        if self.owned_stacks.contains(stack_path) {
            return true;
        }

        // Check wildcard patterns
        for pattern in &self.owned_stacks {
            if pattern.ends_with("/*") {
                let prefix = &pattern[..pattern.len() - 2];
                if stack_path.starts_with(prefix) {
                    return true;
                }
            }
        }

        false
    }

    /// Member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_creation() {
        let org_id = OrganizationId::new();
        let team = Team::new(org_id, "marketing", "Marketing Team")
            .with_description("The marketing team");

        assert_eq!(team.slug, "marketing");
        assert_eq!(team.name, "Marketing Team");
        assert!(team.active);
    }

    #[test]
    fn test_team_members() {
        let org_id = OrganizationId::new();
        let mut team = Team::new(org_id, "test", "Test Team");

        let user1 = UserId::new();
        let user2 = UserId::new();

        team.add_member(user1.clone(), TeamRole::Owner, None);
        team.add_member(user2.clone(), TeamRole::Member, Some(user1.clone()));

        assert!(team.is_owner(&user1));
        assert!(team.is_member(&user2));
        assert!(!team.is_owner(&user2));

        assert_eq!(team.member_count(), 2);
    }

    #[test]
    fn test_stack_ownership() {
        let org_id = OrganizationId::new();
        let mut team = Team::new(org_id, "marketing", "Marketing");

        team.add_owned_stack("marketing/website");
        team.add_owned_stack("marketing/campaigns/*");

        assert!(team.owns_stack("marketing/website"));
        assert!(team.owns_stack("marketing/campaigns/summer"));
        assert!(team.owns_stack("marketing/campaigns/winter"));
        assert!(!team.owns_stack("platform/api"));
    }
}
