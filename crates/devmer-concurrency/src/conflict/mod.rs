//! Conflict detection and prevention.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::{ConcurrencyError, Result};
use crate::lock::{LockInfo, LockManager};
use crate::session::{SessionInfo, SessionManager};

/// A potential conflict between operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// Conflict ID.
    pub id: String,
    /// Resource involved.
    pub resource: String,
    /// Users involved.
    pub users: Vec<String>,
    /// Operations involved.
    pub operations: Vec<String>,
    /// Conflict type.
    pub conflict_type: ConflictType,
    /// Severity.
    pub severity: ConflictSeverity,
    /// Detected at.
    pub detected_at: DateTime<Utc>,
    /// Description.
    pub description: String,
    /// Recommendations.
    pub recommendations: Vec<String>,
}

impl Conflict {
    /// Create a new conflict.
    pub fn new(
        resource: impl Into<String>,
        conflict_type: ConflictType,
        severity: ConflictSeverity,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            resource: resource.into(),
            users: Vec::new(),
            operations: Vec::new(),
            conflict_type,
            severity,
            detected_at: Utc::now(),
            description: description.into(),
            recommendations: Vec::new(),
        }
    }

    /// Add user.
    pub fn with_user(mut self, user: impl Into<String>) -> Self {
        self.users.push(user.into());
        self
    }

    /// Add operation.
    pub fn with_operation(mut self, operation: impl Into<String>) -> Self {
        self.operations.push(operation.into());
        self
    }

    /// Add recommendation.
    pub fn with_recommendation(mut self, rec: impl Into<String>) -> Self {
        self.recommendations.push(rec.into());
        self
    }
}

/// Type of conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictType {
    /// Two users trying to modify the same resource.
    ConcurrentModification,
    /// Dependent resource is being modified.
    DependencyConflict,
    /// Resource is locked.
    ResourceLocked,
    /// Operation would overwrite uncommitted changes.
    UncommittedChanges,
    /// State version mismatch.
    VersionMismatch,
}

/// Severity of conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConflictSeverity {
    /// Warning - can proceed with caution.
    Warning,
    /// Error - should not proceed.
    Error,
    /// Critical - must not proceed.
    Critical,
}

/// Pre-operation check result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreOperationCheck {
    /// Can proceed with operation.
    pub can_proceed: bool,
    /// Conflicts detected.
    pub conflicts: Vec<Conflict>,
    /// Warnings (non-blocking).
    pub warnings: Vec<String>,
    /// Other users accessing the same resources.
    pub other_users: Vec<OtherUser>,
    /// Checked at.
    pub checked_at: DateTime<Utc>,
}

impl PreOperationCheck {
    /// Create a passing check.
    pub fn pass() -> Self {
        Self {
            can_proceed: true,
            conflicts: Vec::new(),
            warnings: Vec::new(),
            other_users: Vec::new(),
            checked_at: Utc::now(),
        }
    }

    /// Create a failing check.
    pub fn fail(conflicts: Vec<Conflict>) -> Self {
        Self {
            can_proceed: false,
            conflicts,
            warnings: Vec::new(),
            other_users: Vec::new(),
            checked_at: Utc::now(),
        }
    }

    /// Add warning.
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Add other user.
    pub fn with_other_user(mut self, user: OtherUser) -> Self {
        self.other_users.push(user);
        self
    }
}

/// Information about another user accessing the resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtherUser {
    /// User ID.
    pub user_id: String,
    /// User name.
    pub user_name: Option<String>,
    /// What they're doing.
    pub operation: Option<String>,
    /// Since when.
    pub since: DateTime<Utc>,
}

/// Conflict detector.
pub struct ConflictDetector {
    lock_manager: Arc<LockManager>,
    session_manager: Arc<SessionManager>,
    // Track resources and their dependencies
    dependencies: RwLock<HashMap<String, HashSet<String>>>,
}

impl ConflictDetector {
    /// Create a new conflict detector.
    pub fn new(lock_manager: Arc<LockManager>, session_manager: Arc<SessionManager>) -> Self {
        Self {
            lock_manager,
            session_manager,
            dependencies: RwLock::new(HashMap::new()),
        }
    }

    /// Register a dependency between resources.
    pub async fn register_dependency(&self, resource: &str, depends_on: &str) {
        let mut deps = self.dependencies.write().await;
        deps.entry(resource.to_string())
            .or_default()
            .insert(depends_on.to_string());
    }

