//! User management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a user
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(pub Uuid);

impl UserId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn parse(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }
}

impl Default for UserId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// User status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UserStatus {
    /// User is active
    Active,
    /// User is invited but hasn't accepted
    Invited,
    /// User is suspended
    Suspended,
    /// User is deactivated
    Deactivated,
}

impl Default for UserStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Authentication method
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthMethod {
    /// Password-based authentication
    Password {
        /// Password hash
        hash: String,
        /// Last password change
        changed_at: DateTime<Utc>,
        /// Require password change on next login
        require_change: bool,
    },
    /// SSO/OIDC authentication
    Sso {
        /// Identity provider
        provider: String,
        /// External user ID
        external_id: String,
    },
    /// API key authentication
    ApiKey {
        /// Key prefix (for identification)
        prefix: String,
        /// Key hash
        hash: String,
        /// Expiration
        expires_at: Option<DateTime<Utc>>,
    },
    /// Personal access token
    Token {
        /// Token name
        name: String,
        /// Token hash
        hash: String,
        /// Scopes
        scopes: Vec<String>,
        /// Expiration
        expires_at: Option<DateTime<Utc>>,
    },
}

/// A user in the system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    /// Unique identifier
    pub id: UserId,

    /// Email address (unique)
    pub email: String,

    /// Username (unique, URL-safe)
    pub username: String,

    /// Display name
    pub display_name: String,

    /// Avatar URL
    pub avatar_url: Option<String>,

    /// User status
    pub status: UserStatus,

    /// Authentication methods
    pub auth_methods: Vec<AuthMethod>,

    /// Two-factor authentication enabled
    pub mfa_enabled: bool,

    /// User preferences
    pub preferences: UserPreferences,

    /// Custom metadata
    pub metadata: HashMap<String, String>,

    /// When created
    pub created_at: DateTime<Utc>,

    /// When last updated
    pub updated_at: DateTime<Utc>,

    /// Last login time
    pub last_login_at: Option<DateTime<Utc>>,

    /// Last active time
    pub last_active_at: Option<DateTime<Utc>>,
}

/// User preferences
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Default organization
    pub default_organization: Option<String>,

    /// Default stack
    pub default_stack: Option<String>,

    /// Theme preference
    pub theme: String,

    /// Timezone
    pub timezone: String,

    /// Email notification preferences
    pub email_notifications: EmailNotifications,
}

/// Email notification settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailNotifications {
    /// Deployment started
    pub deployment_started: bool,
    /// Deployment completed
    pub deployment_completed: bool,
    /// Deployment failed
    pub deployment_failed: bool,
    /// Approval requested
    pub approval_requested: bool,
    /// Weekly digest
    pub weekly_digest: bool,
}

impl Default for EmailNotifications {
    fn default() -> Self {
        Self {
            deployment_started: false,
            deployment_completed: true,
            deployment_failed: true,
            approval_requested: true,
            weekly_digest: true,
        }
    }
}

impl User {
    /// Create a new user
    pub fn new(
        email: impl Into<String>,
        username: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: UserId::new(),
            email: email.into(),
            username: username.into(),
            display_name: display_name.into(),
            avatar_url: None,
            status: UserStatus::Active,
            auth_methods: Vec::new(),
            mfa_enabled: false,
            preferences: UserPreferences::default(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
            last_login_at: None,
            last_active_at: None,
        }
    }

    /// Create an invited user
    pub fn invited(email: impl Into<String>) -> Self {
        let email = email.into();
        let username = email.split('@').next().unwrap_or("user").to_string();
        let mut user = Self::new(&email, &username, &username);
        user.status = UserStatus::Invited;
        user
    }

    /// Check if user is active
    pub fn is_active(&self) -> bool {
        self.status == UserStatus::Active
    }

    /// Check if user can login
    pub fn can_login(&self) -> bool {
        matches!(self.status, UserStatus::Active | UserStatus::Invited)
    }

    /// Record login
    pub fn record_login(&mut self) {
        let now = Utc::now();
        self.last_login_at = Some(now);
        self.last_active_at = Some(now);
        self.updated_at = now;

        // Activate invited users on first login
        if self.status == UserStatus::Invited {
            self.status = UserStatus::Active;
        }
    }

    /// Record activity
    pub fn record_activity(&mut self) {
        self.last_active_at = Some(Utc::now());
    }

    /// Suspend user
    pub fn suspend(&mut self) {
        self.status = UserStatus::Suspended;
        self.updated_at = Utc::now();
    }

    /// Reactivate user
    pub fn reactivate(&mut self) {
        self.status = UserStatus::Active;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new("alice@example.com", "alice", "Alice Smith");

        assert_eq!(user.email, "alice@example.com");
        assert_eq!(user.username, "alice");
        assert!(user.is_active());
    }

    #[test]
    fn test_invited_user() {
        let mut user = User::invited("bob@example.com");

        assert_eq!(user.status, UserStatus::Invited);
        assert!(user.can_login());

        user.record_login();
        assert_eq!(user.status, UserStatus::Active);
    }

    #[test]
    fn test_user_suspension() {
        let mut user = User::new("alice@example.com", "alice", "Alice");

        user.suspend();
        assert_eq!(user.status, UserStatus::Suspended);
        assert!(!user.can_login());

        user.reactivate();
        assert!(user.is_active());
    }
}
