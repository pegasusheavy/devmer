//! Hash chain for audit log integrity

use crate::event::AuditEvent;
use crate::Result;
use sha2::{Digest, Sha256};
use serde::{Deserialize, Serialize};

/// A hash chain for ensuring audit log integrity
#[derive(Debug)]
pub struct HashChain {
    /// Last hash in the chain
    last_hash: Option<String>,
    /// Number of events in the chain
    count: u64,
}

impl HashChain {
    /// Create a new hash chain
    pub fn new() -> Self {
        Self {
            last_hash: None,
            count: 0,
        }
    }

    /// Create a hash chain from a known last hash
    pub fn from_last_hash(hash: String, count: u64) -> Self {
        Self {
            last_hash: Some(hash),
            count,
        }
    }

    /// Get the last hash
    pub fn last_hash(&self) -> Option<&str> {
        self.last_hash.as_deref()
    }

    /// Get the event count
    pub fn count(&self) -> u64 {
        self.count
    }

    /// Add an event to the chain
    pub fn add_event(&mut self, event: &mut AuditEvent) {
        // Set the previous hash
        event.previous_hash = self.last_hash.clone();

        // Compute this event's hash
        let hash = self.compute_hash(event);
        event.hash = Some(hash.clone());

        // Update chain state
        self.last_hash = Some(hash);
        self.count += 1;
    }

    /// Compute hash for an event
    fn compute_hash(&self, event: &AuditEvent) -> String {
        let mut hasher = Sha256::new();

        // Include event ID
        hasher.update(event.id.to_string().as_bytes());

        // Include timestamp
        hasher.update(event.timestamp.to_rfc3339().as_bytes());

        // Include event type
        hasher.update(format!("{:?}", event.event_type).as_bytes());

        // Include actor ID
        hasher.update(event.actor.id.as_bytes());

        // Include description
        hasher.update(event.description.as_bytes());

        // Include previous hash if present
        if let Some(ref prev) = event.previous_hash {
            hasher.update(prev.as_bytes());
        }

        // Include metadata as sorted JSON
        if !event.metadata.is_empty() {
            let mut keys: Vec<_> = event.metadata.keys().collect();
            keys.sort();
            for key in keys {
                hasher.update(key.as_bytes());
                if let Ok(value) = serde_json::to_string(&event.metadata[key]) {
                    hasher.update(value.as_bytes());
                }
            }
        }

        hex::encode(hasher.finalize())
    }

    /// Verify a chain of events
    pub fn verify_chain(events: &[AuditEvent]) -> Result<ChainVerificationResult> {
        if events.is_empty() {
            return Ok(ChainVerificationResult {
                valid: true,
                verified_count: 0,
                first_invalid: None,
                details: vec![],
            });
        }

        let mut result = ChainVerificationResult {
            valid: true,
            verified_count: 0,
            first_invalid: None,
            details: vec![],
        };

        let mut expected_previous: Option<String> = None;

        for (i, event) in events.iter().enumerate() {
            // Check previous hash matches
            if event.previous_hash != expected_previous {
                result.valid = false;
                if result.first_invalid.is_none() {
                    result.first_invalid = Some(event.id.to_string());
                }
                result.details.push(ChainVerificationDetail {
                    event_id: event.id.to_string(),
                    index: i,
                    error: format!(
                        "Previous hash mismatch: expected {:?}, got {:?}",
                        expected_previous, event.previous_hash
                    ),
                });
            }

            // Verify event hash
            let mut temp_chain = HashChain::new();
            temp_chain.last_hash = event.previous_hash.clone();
            let mut temp_event = event.clone();
            temp_event.hash = None;
            let computed_hash = temp_chain.compute_hash(&temp_event);

            if event.hash.as_deref() != Some(&computed_hash) {
                result.valid = false;
                if result.first_invalid.is_none() {
                    result.first_invalid = Some(event.id.to_string());
                }
                result.details.push(ChainVerificationDetail {
                    event_id: event.id.to_string(),
                    index: i,
                    error: format!(
                        "Hash mismatch: computed {}, stored {:?}",
                        computed_hash, event.hash
                    ),
                });
            }

            expected_previous = event.hash.clone();
            result.verified_count += 1;
        }

        Ok(result)
    }
}

