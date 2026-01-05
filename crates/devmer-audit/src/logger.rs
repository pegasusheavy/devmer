//! Audit logger

use crate::backend::AuditBackend;
use crate::chain::{ChainMetadata, HashChain, ChainVerificationResult};
use crate::event::AuditEvent;
use crate::query::{AuditQuery, QueryResult};
use crate::Result;
use std::sync::{Arc, RwLock};
use tracing::{debug, info};

/// Main audit logger that manages events and backends
pub struct AuditLogger {
    /// Storage backend
    backend: Arc<dyn AuditBackend>,
    /// Hash chain for integrity
    chain: Arc<RwLock<HashChain>>,
    /// Chain metadata
    chain_metadata: Arc<RwLock<ChainMetadata>>,
    /// Whether hash chaining is enabled
    chain_enabled: bool,
    /// Default project name
    default_project: Option<String>,
    /// Default organization ID
    default_organization: Option<String>,
}

impl AuditLogger {
    /// Create a new audit logger with the given backend
    pub fn new(backend: impl AuditBackend + 'static) -> Self {
        let backend = Arc::new(backend);
        Self {
            backend,
            chain: Arc::new(RwLock::new(HashChain::new())),
            chain_metadata: Arc::new(RwLock::new(ChainMetadata::new("default"))),
            chain_enabled: true,
            default_project: None,
            default_organization: None,
        }
    }

    /// Create from an Arc backend
    pub fn from_arc(backend: Arc<dyn AuditBackend>) -> Self {
        Self {
            backend,
            chain: Arc::new(RwLock::new(HashChain::new())),
            chain_metadata: Arc::new(RwLock::new(ChainMetadata::new("default"))),
            chain_enabled: true,
            default_project: None,
            default_organization: None,
        }
    }

    /// Disable hash chaining
    pub fn without_chain(mut self) -> Self {
        self.chain_enabled = false;
        self
    }

    /// Set default project
    pub fn with_default_project(mut self, project: impl Into<String>) -> Self {
        self.default_project = Some(project.into());
        self
    }

    /// Set default organization
    pub fn with_default_organization(mut self, org_id: impl Into<String>) -> Self {
        self.default_organization = Some(org_id.into());
        self
    }

    /// Initialize the logger (load chain state from backend)
    pub async fn init(&self) -> Result<()> {
        if let Some(metadata) = self.backend.get_chain_metadata().await? {
            let mut chain = self.chain.write().unwrap();
            if let Some(ref last_hash) = metadata.last_hash {
                *chain = HashChain::from_last_hash(last_hash.clone(), metadata.event_count);
            }
            *self.chain_metadata.write().unwrap() = metadata;
            info!("Loaded audit chain with {} events", chain.count());
        }
        Ok(())
    }

    /// Log an event
    pub async fn log(&self, mut event: AuditEvent) -> Result<()> {
        // Apply defaults
        if event.project.is_none() {
            event.project = self.default_project.clone();
        }
        if event.organization_id.is_none() {
            event.organization_id = self.default_organization.clone();
        }

        // Add to hash chain
        if self.chain_enabled {
            let mut chain = self.chain.write().unwrap();
            chain.add_event(&mut event);

            // Update metadata
            let mut metadata = self.chain_metadata.write().unwrap();
            metadata.update(&event);
        }

        debug!(
            event_id = %event.id,
            event_type = ?event.event_type,
            actor = %event.actor.id,
            "Logging audit event"
        );

        // Write to backend
        self.backend.write(&event).await?;

        // Periodically save chain metadata
        if self.chain_enabled {
            let metadata = self.chain_metadata.read().unwrap().clone();
            if metadata.event_count % 100 == 0 {
                self.backend.save_chain_metadata(&metadata).await?;
            }
        }

        Ok(())
    }

    /// Log multiple events
    pub async fn log_batch(&self, events: Vec<AuditEvent>) -> Result<()> {
        let mut processed_events = Vec::with_capacity(events.len());

        for mut event in events {
            // Apply defaults
            if event.project.is_none() {
                event.project = self.default_project.clone();
            }
            if event.organization_id.is_none() {
                event.organization_id = self.default_organization.clone();
            }

            // Add to hash chain
            if self.chain_enabled {
                let mut chain = self.chain.write().unwrap();
                chain.add_event(&mut event);

                let mut metadata = self.chain_metadata.write().unwrap();
                metadata.update(&event);
            }

            processed_events.push(event);
        }

        // Write batch to backend
        self.backend.write_batch(&processed_events).await?;

        // Save chain metadata
        if self.chain_enabled {
            let metadata = self.chain_metadata.read().unwrap().clone();
            self.backend.save_chain_metadata(&metadata).await?;
        }

        info!("Logged {} audit events", processed_events.len());

        Ok(())
    }

    /// Query events
    pub async fn query(&self, query: &AuditQuery) -> Result<QueryResult> {
        self.backend.query(query).await
    }

    /// Get events by IDs
    pub async fn get_by_ids(&self, ids: &[uuid::Uuid]) -> Result<Vec<AuditEvent>> {
        self.backend.get_by_ids(ids).await
    }

    /// Get recent events
    pub async fn recent(&self, limit: usize) -> Result<Vec<AuditEvent>> {
        let query = AuditQuery::new().with_pagination(0, limit);
        let result = self.backend.query(&query).await?;
        Ok(result.events)
    }

