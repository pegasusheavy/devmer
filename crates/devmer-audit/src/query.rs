//! Audit event querying

use crate::event::{AuditEvent, EventSeverity, EventType, EventOutcome};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Query for filtering audit events
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuditQuery {
    /// Time range to query
    pub time_range: Option<TimeRange>,

    /// Filter by event types
    pub event_types: Option<Vec<EventType>>,

    /// Filter by event categories
    pub categories: Option<Vec<String>>,

    /// Filter by severity (minimum)
    pub min_severity: Option<EventSeverity>,

    /// Filter by outcome
    pub outcomes: Option<Vec<EventOutcome>>,

    /// Filter by actor ID
    pub actor_id: Option<String>,

    /// Filter by actor type
    pub actor_type: Option<String>,

    /// Filter by stack
    pub stack: Option<String>,

    /// Filter by project
    pub project: Option<String>,

    /// Filter by organization
    pub organization_id: Option<String>,

    /// Filter by resource type
    pub resource_type: Option<String>,

    /// Filter by resource name
    pub resource_name: Option<String>,

    /// Full-text search in description/message
    pub search_text: Option<String>,

    /// Filter by request ID
    pub request_id: Option<String>,

    /// Filter by session ID
    pub session_id: Option<String>,

    /// Only security events
    pub security_only: bool,

    /// Only compliance events
    pub compliance_only: bool,

    /// Pagination offset
    pub offset: Option<usize>,

    /// Pagination limit
    pub limit: Option<usize>,

    /// Sort order (true = ascending, false = descending)
    pub sort_ascending: bool,
}

impl AuditQuery {
    /// Create a new empty query
    pub fn new() -> Self {
        Self::default()
    }

    /// Set time range
    pub fn with_time_range(mut self, range: TimeRange) -> Self {
        self.time_range = Some(range);
        self
    }

    /// Filter by event types
    pub fn with_event_types(mut self, types: Vec<EventType>) -> Self {
        self.event_types = Some(types);
        self
    }

    /// Filter by single event type
    pub fn with_event_type(mut self, event_type: EventType) -> Self {
        self.event_types = Some(vec![event_type]);
        self
    }

    /// Filter by categories
    pub fn with_categories(mut self, categories: Vec<String>) -> Self {
        self.categories = Some(categories);
        self
    }

    /// Filter by minimum severity
    pub fn with_min_severity(mut self, severity: EventSeverity) -> Self {
        self.min_severity = Some(severity);
        self
    }

    /// Filter by outcomes
    pub fn with_outcomes(mut self, outcomes: Vec<EventOutcome>) -> Self {
        self.outcomes = Some(outcomes);
        self
    }

    /// Filter by actor
    pub fn with_actor(mut self, actor_id: impl Into<String>) -> Self {
        self.actor_id = Some(actor_id.into());
        self
    }

    /// Filter by stack
    pub fn with_stack(mut self, stack: impl Into<String>) -> Self {
        self.stack = Some(stack.into());
        self
    }

    /// Filter by project
    pub fn with_project(mut self, project: impl Into<String>) -> Self {
        self.project = Some(project.into());
        self
    }

    /// Filter by organization
    pub fn with_organization(mut self, org_id: impl Into<String>) -> Self {
        self.organization_id = Some(org_id.into());
        self
    }

    /// Filter by resource type
    pub fn with_resource_type(mut self, resource_type: impl Into<String>) -> Self {
        self.resource_type = Some(resource_type.into());
        self
    }

    /// Search in text fields
    pub fn with_search(mut self, text: impl Into<String>) -> Self {
        self.search_text = Some(text.into());
        self
    }

    /// Only security events
    pub fn security_events_only(mut self) -> Self {
        self.security_only = true;
        self
    }

    /// Only compliance events
    pub fn compliance_events_only(mut self) -> Self {
        self.compliance_only = true;
        self
    }

    /// Set pagination
    pub fn with_pagination(mut self, offset: usize, limit: usize) -> Self {
        self.offset = Some(offset);
        self.limit = Some(limit);
        self
    }

