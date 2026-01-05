//! Local filesystem state backend

use crate::backend::{StateBackend, StateHistory};
use crate::error::{Result, StateError};
use crate::locking::{LockId, LockInfo, LockStatus};
use async_trait::async_trait;
use chrono::Utc;
use devmer_core::state::StackState;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;
use tracing::{debug, info};

/// Local filesystem state backend
pub struct LocalBackend {
    /// Root directory for state storage
    root: PathBuf,
}

impl LocalBackend {
    /// Create a new local backend
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// Create a backend using the default location (~/.devmer/state)
    pub fn default_location() -> Result<Self> {
        let home = dirs::home_dir()
            .ok_or_else(|| StateError::backend_error("Could not determine home directory"))?;
        Ok(Self::new(home.join(".devmer").join("state")))
    }

    /// Get the path for a project
    fn project_path(&self, project: &str) -> PathBuf {
        self.root.join(project)
    }

    /// Get the path for a stack
    fn stack_path(&self, project: &str, stack: &str) -> PathBuf {
        self.project_path(project).join(stack)
    }

    /// Get the state file path
    fn state_file(&self, project: &str, stack: &str) -> PathBuf {
        self.stack_path(project, stack).join("state.json")
    }

    /// Get the lock file path
    fn lock_file(&self, project: &str, stack: &str) -> PathBuf {
        self.stack_path(project, stack).join(".lock")
    }

    /// Get the history directory path
    fn history_dir(&self, project: &str, stack: &str) -> PathBuf {
        self.stack_path(project, stack).join("history")
    }

    /// Ensure directory exists
    async fn ensure_dir(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            fs::create_dir_all(path).await?;
        }
        Ok(())
    }

    /// Calculate checksum of state
    fn calculate_checksum(state: &StackState) -> String {
        let json = serde_json::to_string(state).unwrap_or_default();
        let hash = Sha256::digest(json.as_bytes());
        hex::encode(hash)
    }
}

#[async_trait]
impl StateBackend for LocalBackend {
    fn name(&self) -> &str {
        "local"
    }

    async fn get_state(&self, project: &str, stack: &str) -> Result<Option<StackState>> {
        let path = self.state_file(project, stack);

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path).await?;
        let state: StackState = serde_json::from_str(&content)?;

        debug!("Loaded state for {}/{} from {}", project, stack, path.display());