    /// Get events for a specific stack
    pub async fn for_stack(&self, stack: &str, limit: usize) -> Result<Vec<AuditEvent>> {
        let query = AuditQuery::new()
            .with_stack(stack)
            .with_pagination(0, limit);
        let result = self.backend.query(&query).await?;
        Ok(result.events)
    }

    /// Get events for a specific actor
    pub async fn for_actor(&self, actor_id: &str, limit: usize) -> Result<Vec<AuditEvent>> {
        let query = AuditQuery::new()
            .with_actor(actor_id)
            .with_pagination(0, limit);
        let result = self.backend.query(&query).await?;
        Ok(result.events)
    }

    /// Verify the integrity of the audit chain
    pub async fn verify_chain(&self) -> Result<ChainVerificationResult> {
        // Get all events in order
        let query = AuditQuery::new()
            .ascending()
            .with_pagination(0, 100_000);
        let result = self.backend.query(&query).await?;

        HashChain::verify_chain(&result.events)
    }

    /// Get chain statistics
    pub fn chain_stats(&self) -> ChainStats {
        let chain = self.chain.read().unwrap();
        let metadata = self.chain_metadata.read().unwrap();

        ChainStats {
            event_count: chain.count(),
            last_hash: chain.last_hash().map(|s| s.to_string()),
            first_timestamp: metadata.first_timestamp,
            last_timestamp: metadata.last_timestamp,
        }
    }

    /// Flush any buffered events
    pub async fn flush(&self) -> Result<()> {
        self.backend.flush().await?;

        // Save chain metadata
        if self.chain_enabled {
            let metadata = self.chain_metadata.read().unwrap().clone();
            self.backend.save_chain_metadata(&metadata).await?;
        }

        Ok(())
    }

    /// Close the logger
    pub async fn close(&self) -> Result<()> {
        self.flush().await?;
        self.backend.close().await
    }

    /// Get the backend name
    pub fn backend_name(&self) -> &str {
        self.backend.name()
    }

    /// Health check
    pub async fn health_check(&self) -> Result<bool> {
        self.backend.health_check().await
    }
}

/// Chain statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChainStats {
    /// Number of events in the chain
    pub event_count: u64,
    /// Last hash in the chain
    pub last_hash: Option<String>,
    /// Timestamp of first event
    pub first_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    /// Timestamp of last event
    pub last_timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

/// Builder for AuditLogger
pub struct AuditLoggerBuilder {
    backend: Option<Arc<dyn AuditBackend>>,
    chain_enabled: bool,
    default_project: Option<String>,
    default_organization: Option<String>,
}

impl AuditLoggerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            backend: None,
            chain_enabled: true,
            default_project: None,
            default_organization: None,
        }
    }

    /// Set the backend
    pub fn backend(mut self, backend: impl AuditBackend + 'static) -> Self {
        self.backend = Some(Arc::new(backend));
        self
    }

    /// Set the backend from Arc
    pub fn backend_arc(mut self, backend: Arc<dyn AuditBackend>) -> Self {
        self.backend = Some(backend);
        self
    }

    /// Disable hash chaining
    pub fn without_chain(mut self) -> Self {
        self.chain_enabled = false;
        self
    }

    /// Set default project
    pub fn default_project(mut self, project: impl Into<String>) -> Self {
        self.default_project = Some(project.into());
        self
    }

    /// Set default organization
    pub fn default_organization(mut self, org_id: impl Into<String>) -> Self {
        self.default_organization = Some(org_id.into());
        self
    }

    /// Build the logger
    pub fn build(self) -> Result<AuditLogger> {
        let backend = self
            .backend
            .ok_or_else(|| crate::AuditError::BackendUnavailable("No backend configured".into()))?;

        let mut logger = AuditLogger::from_arc(backend);
        
        if !self.chain_enabled {
            logger = logger.without_chain();
        }
        if let Some(project) = self.default_project {
            logger = logger.with_default_project(project);
        }
        if let Some(org_id) = self.default_organization {
            logger = logger.with_default_organization(org_id);
        }

        Ok(logger)
    }
}

impl Default for AuditLoggerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MemoryBackend;
    use crate::event::{Actor, EventType};

    #[tokio::test]
    async fn test_audit_logger() {
        let backend = MemoryBackend::new();
        let logger = AuditLogger::new(backend)
            .with_default_project("test-project");

        let event = AuditEvent::deployment_started("dev", "user@example.com");
        logger.log(event).await.unwrap();

        let recent = logger.recent(10).await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].project, Some("test-project".to_string()));
    }

    #[tokio::test]
    async fn test_audit_logger_chain() {
        let backend = MemoryBackend::new();
        let logger = AuditLogger::new(backend);

        for i in 0..5 {
            let event = AuditEvent::new(
                EventType::DeploymentStarted,
                Actor::user("user1"),
                format!("Event {}", i),
            );
            logger.log(event).await.unwrap();
        }

        let stats = logger.chain_stats();
        assert_eq!(stats.event_count, 5);
        assert!(stats.last_hash.is_some());
    }

    #[tokio::test]
    async fn test_chain_verification() {
        let backend = MemoryBackend::new();
        let logger = AuditLogger::new(backend);

        for i in 0..3 {
            let event = AuditEvent::new(
                EventType::DeploymentStarted,
                Actor::user("user1"),
                format!("Event {}", i),
            );
            logger.log(event).await.unwrap();
        }

        let result = logger.verify_chain().await.unwrap();
        assert!(result.valid);
        assert_eq!(result.verified_count, 3);
    }
}
