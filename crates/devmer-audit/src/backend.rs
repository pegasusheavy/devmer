//! Audit storage backends

use crate::chain::{ChainMetadata, HashChain};
use crate::event::AuditEvent;
use crate::query::{AuditQuery, QueryResult};
use crate::Result;
use async_trait::async_trait;
use chrono::{DateTime, Utc, Datelike};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use tokio::fs;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Trait for audit storage backends
#[async_trait]
pub trait AuditBackend: Send + Sync {
    /// Write an event to storage
    async fn write(&self, event: &AuditEvent) -> Result<()>;

    /// Write multiple events
    async fn write_batch(&self, events: &[AuditEvent]) -> Result<()> {
        for event in events {
            self.write(event).await?;
        }
        Ok(())
    }

    /// Query events
    async fn query(&self, query: &AuditQuery) -> Result<QueryResult>;

    /// Get events by IDs
    async fn get_by_ids(&self, ids: &[uuid::Uuid]) -> Result<Vec<AuditEvent>>;

    /// Get chain metadata
    async fn get_chain_metadata(&self) -> Result<Option<ChainMetadata>>;

    /// Save chain metadata
    async fn save_chain_metadata(&self, metadata: &ChainMetadata) -> Result<()>;

    /// Flush any buffered data
    async fn flush(&self) -> Result<()>;

    /// Close the backend
    async fn close(&self) -> Result<()>;

    /// Get backend name
    fn name(&self) -> &str;

    /// Check if backend is healthy
    async fn health_check(&self) -> Result<bool>;
}

// =============================================================================
// Memory Backend (for testing)
// =============================================================================

/// In-memory audit backend for testing
pub struct MemoryBackend {
    events: Arc<RwLock<Vec<AuditEvent>>>,
    chain_metadata: Arc<RwLock<Option<ChainMetadata>>>,
    max_events: usize,
}

impl MemoryBackend {
    /// Create a new memory backend
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            chain_metadata: Arc::new(RwLock::new(None)),
            max_events: 100_000,
        }
    }

    /// Create with a maximum event count
    pub fn with_max_events(max_events: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            chain_metadata: Arc::new(RwLock::new(None)),
            max_events,
        }
    }

    /// Get all events (for testing)
    pub fn get_all_events(&self) -> Vec<AuditEvent> {
        self.events.read().unwrap().clone()
    }

    /// Clear all events
    pub fn clear(&self) {
        self.events.write().unwrap().clear();
    }
}

impl Default for MemoryBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuditBackend for MemoryBackend {
    async fn write(&self, event: &AuditEvent) -> Result<()> {
        let mut events = self.events.write().unwrap();
        if events.len() >= self.max_events {
            events.remove(0);
        }
        events.push(event.clone());
        Ok(())
    }

    async fn query(&self, query: &AuditQuery) -> Result<QueryResult> {
        let events = self.events.read().unwrap();
        let filtered: Vec<AuditEvent> = events
            .iter()
            .filter(|e| query.matches(e))
            .cloned()
            .collect();

        let total = filtered.len();
        let skip = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(100);

        let results: Vec<AuditEvent> = filtered
            .into_iter()
            .skip(skip)
            .take(limit)
            .collect();

        Ok(QueryResult {
            events: results,
            total,
            offset: skip,
            limit,
            has_more: total > skip + limit,
        })
    }

    async fn get_by_ids(&self, ids: &[uuid::Uuid]) -> Result<Vec<AuditEvent>> {
        let events = self.events.read().unwrap();
        Ok(events
            .iter()
            .filter(|e| ids.contains(&e.id))
            .cloned()
            .collect())
    }

    async fn get_chain_metadata(&self) -> Result<Option<ChainMetadata>> {
        Ok(self.chain_metadata.read().unwrap().clone())
    }

    async fn save_chain_metadata(&self, metadata: &ChainMetadata) -> Result<()> {
        *self.chain_metadata.write().unwrap() = Some(metadata.clone());
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        Ok(())
    }

