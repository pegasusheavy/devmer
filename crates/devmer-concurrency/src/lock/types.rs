//! Lock types and structures.

use std::fmt;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for a lock.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LockId(Uuid);

impl LockId {
    /// Create a new random lock ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse from string.
    pub fn parse(s: &str) -> Option<Self> {
        Uuid::parse_str(s).ok().map(Self)
    }

    /// Get the inner UUID.
    pub fn as_uuid(&self) -> &Uuid {
        &self.0
    }
}

impl Default for LockId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for LockId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Information about a lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockInfo {
    /// Unique lock ID.
    pub id: LockId,
    /// Resource being locked (e.g., "project/stack").
    pub resource: String,
    /// Who holds the lock (user ID or system identifier).
    pub holder: String,
    /// Human-readable holder name.
    pub holder_name: Option<String>,
    /// Operation being performed.
    pub operation: String,
    /// When the lock was acquired.
    pub acquired_at: DateTime<Utc>,
    /// When the lock expires (for automatic cleanup).
    pub expires_at: DateTime<Utc>,
    /// Last heartbeat time.
    pub last_heartbeat: DateTime<Utc>,
    /// Machine/host that holds the lock.
    pub host: Option<String>,
    /// Process ID on that host.
    pub pid: Option<u32>,
    /// Additional context/message.
    pub message: Option<String>,
    /// Whether this is a force-acquired lock (broke another lock).
    pub force_acquired: bool,
    /// Previous lock holder if force-acquired.
    pub previous_holder: Option<String>,
}

impl LockInfo {
    /// Create a new lock info.
    pub fn new(
        resource: impl Into<String>,
        holder: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: LockId::new(),
            resource: resource.into(),
            holder: holder.into(),
            holder_name: None,
            operation: operation.into(),
            acquired_at: now,
            expires_at: now + Duration::minutes(30), // Default 30 min TTL
            last_heartbeat: now,
            host: hostname::get().ok().and_then(|h| h.into_string().ok()),
            pid: Some(std::process::id()),
            message: None,
            force_acquired: false,
            previous_holder: None,
        }
    }

    /// Set holder name.
    pub fn with_holder_name(mut self, name: impl Into<String>) -> Self {
        self.holder_name = Some(name.into());
        self
    }

    /// Set TTL (time to live).
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.expires_at = Utc::now() + ttl;
        self
    }

    /// Set message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Check if the lock is expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if the lock needs a heartbeat (within 20% of TTL).
    pub fn needs_heartbeat(&self) -> bool {
        let remaining = self.expires_at - Utc::now();
        let ttl = self.expires_at - self.acquired_at;
        remaining < ttl / 5
    }

    /// Refresh the heartbeat.
    pub fn refresh_heartbeat(&mut self, new_ttl: Option<Duration>) {
        self.last_heartbeat = Utc::now();
        if let Some(ttl) = new_ttl {
            self.expires_at = Utc::now() + ttl;
        }
    }

    /// Get time remaining until expiry.
    pub fn time_remaining(&self) -> Duration {
        let remaining = self.expires_at - Utc::now();
        if remaining.num_seconds() < 0 {
            Duration::zero()
        } else {
            remaining
        }
    }

    /// Get a display string for the holder.
    pub fn holder_display(&self) -> String {
        if let Some(name) = &self.holder_name {
            format!("{} ({})", name, self.holder)
        } else {
            self.holder.clone()
        }
    }
}

/// Lock request - used when trying to acquire a lock.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockRequest {
    /// Resource to lock.
    pub resource: String,
    /// Requester ID.
    pub holder: String,
    /// Requester name.
    pub holder_name: Option<String>,
    /// Operation to perform.
    pub operation: String,
    /// Requested TTL.
    pub ttl: Duration,
    /// Wait for lock if not immediately available.
    pub wait: bool,
    /// Maximum time to wait.
    pub wait_timeout: Option<Duration>,
    /// Message/context.
    pub message: Option<String>,
    /// Force acquire (break existing lock).
    pub force: bool,
}

impl LockRequest {
    /// Create a new lock request.
    pub fn new(
        resource: impl Into<String>,
        holder: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        Self {
            resource: resource.into(),
            holder: holder.into(),
            holder_name: None,
            operation: operation.into(),
            ttl: Duration::minutes(30),
            wait: false,
            wait_timeout: None,
            message: None,
            force: false,
        }
    }

    /// Set holder name.
    pub fn with_holder_name(mut self, name: impl Into<String>) -> Self {
        self.holder_name = Some(name.into());
        self
    }

    /// Set TTL.
    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    /// Wait for lock.
    pub fn wait_for_lock(mut self, timeout: Duration) -> Self {
        self.wait = true;
        self.wait_timeout = Some(timeout);
        self
    }

    /// Set message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Force acquire (admin operation).
    pub fn force(mut self) -> Self {
        self.force = true;
        self
    }
}

/// Status of a lock check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LockStatus {
    /// No lock exists - resource is available.
    Available,
    /// Locked by the requesting user.
    LockedByYou { info: LockInfo },
    /// Locked by another user.
    LockedByOther { info: LockInfo },
    /// Lock exists but is expired (can be acquired).
    Expired { info: LockInfo },
}

impl LockStatus {
    /// Check if the resource can be locked by the given holder.
    pub fn can_acquire(&self, holder: &str) -> bool {
        match self {
            LockStatus::Available => true,
            LockStatus::Expired { .. } => true,
            LockStatus::LockedByYou { info } => info.holder == holder,
            LockStatus::LockedByOther { .. } => false,
        }
    }

    /// Get lock info if locked.
    pub fn lock_info(&self) -> Option<&LockInfo> {
        match self {
            LockStatus::Available => None,
            LockStatus::LockedByYou { info } => Some(info),
            LockStatus::LockedByOther { info } => Some(info),
            LockStatus::Expired { info } => Some(info),
        }
    }

    /// Check if locked by anyone (including expired).
    pub fn is_locked(&self) -> bool {
        !matches!(self, LockStatus::Available)
    }
}

/// Queue entry for waiting lock requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    /// Queue position (1-based).
    pub position: u32,
    /// The lock request.
    pub request: LockRequest,
    /// When the request was queued.
    pub queued_at: DateTime<Utc>,
    /// Estimated wait time.
    pub estimated_wait: Option<Duration>,
}

/// Result of a lock acquisition attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum LockResult {
    /// Lock acquired successfully.
    Acquired { info: LockInfo },
    /// Added to queue, waiting for lock.
    Queued { entry: QueueEntry },
    /// Lock denied - held by another user.
    Denied { holder: LockInfo },
    /// Lock force-acquired (broke another lock).
    ForceAcquired {
        info: LockInfo,
        previous: LockInfo,
    },
}

impl LockResult {
    /// Check if lock was acquired.
    pub fn is_acquired(&self) -> bool {
        matches!(self, LockResult::Acquired { .. } | LockResult::ForceAcquired { .. })
    }

    /// Get the lock info if acquired.
    pub fn lock_info(&self) -> Option<&LockInfo> {
        match self {
            LockResult::Acquired { info } => Some(info),
            LockResult::ForceAcquired { info, .. } => Some(info),
            _ => None,
        }
    }
}
