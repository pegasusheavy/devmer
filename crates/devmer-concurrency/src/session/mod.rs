//! User session tracking for multi-user coordination.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::{ConcurrencyError, Result};

/// Unique session identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    /// Create a new session ID.
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4().to_string())
    }

    /// Parse from string.
    pub fn parse(s: impl Into<String>) -> Self {
        Self(s.into())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Information about an active user session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session ID.
    pub id: SessionId,
    /// User ID.
    pub user_id: String,
    /// User display name.
    pub user_name: Option<String>,
    /// When the session started.
    pub started_at: DateTime<Utc>,
    /// Last activity time.
    pub last_activity: DateTime<Utc>,
    /// Session expires at.
    pub expires_at: DateTime<Utc>,
    /// Current operation (if any).
    pub current_operation: Option<String>,
    /// Resources currently being accessed.
    pub active_resources: Vec<String>,
    /// Client information.
    pub client_info: ClientInfo,
}

impl SessionInfo {
    /// Create a new session.
    pub fn new(user_id: impl Into<String>, client_info: ClientInfo) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::new(),
            user_id: user_id.into(),
            user_name: None,
            started_at: now,
            last_activity: now,
            expires_at: now + Duration::hours(8), // 8-hour default session
            current_operation: None,
            active_resources: Vec::new(),
            client_info,
        }
    }

    /// Set user name.
    pub fn with_user_name(mut self, name: impl Into<String>) -> Self {
        self.user_name = Some(name.into());
        self
    }

    /// Check if session is expired.
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Check if session is idle (no activity for given duration).
    pub fn is_idle(&self, idle_threshold: Duration) -> bool {
        Utc::now() - self.last_activity > idle_threshold
    }

    /// Update last activity.
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Set current operation.
    pub fn set_operation(&mut self, operation: Option<String>) {
        self.current_operation = operation;
        self.touch();
    }

    /// Add active resource.
    pub fn add_resource(&mut self, resource: impl Into<String>) {
        let r = resource.into();
        if !self.active_resources.contains(&r) {
            self.active_resources.push(r);
        }
        self.touch();
    }

    /// Remove active resource.
    pub fn remove_resource(&mut self, resource: &str) {
        self.active_resources.retain(|r| r != resource);
        self.touch();
    }
}

/// Client information for a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientInfo {
    /// Client type (cli, web, api, etc.).
    pub client_type: String,
    /// Client version.
    pub client_version: Option<String>,
    /// Hostname.
    pub hostname: Option<String>,
    /// IP address.
    pub ip_address: Option<String>,
    /// User agent.
    pub user_agent: Option<String>,
}

impl ClientInfo {
    /// Create CLI client info.
    pub fn cli(version: Option<String>) -> Self {
        Self {
            client_type: "cli".to_string(),
            client_version: version,
            hostname: hostname::get().ok().and_then(|h| h.into_string().ok()),
            ip_address: None,
            user_agent: None,
        }
    }

    /// Create API client info.
    pub fn api(ip_address: Option<String>, user_agent: Option<String>) -> Self {
        Self {
            client_type: "api".to_string(),
            client_version: None,
            hostname: None,
            ip_address,
            user_agent,
        }
    }
}

/// Trait for session storage.
#[async_trait]
pub trait SessionBackend: Send + Sync {
    /// Get a session.
    async fn get_session(&self, id: &SessionId) -> Result<Option<SessionInfo>>;

    /// Store a session.
    async fn store_session(&self, session: &SessionInfo) -> Result<()>;

    /// Remove a session.
    async fn remove_session(&self, id: &SessionId) -> Result<()>;

    /// Get sessions for a user.
    async fn get_user_sessions(&self, user_id: &str) -> Result<Vec<SessionInfo>>;

    /// Get sessions accessing a resource.
    async fn get_resource_sessions(&self, resource: &str) -> Result<Vec<SessionInfo>>;

    /// List all active sessions.
    async fn list_sessions(&self) -> Result<Vec<SessionInfo>>;
}

/// In-memory session backend.
pub struct InMemorySessionBackend {
    sessions: RwLock<HashMap<String, SessionInfo>>,
}

impl InMemorySessionBackend {
    /// Create a new in-memory backend.
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }
}

impl Default for InMemorySessionBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SessionBackend for InMemorySessionBackend {
    async fn get_session(&self, id: &SessionId) -> Result<Option<SessionInfo>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(&id.0).cloned())
    }

    async fn store_session(&self, session: &SessionInfo) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.id.0.clone(), session.clone());
        Ok(())
    }

    async fn remove_session(&self, id: &SessionId) -> Result<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(&id.0);
        Ok(())
    }

    async fn get_user_sessions(&self, user_id: &str) -> Result<Vec<SessionInfo>> {
        let sessions = self.sessions.read().await;
        Ok(sessions
            .values()
            .filter(|s| s.user_id == user_id && !s.is_expired())
            .cloned()
            .collect())
    }

    async fn get_resource_sessions(&self, resource: &str) -> Result<Vec<SessionInfo>> {
        let sessions = self.sessions.read().await;
        Ok(sessions
            .values()
            .filter(|s| s.active_resources.contains(&resource.to_string()) && !s.is_expired())
            .cloned()
            .collect())
    }

    async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let sessions = self.sessions.read().await;
        Ok(sessions
            .values()
            .filter(|s| !s.is_expired())
            .cloned()
            .collect())
    }
}

