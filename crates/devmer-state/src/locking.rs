//! State locking types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a lock
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LockId(pub Uuid);

impl LockId {
    /// Create a new random lock ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse from string
    pub fn parse(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }
}

impl Default for LockId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for LockId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Information about a lock holder
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    /// Lock ID
    pub id: LockId,

    /// Who holds the lock
    pub owner: String,

    /// Operation being performed
    pub operation: String,

    /// When the lock was acquired
    pub created_at: DateTime<Utc>,

    /// When the lock expires (if any)
    pub expires_at: Option<DateTime<Utc>>,

    /// Additional info
    pub info: Option<String>,
}

impl LockInfo {
    /// Create a new lock info
    pub fn new(owner: impl Into<String>, operation: impl Into<String>) -> Self {
        Self {
            id: LockId::new(),
            owner: owner.into(),
            operation: operation.into(),
            created_at: Utc::now(),
            expires_at: None,
            info: None,
        }
    }

    /// Set expiration time
    pub fn with_expiration(mut self, expires_at: DateTime<Utc>) -> Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Set expiration duration from now
    pub fn with_ttl(mut self, ttl: chrono::Duration) -> Self {
        self.expires_at = Some(Utc::now() + ttl);
        self
    }

    /// Set additional info
    pub fn with_info(mut self, info: impl Into<String>) -> Self {
        self.info = Some(info.into());
        self
    }

    /// Check if the lock is expired
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|exp| exp < Utc::now())
            .unwrap_or(false)
    }
}

/// Status of a lock check
#[derive(Debug, Clone)]
pub enum LockStatus {
    /// No lock exists
    Unlocked,

    /// Lock exists (generic)
    Locked(LockInfo),

    /// Lock exists and is held by us
    LockedByUs(LockInfo),

    /// Lock exists and is held by someone else
    LockedByOther(LockInfo),

    /// Lock exists but is expired
    Expired(LockInfo),
}

impl LockStatus {
    /// Check if we can proceed with an operation
    pub fn can_proceed(&self, our_lock_id: Option<&LockId>) -> bool {
        match self {
            LockStatus::Unlocked => true,
            LockStatus::Expired(_) => true,
            LockStatus::LockedByUs(info) => {
                our_lock_id.map(|id| id == &info.id).unwrap_or(false)
            }
            LockStatus::Locked(_) | LockStatus::LockedByOther(_) => false,
        }
    }

    /// Get the lock info if any
    pub fn lock_info(&self) -> Option<&LockInfo> {
        match self {
            LockStatus::Unlocked => None,
            LockStatus::Locked(info) => Some(info),
            LockStatus::LockedByUs(info) => Some(info),
            LockStatus::LockedByOther(info) => Some(info),
            LockStatus::Expired(info) => Some(info),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lock_info_expiration() {
        let lock = LockInfo::new("user", "deploy")
            .with_ttl(chrono::Duration::hours(1));

        assert!(!lock.is_expired());

        let expired_lock = LockInfo::new("user", "deploy")
            .with_ttl(chrono::Duration::hours(-1));

        assert!(expired_lock.is_expired());
    }

    #[test]
    fn test_lock_status() {
        let lock = LockInfo::new("user", "deploy");
        let lock_id = lock.id.clone();

        let status = LockStatus::LockedByUs(lock);
        assert!(status.can_proceed(Some(&lock_id)));

        let other_lock = LockInfo::new("other", "deploy");
        let other_status = LockStatus::LockedByOther(other_lock);
        assert!(!other_status.can_proceed(Some(&lock_id)));
    }
}
