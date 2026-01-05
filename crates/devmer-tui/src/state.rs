//! Application state management

use crate::event::{KeyBindings, OperationStatus, OperationType};
use crate::theme::Theme;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use uuid::Uuid;

/// Main application state
#[derive(Debug)]
pub struct AppState {
    /// Current view
    pub current_view: View,
    /// Previous view (for back navigation)
    pub previous_view: Option<View>,
    /// Theme
    pub theme: Theme,
    /// Key bindings
    pub key_bindings: KeyBindings,
    /// Project name
    pub project_name: Option<String>,
    /// Current stack
    pub current_stack: Option<String>,
    /// Available stacks
    pub stacks: Vec<StackInfo>,
    /// Resources in current stack
    pub resources: Vec<ResourceInfo>,
    /// Deployment state (if deployment is running)
    pub deployment: Option<DeploymentState>,
    /// Preview/plan state
    pub preview: Option<PreviewState>,
    /// Selected resource index
    pub selected_resource: usize,
    /// Selected stack index
    pub selected_stack: usize,
    /// Active tab index
    pub active_tab: usize,
    /// Show help overlay
    pub show_help: bool,
    /// Search/filter query
    pub search_query: String,
    /// Is in search mode
    pub search_mode: bool,
    /// Status message
    pub status_message: Option<StatusMessage>,
    /// Spinner frame
    pub spinner_frame: usize,
    /// Is loading
    pub is_loading: bool,
    /// Scroll offset for resource details
    pub scroll_offset: u16,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current_view: View::Dashboard,
            previous_view: None,
            theme: Theme::dark(),
            key_bindings: KeyBindings::default(),
            project_name: None,
            current_stack: None,
            stacks: Vec::new(),
            resources: Vec::new(),
            deployment: None,
            preview: None,
            selected_resource: 0,
            selected_stack: 0,
            active_tab: 0,
            show_help: false,
            search_query: String::new(),
            search_mode: false,
            status_message: None,
            spinner_frame: 0,
            is_loading: false,
            scroll_offset: 0,
        }
    }
}

impl AppState {
    /// Create a new app state
    pub fn new() -> Self {
        Self::default()
    }

    /// Set project name
    pub fn with_project(mut self, name: impl Into<String>) -> Self {
        self.project_name = Some(name.into());
        self
    }

    /// Set current stack
    pub fn with_stack(mut self, stack: impl Into<String>) -> Self {
        self.current_stack = Some(stack.into());
        self
    }

    /// Navigate to a view
    pub fn navigate_to(&mut self, view: View) {
        self.previous_view = Some(self.current_view);
        self.current_view = view;
        self.scroll_offset = 0;
    }

    /// Go back to previous view
    pub fn go_back(&mut self) {
        if let Some(prev) = self.previous_view.take() {
            self.current_view = prev;
            self.scroll_offset = 0;
        }
    }

    /// Move selection up
    pub fn select_up(&mut self) {
        match self.current_view {
            View::Resources | View::ResourceDetails(_) => {
                if self.selected_resource > 0 {
                    self.selected_resource -= 1;
                }
            }
            View::Stacks => {
                if self.selected_stack > 0 {
                    self.selected_stack -= 1;
                }
            }
            _ => {}
        }
    }

    /// Move selection down
    pub fn select_down(&mut self) {
        match self.current_view {
            View::Resources | View::ResourceDetails(_) => {
                if !self.resources.is_empty() && self.selected_resource < self.resources.len() - 1 {
                    self.selected_resource += 1;
                }
            }
            View::Stacks => {
                if !self.stacks.is_empty() && self.selected_stack < self.stacks.len() - 1 {
                    self.selected_stack += 1;
                }
            }
            _ => {}
        }
    }

    /// Get currently selected resource
    pub fn selected_resource_info(&self) -> Option<&ResourceInfo> {
        self.resources.get(self.selected_resource)
    }

    /// Get currently selected stack
    pub fn selected_stack_info(&self) -> Option<&StackInfo> {
        self.stacks.get(self.selected_stack)
    }

    /// Set status message
    pub fn set_status(&mut self, message: impl Into<String>, level: StatusLevel) {
        self.status_message = Some(StatusMessage {
            text: message.into(),
            level,
            timestamp: Utc::now(),
        });
    }