        Ok(Some(state))
    }

    async fn save_state(&self, project: &str, stack: &str, state: &StackState) -> Result<()> {
        let stack_dir = self.stack_path(project, stack);
        self.ensure_dir(&stack_dir).await?;

        let state_file = self.state_file(project, stack);

        // Save to history first
        let history_dir = self.history_dir(project, stack);
        self.ensure_dir(&history_dir).await?;

        let version = state.history.len() as u64 + 1;
        let history_file = history_dir.join(format!("{:08}.json", version));

        let content = serde_json::to_string_pretty(state)?;

        // Write history version
        fs::write(&history_file, &content).await?;

        // Write current state
        fs::write(&state_file, &content).await?;

        info!(
            "Saved state for {}/{} (version {})",
            project, stack, version
        );

        Ok(())
    }

    async fn delete_state(&self, project: &str, stack: &str) -> Result<()> {
        let stack_dir = self.stack_path(project, stack);

        if stack_dir.exists() {
            fs::remove_dir_all(&stack_dir).await?;
            info!("Deleted state for {}/{}", project, stack);
        }

        Ok(())
    }

    async fn list_stacks(&self, project: &str) -> Result<Vec<String>> {
        let project_dir = self.project_path(project);

        if !project_dir.exists() {
            return Ok(vec![]);
        }

        let mut stacks = vec![];
        let mut entries = fs::read_dir(&project_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            if entry.file_type().await?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    // Check if it has a state file
                    let state_file = entry.path().join("state.json");
                    if state_file.exists() {
                        stacks.push(name.to_string());
                    }
                }
            }
        }

        Ok(stacks)
    }

    async fn lock(&self, project: &str, stack: &str, info: LockInfo) -> Result<LockId> {
        let stack_dir = self.stack_path(project, stack);
        self.ensure_dir(&stack_dir).await?;

        let lock_file = self.lock_file(project, stack);

        // Check for existing lock
        if lock_file.exists() {
            let content = fs::read_to_string(&lock_file).await?;
            let existing: LockInfo = serde_json::from_str(&content)?;

            if !existing.is_expired() {
                return Err(StateError::locked(format!(
                    "Stack is locked by {} since {} for operation: {}",
                    existing.owner, existing.created_at, existing.operation
                )));
            }

            // Lock is expired, we can take over
            debug!("Taking over expired lock on {}/{}", project, stack);
        }

        // Write lock file
        let content = serde_json::to_string_pretty(&info)?;
        fs::write(&lock_file, content).await?;

        info!(
            "Acquired lock on {}/{} ({})",
            project, stack, info.operation
        );

        Ok(info.id)
    }

    async fn unlock(&self, project: &str, stack: &str, lock_id: &LockId) -> Result<()> {
        let lock_file = self.lock_file(project, stack);

        if !lock_file.exists() {
            return Ok(());
        }

        // Verify we hold the lock
        let content = fs::read_to_string(&lock_file).await?;
        let existing: LockInfo = serde_json::from_str(&content)?;

        if &existing.id != lock_id {
            return Err(StateError::LockNotHeld);
        }

        fs::remove_file(&lock_file).await?;

        info!("Released lock on {}/{}", project, stack);

        Ok(())
    }

    async fn get_lock_status(&self, project: &str, stack: &str) -> Result<LockStatus> {
        let lock_file = self.lock_file(project, stack);

        if !lock_file.exists() {
            return Ok(LockStatus::Unlocked);
        }

        let content = fs::read_to_string(&lock_file).await?;
        let info: LockInfo = serde_json::from_str(&content)?;

        if info.is_expired() {
            Ok(LockStatus::Expired(info))
        } else {
            // We can't tell if it's "us" without more context, so assume other
            Ok(LockStatus::LockedByOther(info))
        }
    }

    async fn force_unlock(&self, project: &str, stack: &str) -> Result<()> {
        let lock_file = self.lock_file(project, stack);

        if lock_file.exists() {
            fs::remove_file(&lock_file).await?;
            info!("Force unlocked {}/{}", project, stack);
        }

        Ok(())
    }

    async fn get_history(
        &self,
        project: &str,
        stack: &str,
        limit: Option<u32>,
    ) -> Result<Vec<StateHistory>> {
        let history_dir = self.history_dir(project, stack);

        if !history_dir.exists() {
            return Ok(vec![]);
        }

        // Collect entries
        let mut entries = vec![];
        let mut dir = fs::read_dir(&history_dir).await?;
        while let Some(entry) = dir.next_entry().await? {
            entries.push(entry);
        }

        // Sort by name (version number) descending
        entries.sort_by(|a, b| b.file_name().cmp(&a.file_name()));

        let limit = limit.unwrap_or(100) as usize;
        let mut history = vec![];

        for entry in entries.into_iter().take(limit) {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = fs::read_to_string(&path).await?;
                let state: StackState = serde_json::from_str(&content)?;

                let version = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                history.push(StateHistory {
                    version,
                    timestamp: state.metadata.last_modified.unwrap_or_else(Utc::now),
                    actor: None,
                    operation: "update".to_string(),
                    message: None,
                    checksum: Self::calculate_checksum(&state),
                });
            }
        }

        Ok(history)
    }

    async fn get_state_version(
        &self,
        project: &str,
        stack: &str,
        version: u64,
    ) -> Result<Option<StackState>> {
        let history_file = self.history_dir(project, stack).join(format!("{:08}.json", version));

        if !history_file.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&history_file).await?;
        let state: StackState = serde_json::from_str(&content)?;

        Ok(Some(state))
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn create_test_backend() -> (LocalBackend, TempDir) {
        let temp = TempDir::new().unwrap();
        let backend = LocalBackend::new(temp.path());
        (backend, temp)
    }

    #[tokio::test]
    async fn test_save_and_get_state() {
        let (backend, _temp) = create_test_backend().await;

        let state = StackState::with_project("test-project", "dev");

        backend
            .save_state("test-project", "dev", &state)
            .await
            .unwrap();

        let loaded = backend
            .get_state("test-project", "dev")
            .await
            .unwrap()
            .unwrap();

        assert_eq!(loaded.project, "test-project");
        assert_eq!(loaded.stack, "dev");
    }

    #[tokio::test]
    async fn test_list_stacks() {
        let (backend, _temp) = create_test_backend().await;

        let state1 = StackState::with_project("project", "dev");
        let state2 = StackState::with_project("project", "prod");

        backend.save_state("project", "dev", &state1).await.unwrap();
        backend.save_state("project", "prod", &state2).await.unwrap();

        let stacks = backend.list_stacks("project").await.unwrap();
        assert_eq!(stacks.len(), 2);
        assert!(stacks.contains(&"dev".to_string()));
        assert!(stacks.contains(&"prod".to_string()));
    }

    #[tokio::test]
    async fn test_locking() {
        let (backend, _temp) = create_test_backend().await;

        let lock_info = LockInfo::new("test-user", "deploy");
        let lock_id = backend
            .lock("project", "dev", lock_info)
            .await
            .unwrap();

        // Try to lock again - should fail
        let lock_info2 = LockInfo::new("other-user", "deploy");
        let result = backend.lock("project", "dev", lock_info2).await;
        assert!(result.is_err());

        // Unlock
        backend.unlock("project", "dev", &lock_id).await.unwrap();

        // Now should be able to lock
        let lock_info3 = LockInfo::new("other-user", "deploy");
        let result = backend.lock("project", "dev", lock_info3).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_state_not_found() {
        let (backend, _temp) = create_test_backend().await;

        let result = backend.get_state("nonexistent", "stack").await.unwrap();
        assert!(result.is_none());
    }
}
