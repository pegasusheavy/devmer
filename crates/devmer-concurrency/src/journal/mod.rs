//! Operation journal for audit and tracking.

use std::collections::VecDeque;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::error::Result;

/// Journal entry for an operation event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntry {
    /// Entry ID.
    pub id: String,
    /// Resource involved.
    pub resource: String,
    /// Actor (user/system).
    pub actor: String,
    /// Event type.
    pub event_type: JournalEventType,
    /// Timestamp.
    pub timestamp: DateTime<Utc>,
    /// Message/description.
    pub message: Option<String>,
    /// Additional metadata.
    pub metadata: std::collections::HashMap<String, String>,
}

impl JournalEntry {
    /// Create a new journal entry.
    pub fn new(resource: &str, actor: &str, event_type: JournalEventType) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            resource: resource.to_string(),
            actor: actor.to_string(),
            event_type,
            timestamp: Utc::now(),
            message: None,
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set message.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = Some(message.into());
        self
    }

    /// Add metadata.
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

/// Types of journal events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalEventType {
    /// Lock acquired.
    LockAcquired,
    /// Lock released.
    LockReleased,
    /// Lock force-acquired (broke another lock).
    LockForceAcquired,
    /// Lock force-released (admin).
    LockForceReleased,
    /// Lock expired.
    LockExpired,
    /// Operation started.
    OperationStarted,
    /// Operation completed.
    OperationCompleted,
    /// Operation failed.
    OperationFailed,
    /// Conflict detected.
    ConflictDetected,
    /// Session started.
    SessionStarted,
    /// Session ended.
    SessionEnded,
    /// State updated.
    StateUpdated,
}

/// Query for journal entries.
#[derive(Debug, Clone, Default)]
pub struct JournalQuery {
    /// Filter by resource.
    pub resource: Option<String>,
    /// Filter by actor.
    pub actor: Option<String>,
    /// Filter by event type.
    pub event_type: Option<JournalEventType>,
    /// From timestamp.
    pub from: Option<DateTime<Utc>>,
    /// To timestamp.
    pub to: Option<DateTime<Utc>>,
    /// Limit.
    pub limit: usize,
    /// Offset.
    pub offset: usize,
}

impl JournalQuery {
    /// Create a new query.
    pub fn new() -> Self {
        Self {
            limit: 100,
            ..Default::default()
        }
    }

    /// Filter by resource.
    pub fn resource(mut self, resource: impl Into<String>) -> Self {
        self.resource = Some(resource.into());
        self
    }

    /// Filter by actor.
    pub fn actor(mut self, actor: impl Into<String>) -> Self {
        self.actor = Some(actor.into());
        self
    }

    /// Filter by event type.
    pub fn event_type(mut self, event_type: JournalEventType) -> Self {
        self.event_type = Some(event_type);
        self
    }

    /// Set time range.
    pub fn between(mut self, from: DateTime<Utc>, to: DateTime<Utc>) -> Self {
        self.from = Some(from);
        self.to = Some(to);
        self
    }

    /// Set limit.
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

/// Trait for operation journal storage.
#[async_trait]
pub trait OperationJournal: Send + Sync {
    /// Log an entry.
    async fn log(&self, entry: JournalEntry) -> Result<()>;

    /// Query entries.
    async fn query(&self, query: &JournalQuery) -> Result<Vec<JournalEntry>>;

    /// Get recent entries for a resource.
    async fn recent_for_resource(&self, resource: &str, limit: usize) -> Result<Vec<JournalEntry>>;

    /// Get recent entries for an actor.
    async fn recent_for_actor(&self, actor: &str, limit: usize) -> Result<Vec<JournalEntry>>;
}

/// In-memory journal for testing and simple deployments.
pub struct InMemoryJournal {
    entries: RwLock<VecDeque<JournalEntry>>,
    max_entries: usize,
}

impl InMemoryJournal {
    /// Create a new in-memory journal.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(VecDeque::new()),
            max_entries,
        }
    }
}

impl Default for InMemoryJournal {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[async_trait]
impl OperationJournal for InMemoryJournal {
    async fn log(&self, entry: JournalEntry) -> Result<()> {
        let mut entries = self.entries.write().await;
        
        // Remove old entries if at capacity
        while entries.len() >= self.max_entries {
            entries.pop_front();
        }

        tracing::trace!(
            resource = %entry.resource,
            actor = %entry.actor,
            event = ?entry.event_type,
            "Journal entry"
        );

        entries.push_back(entry);
        Ok(())
    }

    async fn query(&self, query: &JournalQuery) -> Result<Vec<JournalEntry>> {
        let entries = self.entries.read().await;

        let filtered: Vec<_> = entries
            .iter()
            .filter(|e| {
                if let Some(ref r) = query.resource {
                    if &e.resource != r { return false; }
                }
                if let Some(ref a) = query.actor {
                    if &e.actor != a { return false; }
                }
                if let Some(et) = query.event_type {
                    if e.event_type != et { return false; }
                }
                if let Some(from) = query.from {
                    if e.timestamp < from { return false; }
                }
                if let Some(to) = query.to {
                    if e.timestamp > to { return false; }
                }
                true
            })
            .skip(query.offset)
            .take(query.limit)
            .cloned()
            .collect();

        Ok(filtered)
    }

    async fn recent_for_resource(&self, resource: &str, limit: usize) -> Result<Vec<JournalEntry>> {
        let entries = self.entries.read().await;

        let mut filtered: Vec<_> = entries
            .iter()
            .filter(|e| e.resource == resource)
            .cloned()
            .collect();

        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        filtered.truncate(limit);

        Ok(filtered)
    }

    async fn recent_for_actor(&self, actor: &str, limit: usize) -> Result<Vec<JournalEntry>> {
        let entries = self.entries.read().await;

        let mut filtered: Vec<_> = entries
            .iter()
            .filter(|e| e.actor == actor)
            .cloned()
            .collect();

        filtered.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        filtered.truncate(limit);

        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_journal_logging() {
        let journal = InMemoryJournal::default();

        let entry = JournalEntry::new("project/stack", "user1", JournalEventType::LockAcquired)
            .with_message("Acquired lock for deploy");

        journal.log(entry).await.unwrap();

        let recent = journal.recent_for_resource("project/stack", 10).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].event_type, JournalEventType::LockAcquired);
    }

    #[tokio::test]
    async fn test_journal_query() {
        let journal = InMemoryJournal::default();

        // Log multiple entries
        for i in 0..5 {
            let entry = JournalEntry::new(
                "project/stack",
                if i % 2 == 0 { "user1" } else { "user2" },
                JournalEventType::LockAcquired,
            );
            journal.log(entry).await.unwrap();
        }

        // Query by actor
        let query = JournalQuery::new().actor("user1");
        let results = journal.query(&query).await.unwrap();
        assert_eq!(results.len(), 3);
    }
}