    /// Clear status message
    pub fn clear_status(&mut self) {
        self.status_message = None;
    }

    /// Update spinner
    pub fn tick_spinner(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % 10;
    }

    /// Filter resources by search query
    pub fn filtered_resources(&self) -> Vec<&ResourceInfo> {
        if self.search_query.is_empty() {
            return self.resources.iter().collect();
        }

        let query = self.search_query.to_lowercase();
        self.resources
            .iter()
            .filter(|r| {
                r.name.to_lowercase().contains(&query)
                    || r.resource_type.to_lowercase().contains(&query)
                    || r.urn.to_lowercase().contains(&query)
            })
            .collect()
    }

    /// Next tab
    pub fn next_tab(&mut self) {
        self.active_tab = (self.active_tab + 1) % self.tab_count();
    }

    /// Previous tab
    pub fn prev_tab(&mut self) {
        if self.active_tab == 0 {
            self.active_tab = self.tab_count() - 1;
        } else {
            self.active_tab -= 1;
        }
    }

    /// Get tab count for current view
    fn tab_count(&self) -> usize {
        match self.current_view {
            View::Dashboard => 4,      // Overview, Resources, Stacks, Activity
            View::ResourceDetails(_) => 3, // Properties, Inputs, Outputs
            _ => 1,
        }
    }

    /// Scroll up
    pub fn scroll_up(&mut self) {
        if self.scroll_offset > 0 {
            self.scroll_offset -= 1;
        }
    }

    /// Scroll down
    pub fn scroll_down(&mut self) {
        self.scroll_offset += 1;
    }
}

/// Application views
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// Main dashboard
    Dashboard,
    /// Stack list
    Stacks,
    /// Resource browser
    Resources,
    /// Resource details
    ResourceDetails(usize),
    /// Deployment progress
    Deployment,
    /// Preview/plan view
    Preview,
    /// State browser
    State,
    /// Settings
    Settings,
}

/// Stack information
#[derive(Debug, Clone)]
pub struct StackInfo {
    pub name: String,
    pub resource_count: usize,
    pub last_updated: Option<DateTime<Utc>>,
    pub is_current: bool,
}

/// Resource information
#[derive(Debug, Clone)]
pub struct ResourceInfo {
    pub id: Uuid,
    pub urn: String,
    pub name: String,
    pub resource_type: String,
    pub provider: String,
    pub parent: Option<String>,
    pub inputs: HashMap<String, serde_json::Value>,
    pub outputs: HashMap<String, serde_json::Value>,
    pub status: ResourceStatus,
}

/// Resource status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceStatus {
    Ok,
    Pending,
    Creating,
    Updating,
    Deleting,
    Failed,
    Drifted,
}

impl std::fmt::Display for ResourceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Ok => write!(f, "OK"),
            Self::Pending => write!(f, "Pending"),
            Self::Creating => write!(f, "Creating"),
            Self::Updating => write!(f, "Updating"),
            Self::Deleting => write!(f, "Deleting"),
            Self::Failed => write!(f, "Failed"),
            Self::Drifted => write!(f, "Drifted"),
        }
    }
}

/// Deployment state
#[derive(Debug, Clone)]
pub struct DeploymentState {
    pub id: Uuid,
    pub started_at: DateTime<Utc>,
    pub operations: Vec<OperationState>,
    pub current_operation: usize,
    pub total_operations: usize,
    pub status: DeploymentStatus,
    pub logs: Vec<LogEntry>,
}

impl DeploymentState {
    /// Create a new deployment state
    pub fn new(total_operations: usize) -> Self {
        Self {
            id: Uuid::new_v4(),
            started_at: Utc::now(),
            operations: Vec::new(),
            current_operation: 0,
            total_operations,
            status: DeploymentStatus::InProgress,
            logs: Vec::new(),
        }
    }

    /// Get progress as percentage
    pub fn progress(&self) -> f64 {
        if self.total_operations == 0 {
            return 100.0;
        }
        (self.operations.iter().filter(|o| o.status != OperationStatus::Pending).count() as f64
            / self.total_operations as f64)
            * 100.0
    }

