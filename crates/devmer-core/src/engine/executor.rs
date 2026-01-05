//! Deployment executor

use super::plan::{DeploymentPlan, PlannedResource, ResourceOperation};
use super::step::StepResult;
use crate::provider::Provider;
use crate::resource::{Resource, ResourceState};
use crate::state::StackState;
use crate::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use tracing::info;

/// Event emitted during deployment execution
#[derive(Debug, Clone)]
pub enum ExecutionEvent {
    /// Deployment started
    Started {
        stack: String,
        total_operations: usize,
    },
    /// Starting a resource operation
    StepStarted {
        index: usize,
        urn: String,
        operation: String,
    },
    /// Resource operation completed
    StepCompleted {
        index: usize,
        urn: String,
        success: bool,
        duration_ms: u64,
    },
    /// Resource operation failed
    StepFailed {
        index: usize,
        urn: String,
        error: String,
    },
    /// Deployment completed
    Completed {
        success: bool,
        duration_secs: f64,
        created: usize,
        updated: usize,
        deleted: usize,
        errors: Vec<String>,
    },
}

/// Deployment executor
pub struct DeploymentExecutor {
    /// Providers by name
    providers: HashMap<String, Arc<dyn Provider>>,

    /// Event channel sender
    event_tx: Option<mpsc::UnboundedSender<ExecutionEvent>>,
}