    /// Check for conflicts before starting an operation.
    pub async fn check_before_operation(
        &self,
        resource: &str,
        user_id: &str,
        operation: &str,
    ) -> Result<PreOperationCheck> {
        let mut check = PreOperationCheck::pass();
        let mut conflicts = Vec::new();

        // Check if resource is locked by another user
        let lock_status = self.lock_manager.status(resource, user_id).await?;
        if let crate::lock::LockStatus::LockedByOther { info } = lock_status {
            let conflict = Conflict::new(
                resource,
                ConflictType::ResourceLocked,
                ConflictSeverity::Critical,
                format!(
                    "Resource is locked by {} (operation: {})",
                    info.holder_display(),
                    info.operation
                ),
            )
            .with_user(&info.holder)
            .with_operation(&info.operation)
            .with_recommendation("Wait for the lock to be released")
            .with_recommendation(format!(
                "Lock will expire at {}",
                info.expires_at.format("%H:%M:%S UTC")
            ));

            conflicts.push(conflict);
        }

        // Check for other users accessing the same resource
        let sessions = self.session_manager.who_is_accessing(resource).await?;
        for session in sessions {
            if session.user_id != user_id {
                check = check.with_other_user(OtherUser {
                    user_id: session.user_id.clone(),
                    user_name: session.user_name.clone(),
                    operation: session.current_operation.clone(),
                    since: session.last_activity,
                });

                // If they're also modifying, that's a potential conflict
                if let Some(op) = &session.current_operation {
                    if is_modifying_operation(op) && is_modifying_operation(operation) {
                        let conflict = Conflict::new(
                            resource,
                            ConflictType::ConcurrentModification,
                            ConflictSeverity::Warning,
                            format!(
                                "User {} is also performing '{}' on this resource",
                                session.user_name.as_deref().unwrap_or(&session.user_id),
                                op
                            ),
                        )
                        .with_user(&session.user_id)
                        .with_operation(op)
                        .with_recommendation("Coordinate with the other user")
                        .with_recommendation("Consider waiting for them to finish");

                        check.warnings.push(format!(
                            "Warning: {} is also modifying this resource",
                            session.user_name.as_deref().unwrap_or(&session.user_id)
                        ));

                        // Don't block for warnings, but record them
                        if !conflicts.iter().any(|c| c.severity == ConflictSeverity::Critical) {
                            // Only add as warning if no critical conflicts
                        }
                    }
                }
            }
        }

        // Check dependencies
        let deps = self.dependencies.read().await;
        if let Some(resource_deps) = deps.get(resource) {
            for dep in resource_deps {
                let dep_lock = self.lock_manager.status(dep, user_id).await?;
                if let crate::lock::LockStatus::LockedByOther { info } = dep_lock {
                    let conflict = Conflict::new(
                        resource,
                        ConflictType::DependencyConflict,
                        ConflictSeverity::Error,
                        format!(
                            "Dependent resource '{}' is locked by {}",
                            dep,
                            info.holder_display()
                        ),
                    )
                    .with_user(&info.holder)
                    .with_operation(&info.operation)
                    .with_recommendation(format!(
                        "Wait for '{}' to be unlocked first",
                        dep
                    ));

                    conflicts.push(conflict);
                }
            }
        }

        if !conflicts.is_empty() {
            let has_critical = conflicts.iter().any(|c| c.severity >= ConflictSeverity::Error);
            check.can_proceed = !has_critical;
            check.conflicts = conflicts;
        }

        Ok(check)
    }

    /// Check if an operation would conflict with current state.
    pub async fn would_conflict(
        &self,
        resource: &str,
        user_id: &str,
        expected_version: Option<u64>,
        current_version: u64,
    ) -> Result<Option<Conflict>> {
        if let Some(expected) = expected_version {
            if expected != current_version {
                return Ok(Some(
                    Conflict::new(
                        resource,
                        ConflictType::VersionMismatch,
                        ConflictSeverity::Error,
                        format!(
                            "State version mismatch: expected {}, found {}. Someone else may have modified this resource.",
                            expected, current_version
                        ),
                    )
                    .with_recommendation("Refresh your local state and try again")
                    .with_recommendation("Use 'devmer state pull' to get the latest state")
                ));
            }
        }

        Ok(None)
    }
}

/// Check if an operation is a modifying operation.
fn is_modifying_operation(operation: &str) -> bool {
    let modifying_ops = [
        "up", "down", "apply", "destroy", "deploy", "update", "create", "delete",
        "import", "refresh", "migrate",
    ];
    
    let op_lower = operation.to_lowercase();
    modifying_ops.iter().any(|m| op_lower.contains(m))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::LockRequest;
    use crate::session::ClientInfo;

    #[tokio::test]
    async fn test_conflict_detection_locked_resource() {
        let lock_manager = Arc::new(LockManager::in_memory());
        let session_manager = Arc::new(SessionManager::in_memory());
        let detector = ConflictDetector::new(lock_manager.clone(), session_manager);

        // User1 locks the resource
        let req = LockRequest::new("project/stack", "user1", "deploy");
        lock_manager.acquire(req).await.unwrap();

        // User2 checks for conflicts
        let check = detector
            .check_before_operation("project/stack", "user2", "deploy")
            .await
            .unwrap();

        assert!(!check.can_proceed);
        assert_eq!(check.conflicts.len(), 1);
        assert_eq!(check.conflicts[0].conflict_type, ConflictType::ResourceLocked);
    }

    #[tokio::test]
    async fn test_no_conflict_same_user() {
        let lock_manager = Arc::new(LockManager::in_memory());
        let session_manager = Arc::new(SessionManager::in_memory());
        let detector = ConflictDetector::new(lock_manager.clone(), session_manager);

        // User1 locks the resource
        let req = LockRequest::new("project/stack", "user1", "deploy");
        lock_manager.acquire(req).await.unwrap();

        // User1 checks for conflicts (same user)
        let check = detector
            .check_before_operation("project/stack", "user1", "deploy")
            .await
            .unwrap();

        assert!(check.can_proceed);
        assert!(check.conflicts.is_empty());
    }

    #[tokio::test]
    async fn test_version_mismatch() {
        let lock_manager = Arc::new(LockManager::in_memory());
        let session_manager = Arc::new(SessionManager::in_memory());
        let detector = ConflictDetector::new(lock_manager, session_manager);

        let conflict = detector
            .would_conflict("project/stack", "user1", Some(5), 7)
            .await
            .unwrap();

        assert!(conflict.is_some());
        assert_eq!(conflict.unwrap().conflict_type, ConflictType::VersionMismatch);
    }
}
