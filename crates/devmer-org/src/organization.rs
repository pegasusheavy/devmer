//! Organization management

use crate::team::TeamId;
use crate::user::UserId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

/// Unique identifier for an organization
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OrganizationId(pub Uuid);

impl OrganizationId {
    /// Create a new random organization ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }
}

impl Default for OrganizationId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for OrganizationId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Organization settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrganizationSettings {
    /// Allow members to create teams
    pub allow_team_creation: bool,

    /// Allow members to invite users
    pub allow_member_invites: bool,

    /// Require approval for production deployments
    pub require_production_approval: bool,

    /// Default role for new members
    pub default_member_role: String,

    /// Enable audit logging
    pub audit_logging_enabled: bool,

    /// Allowed authentication methods
    pub allowed_auth_methods: Vec<String>,

    /// Session timeout in hours
    pub session_timeout_hours: u32,

    /// IP allowlist (empty = allow all)
    pub ip_allowlist: Vec<String>,

    /// Custom domain for organization
    pub custom_domain: Option<String>,
}

impl Default for OrganizationSettings {
    fn default() -> Self {
        Self {
            allow_team_creation: false,
            allow_member_invites: false,
            require_production_approval: true,
            default_member_role: "member".to_string(),
            audit_logging_enabled: true,
            allowed_auth_methods: vec!["password".to_string(), "sso".to_string()],
            session_timeout_hours: 24,
            ip_allowlist: vec![],
            custom_domain: None,
        }
    }
}

/// Billing tier for organizations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BillingTier {
    /// Free tier
    Free,
    /// Team tier
    Team,
    /// Business tier
    Business,
    /// Enterprise tier
    Enterprise,
}

impl Default for BillingTier {
    fn default() -> Self {
        Self::Free
    }
}

/// An organization in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Organization {
    /// Unique identifier
    pub id: OrganizationId,

    /// URL-safe slug (e.g., "acme-corp")
    pub slug: String,

    /// Display name
    pub name: String,

    /// Description
    pub description: Option<String>,

    /// Teams in this organization
    pub teams: HashSet<TeamId>,

    /// Direct members (not in teams)
    pub members: HashSet<UserId>,

    /// Organization owners (super admins)
    pub owners: HashSet<UserId>,

    /// Organization settings
    pub settings: OrganizationSettings,

    /// Billing tier
    pub billing_tier: BillingTier,

    /// Custom metadata
    pub metadata: HashMap<String, String>,

    /// When created
    pub created_at: DateTime<Utc>,

    /// When last updated
    pub updated_at: DateTime<Utc>,

    /// Whether organization is active
    pub active: bool,
}

impl Organization {
    /// Create a new organization
    pub fn new(slug: impl Into<String>, name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: OrganizationId::new(),
            slug: slug.into(),
            name: name.into(),
            description: None,
            teams: HashSet::new(),
            members: HashSet::new(),
            owners: HashSet::new(),
            settings: OrganizationSettings::default(),
            billing_tier: BillingTier::Free,
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

    /// Set billing tier
    pub fn with_billing_tier(mut self, tier: BillingTier) -> Self {
        self.billing_tier = tier;
        self
    }

    /// Add an owner
    pub fn add_owner(&mut self, user_id: UserId) {
        self.owners.insert(user_id.clone());
        self.members.insert(user_id);
        self.updated_at = Utc::now();
    }

    /// Remove an owner (but keep as member)
    pub fn remove_owner(&mut self, user_id: &UserId) -> bool {
        let removed = self.owners.remove(user_id);
        if removed {
            self.updated_at = Utc::now();
        }
        removed
    }

    /// Add a member
    pub fn add_member(&mut self, user_id: UserId) {
        self.members.insert(user_id);
        self.updated_at = Utc::now();
    }

    /// Remove a member (and from owners if applicable)
    pub fn remove_member(&mut self, user_id: &UserId) -> bool {
        self.owners.remove(user_id);
        let removed = self.members.remove(user_id);
        if removed {
            self.updated_at = Utc::now();
        }
        removed
    }

    /// Add a team
    pub fn add_team(&mut self, team_id: TeamId) {
        self.teams.insert(team_id);
        self.updated_at = Utc::now();
    }

    /// Remove a team
    pub fn remove_team(&mut self, team_id: &TeamId) -> bool {
        let removed = self.teams.remove(team_id);
        if removed {
            self.updated_at = Utc::now();
        }
        removed
    }

    /// Check if user is an owner
    pub fn is_owner(&self, user_id: &UserId) -> bool {
        self.owners.contains(user_id)
    }

    /// Check if user is a member (includes owners)
    pub fn is_member(&self, user_id: &UserId) -> bool {
        self.members.contains(user_id)
    }

    /// Get member count
    pub fn member_count(&self) -> usize {
        self.members.len()
    }

    /// Get team count
    pub fn team_count(&self) -> usize {
        self.teams.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_organization_creation() {
        let org = Organization::new("acme-corp", "Acme Corporation")
            .with_description("The Acme Corporation")
            .with_billing_tier(BillingTier::Business);

        assert_eq!(org.slug, "acme-corp");
        assert_eq!(org.name, "Acme Corporation");
        assert_eq!(org.billing_tier, BillingTier::Business);
        assert!(org.active);
    }

    #[test]
    fn test_organization_members() {
        let mut org = Organization::new("test", "Test Org");
        let user1 = UserId::new();
        let user2 = UserId::new();

        org.add_owner(user1.clone());
        assert!(org.is_owner(&user1));
        assert!(org.is_member(&user1));

        org.add_member(user2.clone());
        assert!(!org.is_owner(&user2));
        assert!(org.is_member(&user2));

        assert_eq!(org.member_count(), 2);
    }
}