impl DeploymentExecutor {
    /// Create a new deployment executor
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
            event_tx: None,
        }
    }

    /// Register a provider
    pub fn register_provider(&mut self, name: impl Into<String>, provider: Arc<dyn Provider>) {
        self.providers.insert(name.into(), provider);
    }

    /// Set event channel for progress updates
    pub fn with_events(mut self, tx: mpsc::UnboundedSender<ExecutionEvent>) -> Self {
        self.event_tx = Some(tx);
        self
    }

    /// Send an event
    fn emit(&self, event: ExecutionEvent) {
        if let Some(ref tx) = self.event_tx {
            let _ = tx.send(event);
        }
    }

    /// Execute a deployment plan
    pub async fn execute(
        &self,
        plan: &DeploymentPlan,
        current_state: Option<StackState>,
    ) -> Result<ExecutionResult> {
        let start_time = Instant::now();
        let mut state = current_state.unwrap_or_else(|| StackState::new(&plan.stack));
        let mut errors = Vec::new();
        let mut created = 0;
        let mut updated = 0;
        let mut deleted = 0;

        self.emit(ExecutionEvent::Started {
            stack: plan.stack.clone(),
            total_operations: plan.total_operations(),
        });

        // Execute each step
        for (index, step) in plan.steps.iter().enumerate() {
            if step.operation == ResourceOperation::Same {
                continue;
            }

            self.emit(ExecutionEvent::StepStarted {
                index,
                urn: step.urn.to_string(),
                operation: format!("{:?}", step.operation),
            });

            let step_start = Instant::now();
            let result = self.execute_step(step, &state).await;
            let duration_ms = step_start.elapsed().as_millis() as u64;

            match result {
                Ok(step_result) => {
                    if step_result.success {
                        // Update state based on operation
                        match step.operation {
                            ResourceOperation::Create | ResourceOperation::Replace => {
                                let mut resource = Resource::new(
                                    step.urn.stack(),
                                    step.resource_type.parse().unwrap_or_default(),
                                    &step.name,
                                    step.new_inputs.clone(),
                                );
                                resource.urn = step.urn.clone();
                                resource.outputs = step_result.outputs.unwrap_or_default();
                                resource.state = ResourceState::Created;
                                state.add_or_update_resource(resource);
                                created += 1;
                            }
                            ResourceOperation::Update => {
                                if let Some(resource) = state.get_resource_mut(step.urn.as_str()) {
                                    resource.inputs = step.new_inputs.clone();
                                    if let Some(outputs) = step_result.outputs {
                                        resource.outputs = outputs;
                                    }
                                }
                                updated += 1;
                            }
                            ResourceOperation::Delete => {
                                state.remove_resource(step.urn.as_str());
                                deleted += 1;
                            }
                            _ => {}
                        }

                        self.emit(ExecutionEvent::StepCompleted {
                            index,
                            urn: step.urn.to_string(),
                            success: true,
                            duration_ms,
                        });
                    } else {
                        let error = step_result.error.unwrap_or_else(|| "Unknown error".to_string());
                        errors.push(format!("{}: {}", step.urn, error));

                        self.emit(ExecutionEvent::StepFailed {
                            index,
                            urn: step.urn.to_string(),
                            error: error.clone(),
                        });
                    }
                }
                Err(e) => {
                    let error = e.to_string();
                    errors.push(format!("{}: {}", step.urn, error));

                    self.emit(ExecutionEvent::StepFailed {
                        index,
                        urn: step.urn.to_string(),
                        error,
                    });
                }
            }
        }

        let duration_secs = start_time.elapsed().as_secs_f64();
        let success = errors.is_empty();

        self.emit(ExecutionEvent::Completed {
            success,
            duration_secs,
            created,
            updated,
            deleted,
            errors: errors.clone(),
        });

        Ok(ExecutionResult {
            success,
            state,
            created,
            updated,
            deleted,
            errors,
            duration_secs,
        })
    }

    /// Execute a single step
    async fn execute_step(
        &self,
        step: &PlannedResource,
        _current_state: &StackState,
    ) -> Result<StepResult> {
        let start = Instant::now();

        // Get the provider for this resource type
        let provider_name = step
            .resource_type
            .split(':')
            .next()
            .unwrap_or("unknown");

        let provider = match self.providers.get(provider_name) {
            Some(p) => p,
            None => {
                return Ok(StepResult::failure(
                    step.urn.clone(),
                    format!("Provider '{}' not found", provider_name),
                    start.elapsed().as_millis() as u64,
                ));
            }
        };

        // Build resource for the operation
        let mut resource = Resource::new(
            step.urn.stack(),
            step.resource_type.parse().unwrap_or_default(),
            &step.name,
            step.new_inputs.clone(),
        );
        resource.urn = step.urn.clone();
        resource.outputs = step.old_outputs.clone().unwrap_or_default();
        resource.state = ResourceState::Created;

        // Execute the operation
        let result = match step.operation {
            ResourceOperation::Create => {
                info!(urn = %step.urn, "Creating resource");
                provider.create(&resource).await
            }
            ResourceOperation::Update => {
                info!(urn = %step.urn, "Updating resource");
                provider.update(&resource, step.new_inputs.clone()).await
            }
            ResourceOperation::Replace => {
                info!(urn = %step.urn, "Replacing resource");
                // Delete then create
                let delete_result = provider.delete(&resource).await;
                if let Err(e) = delete_result {
                    return Ok(StepResult::failure(
                        step.urn.clone(),
                        format!("Failed to delete for replacement: {}", e),
                        start.elapsed().as_millis() as u64,
                    ));
                }
                provider.create(&resource).await
            }
            ResourceOperation::Delete => {
                info!(urn = %step.urn, "Deleting resource");
                provider.delete(&resource).await
            }
            ResourceOperation::Read => {
                info!(urn = %step.urn, "Reading resource");
                provider.read(&resource).await
            }
            ResourceOperation::Same => {
                return Ok(StepResult::success(
                    step.urn.clone(),
                    step.old_outputs.clone().unwrap_or_default(),
                    0,
                ));
            }
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(op_result) => {
                if op_result.success {
                    let outputs = op_result
                        .resource
                        .map(|r| r.outputs)
                        .unwrap_or_default();
                    let mut step_result = StepResult::success(step.urn.clone(), outputs, duration_ms);
                    for warning in op_result.warnings {
                        step_result = step_result.with_warning(warning);
                    }
                    Ok(step_result)
                } else {
                    Ok(StepResult::failure(
                        step.urn.clone(),
                        op_result.error.unwrap_or_else(|| "Unknown error".to_string()),
                        duration_ms,
                    ))
                }
            }
            Err(e) => Ok(StepResult::failure(
                step.urn.clone(),
                e.to_string(),
                duration_ms,
            )),
        }
    }

    /// Refresh resources from cloud state
    pub async fn refresh(&self, state: &mut StackState) -> Result<RefreshResult> {
        let start = Instant::now();
        let mut refreshed = 0;
        let mut drift_detected = 0;
        let mut errors = Vec::new();

        // Get URNs first to avoid borrowing issues
        let urns: Vec<String> = state
            .resources()
            .map(|r| r.urn.as_str().to_string())
            .collect();

        for urn_str in urns {
            let resource = match state.get_resource(&urn_str) {
                Some(r) => r.clone(),
                None => continue,
            };
            let provider_name = resource.resource_type.provider();

            let provider = match self.providers.get(provider_name) {
                Some(p) => p,
                None => {
                    errors.push(format!(
                        "{}: Provider '{}' not found",
                        resource.urn, provider_name
                    ));
                    continue;
                }
            };

            match provider.read(&resource).await {
                Ok(result) => {
                    if result.success {
                        if let Some(new_resource) = result.resource {
                            // Check for drift
                            if new_resource.outputs != resource.outputs {
                                drift_detected += 1;
                            }
                            // Update the resource in state
                            if let Some(r) = state.get_resource_mut(&urn_str) {
                                r.outputs = new_resource.outputs;
                            }
                        }
                        refreshed += 1;
                    } else {
                        errors.push(format!(
                            "{}: {}",
                            resource.urn,
                            result.error.unwrap_or_else(|| "Read failed".to_string())
                        ));
                    }
                }
                Err(e) => {
                    errors.push(format!("{}: {}", resource.urn, e));
                }
            }
        }

        Ok(RefreshResult {
            success: errors.is_empty(),
            refreshed,
            drift_detected,
            errors,
            duration_secs: start.elapsed().as_secs_f64(),
        })
    }
}

impl Default for DeploymentExecutor {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of deployment execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Whether execution succeeded
    pub success: bool,

    /// Final state after execution
    pub state: StackState,

    /// Resources created
    pub created: usize,

    /// Resources updated
    pub updated: usize,

    /// Resources deleted
    pub deleted: usize,

    /// Errors encountered
    pub errors: Vec<String>,

    /// Total duration in seconds
    pub duration_secs: f64,
}

/// Result of refresh operation
#[derive(Debug, Clone)]
pub struct RefreshResult {
    /// Whether refresh succeeded
    pub success: bool,

    /// Resources refreshed
    pub refreshed: usize,

    /// Resources with drift
    pub drift_detected: usize,

    /// Errors encountered
    pub errors: Vec<String>,

    /// Total duration in seconds
    pub duration_secs: f64,
}