/// Session manager for tracking active users.
pub struct SessionManager {
    backend: Arc<dyn SessionBackend>,
    idle_threshold: Duration,
}

impl SessionManager {
    /// Create a new session manager.
    pub fn new(backend: Arc<dyn SessionBackend>) -> Self {
        Self {
            backend,
            idle_threshold: Duration::minutes(30),
        }
    }

    /// Create with in-memory backend.
    pub fn in_memory() -> Self {
        Self::new(Arc::new(InMemorySessionBackend::new()))
    }

    /// Set idle threshold.
    pub fn with_idle_threshold(mut self, threshold: Duration) -> Self {
        self.idle_threshold = threshold;
        self
    }

    /// Create a new session.
    pub async fn create_session(&self, user_id: &str, client_info: ClientInfo) -> Result<SessionInfo> {
        let session = SessionInfo::new(user_id, client_info);
        self.backend.store_session(&session).await?;

        tracing::info!(
            session_id = %session.id,
            user_id = %user_id,
            "Session created"
        );

        Ok(session)
    }

    /// Get a session.
    pub async fn get_session(&self, id: &SessionId) -> Result<Option<SessionInfo>> {
        let session = self.backend.get_session(id).await?;
        
        // Check if expired
        if let Some(ref s) = session {
            if s.is_expired() {
                self.backend.remove_session(id).await?;
                return Err(ConcurrencyError::SessionExpired(id.to_string()));
            }
        }

        Ok(session)
    }

    /// Update session activity.
    pub async fn touch(&self, id: &SessionId) -> Result<SessionInfo> {
        let mut session = self.get_session(id).await?
            .ok_or_else(|| ConcurrencyError::SessionNotFound(id.to_string()))?;

        session.touch();
        self.backend.store_session(&session).await?;

        Ok(session)
    }

    /// End a session.
    pub async fn end_session(&self, id: &SessionId) -> Result<()> {
        self.backend.remove_session(id).await?;

        tracing::info!(session_id = %id, "Session ended");

        Ok(())
    }

    /// Get all sessions for a user.
    pub async fn get_user_sessions(&self, user_id: &str) -> Result<Vec<SessionInfo>> {
        self.backend.get_user_sessions(user_id).await
    }

    /// Get who else is accessing a resource.
    pub async fn who_is_accessing(&self, resource: &str) -> Result<Vec<SessionInfo>> {
        self.backend.get_resource_sessions(resource).await
    }

    /// List all active sessions.
    pub async fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        self.backend.list_sessions().await
    }

    /// Set current operation for a session.
    pub async fn set_operation(&self, id: &SessionId, operation: Option<String>) -> Result<()> {
        let mut session = self.get_session(id).await?
            .ok_or_else(|| ConcurrencyError::SessionNotFound(id.to_string()))?;

        session.set_operation(operation);
        self.backend.store_session(&session).await?;

        Ok(())
    }

    /// Add a resource to a session's active list.
    pub async fn add_resource(&self, id: &SessionId, resource: &str) -> Result<()> {
        let mut session = self.get_session(id).await?
            .ok_or_else(|| ConcurrencyError::SessionNotFound(id.to_string()))?;

        session.add_resource(resource);
        self.backend.store_session(&session).await?;

        Ok(())
    }

    /// Remove a resource from a session's active list.
    pub async fn remove_resource(&self, id: &SessionId, resource: &str) -> Result<()> {
        let mut session = self.get_session(id).await?
            .ok_or_else(|| ConcurrencyError::SessionNotFound(id.to_string()))?;

        session.remove_resource(resource);
        self.backend.store_session(&session).await?;

        Ok(())
    }

    /// Clean up expired and idle sessions.
    pub async fn cleanup(&self) -> Result<usize> {
        let sessions = self.backend.list_sessions().await?;
        let mut cleaned = 0;

        for session in sessions {
            if session.is_expired() || session.is_idle(self.idle_threshold) {
                self.backend.remove_session(&session.id).await?;
                cleaned += 1;

                tracing::debug!(
                    session_id = %session.id,
                    user_id = %session.user_id,
                    "Cleaned up session"
                );
            }
        }

        Ok(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_lifecycle() {
        let manager = SessionManager::in_memory();

        // Create session
        let session = manager
            .create_session("user1", ClientInfo::cli(Some("1.0.0".to_string())))
            .await
            .unwrap();

        assert_eq!(session.user_id, "user1");

        // Get session
        let retrieved = manager.get_session(&session.id).await.unwrap().unwrap();
        assert_eq!(retrieved.user_id, "user1");

        // End session
        manager.end_session(&session.id).await.unwrap();

        // Should be gone
        let gone = manager.get_session(&session.id).await.unwrap();
        assert!(gone.is_none());
    }

    #[tokio::test]
    async fn test_resource_tracking() {
        let manager = SessionManager::in_memory();

        let session = manager
            .create_session("user1", ClientInfo::cli(None))
            .await
            .unwrap();

        // Add resource
        manager.add_resource(&session.id, "project/stack").await.unwrap();

        // Check who's accessing
        let accessors = manager.who_is_accessing("project/stack").await.unwrap();
        assert_eq!(accessors.len(), 1);
        assert_eq!(accessors[0].user_id, "user1");

        // Remove resource
        manager.remove_resource(&session.id, "project/stack").await.unwrap();

        let accessors = manager.who_is_accessing("project/stack").await.unwrap();
        assert_eq!(accessors.len(), 0);
    }
}