    fn name(&self) -> &str {
        "memory"
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(true)
    }
}

// =============================================================================
// File Backend
// =============================================================================

/// File-based audit backend
pub struct FileBackend {
    /// Base directory for audit logs
    base_dir: PathBuf,
    /// Current log file path
    current_file: Arc<RwLock<PathBuf>>,
    /// Buffer for events
    buffer: Arc<RwLock<VecDeque<AuditEvent>>>,
    /// Buffer size before flush
    buffer_size: usize,
    /// Hash chain
    chain: Arc<RwLock<HashChain>>,
    /// Rotate files daily
    rotate_daily: bool,
}

impl FileBackend {
    /// Create a new file backend
    pub fn new(base_dir: impl AsRef<Path>) -> Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();

        // Create directory if it doesn't exist
        std::fs::create_dir_all(&base_dir)?;

        let current_file = Self::get_log_file_path(&base_dir);

        Ok(Self {
            base_dir,
            current_file: Arc::new(RwLock::new(current_file)),
            buffer: Arc::new(RwLock::new(VecDeque::new())),
            buffer_size: 100,
            chain: Arc::new(RwLock::new(HashChain::new())),
            rotate_daily: true,
        })
    }

    /// Set buffer size
    pub fn with_buffer_size(mut self, size: usize) -> Self {
        self.buffer_size = size;
        self
    }

    /// Disable daily rotation
    pub fn without_rotation(mut self) -> Self {
        self.rotate_daily = false;
        self
    }

    /// Get the log file path for today
    fn get_log_file_path(base_dir: &Path) -> PathBuf {
        let now = Utc::now();
        base_dir.join(format!(
            "audit-{:04}-{:02}-{:02}.jsonl",
            now.year(),
            now.month(),
            now.day()
        ))
    }

    /// Get all log files
    fn get_log_files(&self) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        for entry in std::fs::read_dir(&self.base_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                files.push(path);
            }
        }
        files.sort();
        Ok(files)
    }

    /// Rotate log file if needed
    fn maybe_rotate(&self) {
        if !self.rotate_daily {
            return;
        }

        let expected_file = Self::get_log_file_path(&self.base_dir);
        let mut current = self.current_file.write().unwrap();
        if *current != expected_file {
            *current = expected_file;
        }
    }

    /// Write buffered events to file
    async fn write_buffer(&self) -> Result<()> {
        let events: Vec<AuditEvent> = {
            let mut buffer = self.buffer.write().unwrap();
            buffer.drain(..).collect()
        };

        if events.is_empty() {
            return Ok(());
        }

        self.maybe_rotate();

        let file_path = self.current_file.read().unwrap().clone();
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)
            .await?;

        for event in &events {
            let line = serde_json::to_string(event)?;
            file.write_all(line.as_bytes()).await?;
            file.write_all(b"\n").await?;
        }

        file.flush().await?;

        Ok(())
    }

    /// Load chain metadata from file
    async fn load_chain_metadata(&self) -> Result<Option<ChainMetadata>> {
        let metadata_path = self.base_dir.join("chain-metadata.json");
        if !metadata_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&metadata_path).await?;
        let metadata: ChainMetadata = serde_json::from_str(&content)?;
        Ok(Some(metadata))
    }
}

#[async_trait]
impl AuditBackend for FileBackend {
    async fn write(&self, event: &AuditEvent) -> Result<()> {
        let should_flush = {
            let mut buffer = self.buffer.write().unwrap();
            buffer.push_back(event.clone());
            buffer.len() >= self.buffer_size
        };

        if should_flush {
            self.write_buffer().await?;
        }

        Ok(())
    }

    async fn write_batch(&self, events: &[AuditEvent]) -> Result<()> {
        {
            let mut buffer = self.buffer.write().unwrap();
            for event in events {
                buffer.push_back(event.clone());
            }
        }
        self.write_buffer().await
    }