    /// Set sort order
    pub fn ascending(mut self) -> Self {
        self.sort_ascending = true;
        self
    }

    /// Check if an event matches this query
    pub fn matches(&self, event: &AuditEvent) -> bool {
        // Time range filter
        if let Some(ref range) = self.time_range {
            if let Some(ref start) = range.start {
                if event.timestamp < *start {
                    return false;
                }
            }
            if let Some(ref end) = range.end {
                if event.timestamp > *end {
                    return false;
                }
            }
        }

        // Event type filter
        if let Some(ref types) = self.event_types {
            if !types.contains(&event.event_type) {
                return false;
            }
        }

        // Category filter
        if let Some(ref categories) = self.categories {
            let event_category = event.event_type.category();
            if !categories.iter().any(|c| c == event_category) {
                return false;
            }
        }

        // Severity filter
        if let Some(min) = self.min_severity {
            if event.severity.level() < min.level() {
                return false;
            }
        }

        // Outcome filter
        if let Some(ref outcomes) = self.outcomes {
            if !outcomes.contains(&event.outcome) {
                return false;
            }
        }

        // Actor filter
        if let Some(ref actor_id) = self.actor_id {
            if event.actor.id != *actor_id {
                return false;
            }
        }

        // Stack filter
        if let Some(ref stack) = self.stack {
            if event.stack.as_deref() != Some(stack) {
                return false;
            }
        }

        // Project filter
        if let Some(ref project) = self.project {
            if event.project.as_deref() != Some(project) {
                return false;
            }
        }

        // Organization filter
        if let Some(ref org_id) = self.organization_id {
            if event.organization_id.as_deref() != Some(org_id) {
                return false;
            }
        }

        // Resource type filter
        if let Some(ref resource_type) = self.resource_type {
            if let Some(ref resource) = event.resource {
                if resource.resource_type != *resource_type {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Resource name filter
        if let Some(ref resource_name) = self.resource_name {
            if let Some(ref resource) = event.resource {
                if resource.name != *resource_name {
                    return false;
                }
            } else {
                return false;
            }
        }

        // Text search
        if let Some(ref search) = self.search_text {
            let search_lower = search.to_lowercase();
            let matches = event.description.to_lowercase().contains(&search_lower)
                || event
                    .message
                    .as_ref()
                    .map(|m| m.to_lowercase().contains(&search_lower))
                    .unwrap_or(false);
            if !matches {
                return false;
            }
        }

        // Request ID filter
        if let Some(ref request_id) = self.request_id {
            if event.request_id.as_deref() != Some(request_id) {
                return false;
            }
        }

        // Session ID filter
        if let Some(ref session_id) = self.session_id {
            if event.session_id.as_deref() != Some(session_id) {
                return false;
            }
        }

        // Security events filter
        if self.security_only && !event.event_type.is_security_event() {
            return false;
        }

        // Compliance events filter
        if self.compliance_only && !event.event_type.is_compliance_event() {
            return false;
        }

        true
    }
}

/// Time range for queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    /// Start time (inclusive)
    pub start: Option<DateTime<Utc>>,
    /// End time (inclusive)
    pub end: Option<DateTime<Utc>>,
}

impl TimeRange {
    /// Create a new time range
    pub fn new() -> Self {
        Self {
            start: None,
            end: None,
        }
    }

    /// Set start time
    pub fn from(mut self, start: DateTime<Utc>) -> Self {
        self.start = Some(start);
        self
    }

    /// Set end time
    pub fn to(mut self, end: DateTime<Utc>) -> Self {
        self.end = Some(end);
        self
    }

    /// Last N hours
    pub fn last_hours(hours: i64) -> Self {
        Self {
            start: Some(Utc::now() - chrono::Duration::hours(hours)),
            end: None,
        }
    }

    /// Last N days
    pub fn last_days(days: i64) -> Self {
        Self {
            start: Some(Utc::now() - chrono::Duration::days(days)),
            end: None,
        }
    }

    /// Last N weeks
    pub fn last_weeks(weeks: i64) -> Self {
        Self {
            start: Some(Utc::now() - chrono::Duration::weeks(weeks)),
            end: None,
        }
    }

    /// Last 24 hours
    pub fn last_24_hours() -> Self {
        Self::last_hours(24)
    }

    /// Last 7 days
    pub fn last_7_days() -> Self {
        Self::last_days(7)
    }

    /// Last 30 days
    pub fn last_30_days() -> Self {
        Self::last_days(30)
    }

    /// Today
    pub fn today() -> Self {
        let now = Utc::now();
        let start_of_day = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        Self {
            start: Some(DateTime::from_naive_utc_and_offset(start_of_day, Utc)),
            end: None,
        }
    }
}

impl Default for TimeRange {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of an audit query
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    /// Matching events
    pub events: Vec<AuditEvent>,
    /// Total count of matching events
    pub total: usize,
    /// Offset used
    pub offset: usize,
    /// Limit used
    pub limit: usize,
    /// Whether there are more results
    pub has_more: bool,
}

impl QueryResult {
    /// Create an empty result
    pub fn empty() -> Self {
        Self {
            events: vec![],
            total: 0,
            offset: 0,
            limit: 0,
            has_more: false,
        }
    }

    /// Check if result is empty
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }
}

/// Aggregation query for statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationQuery {
    /// Base query filters
    pub query: AuditQuery,
    /// Group by field
    pub group_by: GroupBy,
    /// Time bucket (for time-based grouping)
    pub time_bucket: Option<TimeBucket>,
}