impl Default for HashChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of chain verification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerificationResult {
    /// Whether the entire chain is valid
    pub valid: bool,
    /// Number of events verified
    pub verified_count: usize,
    /// First invalid event ID (if any)
    pub first_invalid: Option<String>,
    /// Detailed verification results
    pub details: Vec<ChainVerificationDetail>,
}

/// Detail about a chain verification issue
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainVerificationDetail {
    /// Event ID with the issue
    pub event_id: String,
    /// Index in the chain
    pub index: usize,
    /// Error description
    pub error: String,
}

/// An event with chain information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainedEvent {
    /// The audit event
    pub event: AuditEvent,
    /// Position in the chain
    pub chain_index: u64,
    /// Hash of this event
    pub hash: String,
    /// Hash of the previous event
    pub previous_hash: Option<String>,
}

impl ChainedEvent {
    /// Create from an event that has been added to a chain
    pub fn from_event(event: AuditEvent, chain_index: u64) -> Option<Self> {
        let hash = event.hash.clone()?;
        Some(Self {
            previous_hash: event.previous_hash.clone(),
            event,
            chain_index,
            hash,
        })
    }
}

/// Chain metadata stored alongside audit logs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMetadata {
    /// Chain ID (for multi-chain setups)
    pub chain_id: String,
    /// Number of events in the chain
    pub event_count: u64,
    /// Hash of the first event
    pub first_hash: Option<String>,
    /// Hash of the last event
    pub last_hash: Option<String>,
    /// Timestamp of first event
    pub first_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    /// Timestamp of last event
    pub last_timestamp: Option<chrono::DateTime<chrono::Utc>>,
    /// When metadata was last updated
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl ChainMetadata {
    /// Create new chain metadata
    pub fn new(chain_id: impl Into<String>) -> Self {
        Self {
            chain_id: chain_id.into(),
            event_count: 0,
            first_hash: None,
            last_hash: None,
            first_timestamp: None,
            last_timestamp: None,
            updated_at: chrono::Utc::now(),
        }
    }

    /// Update metadata with a new event
    pub fn update(&mut self, event: &AuditEvent) {
        if self.first_hash.is_none() {
            self.first_hash = event.hash.clone();
            self.first_timestamp = Some(event.timestamp);
        }
        self.last_hash = event.hash.clone();
        self.last_timestamp = Some(event.timestamp);
        self.event_count += 1;
        self.updated_at = chrono::Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Actor, EventType};

    #[test]
    fn test_hash_chain() {
        let mut chain = HashChain::new();

        let mut event1 = AuditEvent::new(
            EventType::DeploymentStarted,
            Actor::user("user1"),
            "Test event 1",
        );
        chain.add_event(&mut event1);

        assert!(event1.hash.is_some());
        assert!(event1.previous_hash.is_none());
        assert_eq!(chain.count(), 1);

        let mut event2 = AuditEvent::new(
            EventType::DeploymentCompleted,
            Actor::user("user1"),
            "Test event 2",
        );
        chain.add_event(&mut event2);

        assert!(event2.hash.is_some());
        assert_eq!(event2.previous_hash, event1.hash);
        assert_eq!(chain.count(), 2);
    }

    #[test]
    fn test_chain_verification() {
        let mut chain = HashChain::new();

        let mut events = vec![];
        for i in 0..5 {
            let mut event = AuditEvent::new(
                EventType::DeploymentStarted,
                Actor::user("user1"),
                format!("Event {}", i),
            );
            chain.add_event(&mut event);
            events.push(event);
        }

        let result = HashChain::verify_chain(&events).unwrap();
        assert!(result.valid);
        assert_eq!(result.verified_count, 5);
        assert!(result.first_invalid.is_none());
    }

    #[test]
    fn test_tamper_detection() {
        let mut chain = HashChain::new();

        let mut events = vec![];
        for i in 0..3 {
            let mut event = AuditEvent::new(
                EventType::DeploymentStarted,
                Actor::user("user1"),
                format!("Event {}", i),
            );
            chain.add_event(&mut event);
            events.push(event);
        }

        // Tamper with an event
        events[1].description = "Tampered description".to_string();

        let result = HashChain::verify_chain(&events).unwrap();
        assert!(!result.valid);
        assert_eq!(result.first_invalid, Some(events[1].id.to_string()));
    }
}
