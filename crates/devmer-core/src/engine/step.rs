//! Execution step types

use crate::resource::Urn;
use crate::types::PropertyValues;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Status of an execution step
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    /// Step is pending execution
    Pending,
    /// Step is currently running
    Running,
    /// Step completed successfully
    Succeeded,
    /// Step failed
    Failed,
    /// Step was skipped
    Skipped,
    /// Step was cancelled
    Cancelled,
}

impl StepStatus {
    /// Check if the step has finished
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            StepStatus::Succeeded | StepStatus::Failed | StepStatus::Skipped | StepStatus::Cancelled
        )
    }

    /// Check if the step was successful
    pub fn is_success(&self) -> bool {
        matches!(self, StepStatus::Succeeded | StepStatus::Skipped)
    }
}

/// An execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionStep {
    /// Step index
    pub index: usize,

    /// Resource URN
    pub urn: Urn,

    /// Resource type
    pub resource_type: String,

    /// Resource name
    pub name: String,

    /// Operation being performed
    pub operation: String,

    /// Current status
    pub status: StepStatus,

    /// Start time (Unix timestamp ms)
    pub started_at: Option<i64>,

    /// End time (Unix timestamp ms)
    pub ended_at: Option<i64>,

    /// Error message if failed
    pub error: Option<String>,
}

impl ExecutionStep {
    /// Get the duration of this step
    pub fn duration(&self) -> Option<Duration> {
        match (self.started_at, self.ended_at) {
            (Some(start), Some(end)) => Some(Duration::from_millis((end - start) as u64)),
            _ => None,
        }
    }
}

/// Result of an execution step
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    /// URN of the resource
    pub urn: Urn,

    /// Whether the step succeeded
    pub success: bool,

    /// Outputs produced (for create/update)
    pub outputs: Option<PropertyValues>,

    /// Error message if failed
    pub error: Option<String>,

    /// Warnings generated
    #[serde(default)]
    pub warnings: Vec<String>,

    /// Duration in milliseconds
    pub duration_ms: u64,
}

impl StepResult {
    /// Create a success result
    pub fn success(urn: Urn, outputs: PropertyValues, duration_ms: u64) -> Self {
        Self {
            urn,
            success: true,
            outputs: Some(outputs),
            error: None,
            warnings: vec![],
            duration_ms,
        }
    }

    /// Create a failure result
    pub fn failure(urn: Urn, error: impl Into<String>, duration_ms: u64) -> Self {
        Self {
            urn,
            success: false,
            outputs: None,
            error: Some(error.into()),
            warnings: vec![],
            duration_ms,
        }
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }
}