/// Group by options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GroupBy {
    EventType,
    Category,
    Severity,
    Outcome,
    Actor,
    Stack,
    Project,
    Time,
}

/// Time bucket for aggregation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TimeBucket {
    Hour,
    Day,
    Week,
    Month,
}

/// Aggregation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregationResult {
    /// Grouped counts
    pub groups: Vec<GroupCount>,
    /// Total events
    pub total: usize,
}

/// Count for a group
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupCount {
    /// Group key
    pub key: String,
    /// Event count
    pub count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::Actor;

    #[test]
    fn test_query_matching() {
        let event = AuditEvent::new(
            EventType::DeploymentStarted,
            Actor::user("user1"),
            "Test deployment",
        )
        .with_stack("dev")
        .with_severity(EventSeverity::Info);

        // Should match empty query
        let query = AuditQuery::new();
        assert!(query.matches(&event));

        // Should match event type
        let query = AuditQuery::new().with_event_type(EventType::DeploymentStarted);
        assert!(query.matches(&event));

        // Should not match different event type
        let query = AuditQuery::new().with_event_type(EventType::DeploymentCompleted);
        assert!(!query.matches(&event));

        // Should match stack
        let query = AuditQuery::new().with_stack("dev");
        assert!(query.matches(&event));

        // Should not match different stack
        let query = AuditQuery::new().with_stack("prod");
        assert!(!query.matches(&event));

        // Should match severity
        let query = AuditQuery::new().with_min_severity(EventSeverity::Info);
        assert!(query.matches(&event));

        // Should not match higher severity
        let query = AuditQuery::new().with_min_severity(EventSeverity::Warning);
        assert!(!query.matches(&event));
    }

    #[test]
    fn test_time_range() {
        let range = TimeRange::last_24_hours();
        assert!(range.start.is_some());
        assert!(range.end.is_none());

        let range = TimeRange::last_7_days();
        let start = range.start.unwrap();
        let diff = Utc::now() - start;
        assert!(diff.num_days() >= 6 && diff.num_days() <= 7);
    }

    #[test]
    fn test_text_search() {
        let event = AuditEvent::new(
            EventType::DeploymentStarted,
            Actor::user("user1"),
            "Started deployment for production",
        );

        let query = AuditQuery::new().with_search("production");
        assert!(query.matches(&event));

        let query = AuditQuery::new().with_search("staging");
        assert!(!query.matches(&event));

        // Case insensitive
        let query = AuditQuery::new().with_search("PRODUCTION");
        assert!(query.matches(&event));
    }
}