    /// Add a log entry
    pub fn add_log(&mut self, level: LogLevel, message: impl Into<String>) {
        self.logs.push(LogEntry {
            timestamp: Utc::now(),
            level,
            message: message.into(),
        });
    }
}

/// Deployment status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentStatus {
    InProgress,
    Succeeded,
    Failed,
    Cancelled,
}

/// Operation state in deployment
#[derive(Debug, Clone)]
pub struct OperationState {
    pub urn: String,
    pub name: String,
    pub resource_type: String,
    pub operation: OperationType,
    pub status: OperationStatus,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

/// Preview/plan state
#[derive(Debug, Clone)]
pub struct PreviewState {
    pub changes: Vec<PlannedChange>,
    pub summary: ChangeSummary,
}

/// A planned change
#[derive(Debug, Clone)]
pub struct PlannedChange {
    pub urn: String,
    pub name: String,
    pub resource_type: String,
    pub operation: OperationType,
    pub diff: Option<ResourceDiff>,
}

/// Resource diff
#[derive(Debug, Clone)]
pub struct ResourceDiff {
    pub property_diffs: Vec<PropertyDiff>,
}

/// Property diff
#[derive(Debug, Clone)]
pub struct PropertyDiff {
    pub path: String,
    pub old_value: Option<serde_json::Value>,
    pub new_value: Option<serde_json::Value>,
}

/// Change summary
#[derive(Debug, Clone, Default)]
pub struct ChangeSummary {
    pub creates: usize,
    pub updates: usize,
    pub deletes: usize,
    pub replaces: usize,
    pub unchanged: usize,
}

impl ChangeSummary {
    pub fn total(&self) -> usize {
        self.creates + self.updates + self.deletes + self.replaces
    }
}

/// Log entry
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: LogLevel,
    pub message: String,
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

/// Status message
#[derive(Debug, Clone)]
pub struct StatusMessage {
    pub text: String,
    pub level: StatusLevel,
    pub timestamp: DateTime<Utc>,
}

/// Status level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_navigation() {
        let mut state = AppState::new();
        assert_eq!(state.current_view, View::Dashboard);

        state.navigate_to(View::Resources);
        assert_eq!(state.current_view, View::Resources);
        assert_eq!(state.previous_view, Some(View::Dashboard));

        state.go_back();
        assert_eq!(state.current_view, View::Dashboard);
    }

    #[test]
    fn test_resource_selection() {
        let mut state = AppState::new();
        state.resources = vec![
            ResourceInfo {
                id: Uuid::new_v4(),
                urn: "urn:devmer:stack::aws:s3:Bucket::bucket1".to_string(),
                name: "bucket1".to_string(),
                resource_type: "aws:s3:Bucket".to_string(),
                provider: "aws".to_string(),
                parent: None,
                inputs: HashMap::new(),
                outputs: HashMap::new(),
                status: ResourceStatus::Ok,
            },
            ResourceInfo {
                id: Uuid::new_v4(),
                urn: "urn:devmer:stack::aws:s3:Bucket::bucket2".to_string(),
                name: "bucket2".to_string(),
                resource_type: "aws:s3:Bucket".to_string(),
                provider: "aws".to_string(),
                parent: None,
                inputs: HashMap::new(),
                outputs: HashMap::new(),
                status: ResourceStatus::Ok,
            },
        ];
        state.current_view = View::Resources;

        assert_eq!(state.selected_resource, 0);

        state.select_down();
        assert_eq!(state.selected_resource, 1);

        state.select_down();
        assert_eq!(state.selected_resource, 1); // At end

        state.select_up();
        assert_eq!(state.selected_resource, 0);
    }

    #[test]
    fn test_deployment_progress() {
        let mut deployment = DeploymentState::new(10);
        assert_eq!(deployment.progress(), 0.0);

        deployment.operations.push(OperationState {
            urn: "urn:test".to_string(),
            name: "test".to_string(),
            resource_type: "test".to_string(),
            operation: OperationType::Create,
            status: OperationStatus::Succeeded,
            started_at: Some(Utc::now()),
            completed_at: Some(Utc::now()),
            error: None,
        });

        assert_eq!(deployment.progress(), 10.0);
    }
}