    async fn query(&self, query: &AuditQuery) -> Result<QueryResult> {
        // Flush buffer first
        self.write_buffer().await?;

        let files = self.get_log_files()?;
        let mut all_events = Vec::new();

        for file_path in files {
            // Skip files outside the time range if specified
            if let Some(ref time_range) = query.time_range {
                // Extract date from filename
                if let Some(filename) = file_path.file_name().and_then(|f| f.to_str()) {
                    if let Some(date_str) = filename.strip_prefix("audit-").and_then(|s| s.strip_suffix(".jsonl")) {
                        if let Ok(file_date) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                            let file_start = file_date.and_hms_opt(0, 0, 0).unwrap();
                            let file_end = file_date.and_hms_opt(23, 59, 59).unwrap();
                            let file_start_utc = DateTime::<Utc>::from_naive_utc_and_offset(file_start, Utc);
                            let file_end_utc = DateTime::<Utc>::from_naive_utc_and_offset(file_end, Utc);

                            if let Some(ref start) = time_range.start {
                                if file_end_utc < *start {
                                    continue;
                                }
                            }
                            if let Some(ref end) = time_range.end {
                                if file_start_utc > *end {
                                    continue;
                                }
                            }
                        }
                    }
                }
            }

            let file = fs::File::open(&file_path).await?;
            let reader = BufReader::new(file);
            let mut lines = reader.lines();

            while let Some(line) = lines.next_line().await? {
                if let Ok(event) = serde_json::from_str::<AuditEvent>(&line) {
                    if query.matches(&event) {
                        all_events.push(event);
                    }
                }
            }
        }

        // Sort by timestamp (newest first by default)
        all_events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        let total = all_events.len();
        let skip = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(100);

        let results: Vec<AuditEvent> = all_events
            .into_iter()
            .skip(skip)
            .take(limit)
            .collect();

        Ok(QueryResult {
            events: results,
            total,
            offset: skip,
            limit,
            has_more: total > skip + limit,
        })
    }

    async fn get_by_ids(&self, ids: &[uuid::Uuid]) -> Result<Vec<AuditEvent>> {
        // Flush buffer first
        self.write_buffer().await?;

        let files = self.get_log_files()?;
        let mut found = Vec::new();
        let id_set: std::collections::HashSet<_> = ids.iter().collect();

        for file_path in files {
            let file = fs::File::open(&file_path).await?;
            let reader = BufReader::new(file);
            let mut lines = reader.lines();

            while let Some(line) = lines.next_line().await? {
                if let Ok(event) = serde_json::from_str::<AuditEvent>(&line) {
                    if id_set.contains(&event.id) {
                        found.push(event);
                        if found.len() == ids.len() {
                            return Ok(found);
                        }
                    }
                }
            }
        }

        Ok(found)
    }

    async fn get_chain_metadata(&self) -> Result<Option<ChainMetadata>> {
        self.load_chain_metadata().await
    }

    async fn save_chain_metadata(&self, metadata: &ChainMetadata) -> Result<()> {
        let metadata_path = self.base_dir.join("chain-metadata.json");
        let content = serde_json::to_string_pretty(metadata)?;
        fs::write(&metadata_path, content).await?;
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        self.write_buffer().await
    }

    async fn close(&self) -> Result<()> {
        self.write_buffer().await
    }

    fn name(&self) -> &str {
        "file"
    }

    async fn health_check(&self) -> Result<bool> {
        // Check if we can write to the directory
        let test_path = self.base_dir.join(".health_check");
        match fs::write(&test_path, b"ok").await {
            Ok(_) => {
                let _ = fs::remove_file(&test_path).await;
                Ok(true)
            }
            Err(_) => Ok(false),
        }
    }
}

// =============================================================================
// Rotating File Backend
// =============================================================================

/// Configuration for rotating file backend
#[derive(Debug, Clone)]
pub struct RotatingFileConfig {
    /// Base directory for logs
    pub base_dir: PathBuf,
    /// Maximum file size before rotation (bytes)
    pub max_file_size: u64,
    /// Maximum number of files to keep
    pub max_files: usize,
    /// Compress rotated files
    pub compress: bool,
}

impl Default for RotatingFileConfig {
    fn default() -> Self {
        Self {
            base_dir: PathBuf::from(".devmer/audit"),
            max_file_size: 100 * 1024 * 1024, // 100 MB
            max_files: 30,
            compress: true,
        }
    }
}

// =============================================================================
// Multi Backend (write to multiple backends)
// =============================================================================

/// Backend that writes to multiple backends
pub struct MultiBackend {
    backends: Vec<Arc<dyn AuditBackend>>,
}

impl MultiBackend {
    /// Create a new multi-backend
    pub fn new(backends: Vec<Arc<dyn AuditBackend>>) -> Self {
        Self { backends }
    }
}

#[async_trait]
impl AuditBackend for MultiBackend {
    async fn write(&self, event: &AuditEvent) -> Result<()> {
        for backend in &self.backends {
            backend.write(event).await?;
        }
        Ok(())
    }

    async fn write_batch(&self, events: &[AuditEvent]) -> Result<()> {
        for backend in &self.backends {
            backend.write_batch(events).await?;
        }
        Ok(())
    }

    async fn query(&self, query: &AuditQuery) -> Result<QueryResult> {
        // Query from first backend only
        if let Some(backend) = self.backends.first() {
            backend.query(query).await
        } else {
            Ok(QueryResult::empty())
        }
    }

    async fn get_by_ids(&self, ids: &[uuid::Uuid]) -> Result<Vec<AuditEvent>> {
        if let Some(backend) = self.backends.first() {
            backend.get_by_ids(ids).await
        } else {
            Ok(vec![])
        }
    }

    async fn get_chain_metadata(&self) -> Result<Option<ChainMetadata>> {
        if let Some(backend) = self.backends.first() {
            backend.get_chain_metadata().await
        } else {
            Ok(None)
        }
    }

    async fn save_chain_metadata(&self, metadata: &ChainMetadata) -> Result<()> {
        for backend in &self.backends {
            backend.save_chain_metadata(metadata).await?;
        }
        Ok(())
    }

    async fn flush(&self) -> Result<()> {
        for backend in &self.backends {
            backend.flush().await?;
        }
        Ok(())
    }

    async fn close(&self) -> Result<()> {
        for backend in &self.backends {
            backend.close().await?;
        }
        Ok(())
    }

    fn name(&self) -> &str {
        "multi"
    }

    async fn health_check(&self) -> Result<bool> {
        for backend in &self.backends {
            if !backend.health_check().await? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Actor, EventType};

    #[tokio::test]
    async fn test_memory_backend() {
        let backend = MemoryBackend::new();

        let event = AuditEvent::new(
            EventType::DeploymentStarted,
            Actor::user("user1"),
            "Test deployment",
        );

        backend.write(&event).await.unwrap();

        let query = AuditQuery::new();
        let result = backend.query(&query).await.unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.events[0].id, event.id);
    }

    #[tokio::test]
    async fn test_file_backend() {
        let temp_dir = tempfile::tempdir().unwrap();
        let backend = FileBackend::new(temp_dir.path()).unwrap();

        let event = AuditEvent::new(
            EventType::DeploymentStarted,
            Actor::user("user1"),
            "Test deployment",
        );

        backend.write(&event).await.unwrap();
        backend.flush().await.unwrap();

        let query = AuditQuery::new();
        let result = backend.query(&query).await.unwrap();

        assert_eq!(result.total, 1);
        assert_eq!(result.events[0].id, event.id);
    }

    #[tokio::test]
    async fn test_memory_backend_max_events() {
        let backend = MemoryBackend::with_max_events(5);

        for i in 0..10 {
            let event = AuditEvent::new(
                EventType::DeploymentStarted,
                Actor::user("user1"),
                format!("Event {}", i),
            );
            backend.write(&event).await.unwrap();
        }

        let events = backend.get_all_events();
        assert_eq!(events.len(), 5);
    }
}
