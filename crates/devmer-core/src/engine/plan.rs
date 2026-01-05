//! Deployment planning

use crate::graph::ResourceGraph;
use crate::provider::DiffKind;
use crate::resource::{Resource, Urn};
use crate::state::StackState;
use crate::types::PropertyValues;
use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Operation to perform on a resource
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceOperation {
    /// Create a new resource
    Create,
    /// Update an existing resource
    Update,
    /// Replace a resource (delete + create)
    Replace,
    /// Delete a resource
    Delete,
    /// Read/refresh a resource
    Read,
    /// No change needed
    Same,
}

impl ResourceOperation {
    /// Check if this operation modifies state
    pub fn is_mutating(&self) -> bool {
        matches!(
            self,
            ResourceOperation::Create
                | ResourceOperation::Update
                | ResourceOperation::Replace
                | ResourceOperation::Delete
        )
    }
}

/// A planned operation on a resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedResource {
    /// Resource URN
    pub urn: Urn,

    /// Resource type string
    pub resource_type: String,

    /// Logical name
    pub name: String,

    /// Operation to perform
    pub operation: ResourceOperation,

    /// Current/old inputs (if updating/replacing)
    pub old_inputs: Option<PropertyValues>,

    /// New desired inputs
    pub new_inputs: PropertyValues,

    /// Current/old outputs (if updating/replacing)
    pub old_outputs: Option<PropertyValues>,

    /// Property diffs
    pub diffs: Vec<PlannedDiff>,

    /// Whether this requires replacement
    pub requires_replacement: bool,

    /// Dependencies (URNs)
    pub depends_on: Vec<Urn>,
}

/// A diff for a single property
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedDiff {
    /// Property path
    pub path: String,

    /// Kind of diff
    pub kind: DiffKind,

    /// Old value (for display)
    pub old_value: Option<String>,

    /// New value (for display)
    pub new_value: Option<String>,
}

/// A complete deployment plan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentPlan {
    /// Stack name
    pub stack: String,

    /// Resources to operate on, in dependency order
    pub steps: Vec<PlannedResource>,

    /// Summary counts
    pub summary: PlanSummary,
}

/// Summary of planned operations
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlanSummary {
    /// Resources to create
    pub creates: usize,
    /// Resources to update
    pub updates: usize,
    /// Resources to replace
    pub replaces: usize,
    /// Resources to delete
    pub deletes: usize,
    /// Resources unchanged
    pub same: usize,
}

impl DeploymentPlan {
    /// Check if there are any changes
    pub fn has_changes(&self) -> bool {
        self.summary.creates > 0
            || self.summary.updates > 0
            || self.summary.replaces > 0
            || self.summary.deletes > 0
    }

    /// Get total number of operations
    pub fn total_operations(&self) -> usize {
        self.summary.creates + self.summary.updates + self.summary.replaces + self.summary.deletes
    }

    /// Get steps by operation type
    pub fn steps_by_operation(&self, op: ResourceOperation) -> Vec<&PlannedResource> {
        self.steps.iter().filter(|s| s.operation == op).collect()
    }
}

/// Builder for deployment plans
pub struct PlanBuilder {
    stack: String,
    current_state: Option<StackState>,
    desired_resources: Vec<Resource>,
}

impl PlanBuilder {
    /// Create a new plan builder
    pub fn new(stack: impl Into<String>) -> Self {
        Self {
            stack: stack.into(),
            current_state: None,
            desired_resources: vec![],
        }
    }

    /// Set the current state
    pub fn with_current_state(mut self, state: StackState) -> Self {
        self.current_state = Some(state);
        self
    }

    /// Set the desired resources
    pub fn with_desired_resources(mut self, resources: Vec<Resource>) -> Self {
        self.desired_resources = resources;
        self
    }

    /// Build the deployment plan
    pub fn build(self) -> Result<DeploymentPlan> {
        let mut steps = Vec::new();
        let mut summary = PlanSummary::default();

        // Index current resources by URN
        let current_resources: HashMap<String, &Resource> = self
            .current_state
            .as_ref()
            .map(|s| {
                s.resources()
                    .map(|r| (r.urn.as_str().to_string(), r))
                    .collect()
            })
            .unwrap_or_default();

        // Index desired resources by URN
        let desired_by_urn: HashMap<&Urn, &Resource> =
            self.desired_resources.iter().map(|r| (&r.urn, r)).collect();

        // Build resource graph for dependency ordering
        let graph = ResourceGraph::build_from_resources(self.desired_resources.clone())?;
        let creation_order = graph.topological_sort()?;

        // Plan creates and updates (in dependency order)
        for urn in &creation_order {
            let desired = match desired_by_urn.get(&urn) {
                Some(r) => *r,
                None => continue,
            };

            let urn_str = urn.as_str().to_string();
            let (operation, old_inputs, old_outputs, diffs, requires_replacement) =
                if let Some(current) = current_resources.get(&urn_str) {
                    // Resource exists - check for updates
                    let (op, diffs, replace) = diff_resources(current, desired);
                    (
                        op,
                        Some(current.inputs.clone()),
                        Some(current.outputs.clone()),
                        diffs,
                        replace,
                    )
                } else {
                    // New resource
                    (
                        ResourceOperation::Create,
                        None,
                        None,
                        vec![],
                        false,
                    )
                };

            match operation {
                ResourceOperation::Create => summary.creates += 1,
                ResourceOperation::Update => summary.updates += 1,
                ResourceOperation::Replace => summary.replaces += 1,
                ResourceOperation::Same => summary.same += 1,
                _ => {}
            }

            steps.push(PlannedResource {
                urn: desired.urn.clone(),
                resource_type: desired.resource_type.to_string(),
                name: desired.name.clone(),
                operation,
                old_inputs,
                new_inputs: desired.inputs.clone(),
                old_outputs,
                diffs,
                requires_replacement,
                depends_on: desired.options.depends_on.clone(),
            });
        }

        // Plan deletes (resources in current state but not in desired state)
        // These need to be in reverse dependency order
        let deletion_order = graph.reverse_topological_sort()?;
        for urn in &deletion_order {
            let urn_str = urn.as_str().to_string();
            if !desired_by_urn.contains_key(&urn) {
                if let Some(current) = current_resources.get(&urn_str) {
                    summary.deletes += 1;
                    steps.push(PlannedResource {
                        urn: current.urn.clone(),
                        resource_type: current.resource_type.to_string(),
                        name: current.name.clone(),
                        operation: ResourceOperation::Delete,
                        old_inputs: Some(current.inputs.clone()),
                        new_inputs: PropertyValues::new(),
                        old_outputs: Some(current.outputs.clone()),
                        diffs: vec![],
                        requires_replacement: false,
                        depends_on: vec![],
                    });
                }
            }
        }

        Ok(DeploymentPlan {
            stack: self.stack,
            steps,
            summary,
        })
    }
}

/// Compare two resources and determine what operation is needed
fn diff_resources(
    current: &Resource,
    desired: &Resource,
) -> (ResourceOperation, Vec<PlannedDiff>, bool) {
    let mut diffs = Vec::new();
    let mut has_changes = false;
    let requires_replacement = false;

    // Compare inputs
    for (key, new_value) in &desired.inputs {
        if let Some(old_value) = current.inputs.get(key) {
            if old_value != new_value {
                has_changes = true;
                diffs.push(PlannedDiff {
                    path: key.clone(),
                    kind: DiffKind::Update,
                    old_value: Some(format!("{:?}", old_value)),
                    new_value: Some(format!("{:?}", new_value)),
                });
            }
        } else {
            has_changes = true;
            diffs.push(PlannedDiff {
                path: key.clone(),
                kind: DiffKind::Add,
                old_value: None,
                new_value: Some(format!("{:?}", new_value)),
            });
        }
    }

    // Check for removed inputs
    for key in current.inputs.keys() {
        if !desired.inputs.contains_key(key) {
            has_changes = true;
            diffs.push(PlannedDiff {
                path: key.clone(),
                kind: DiffKind::Delete,
                old_value: Some(format!("{:?}", current.inputs.get(key))),
                new_value: None,
            });
        }
    }

    let operation = if requires_replacement {
        ResourceOperation::Replace
    } else if has_changes {
        ResourceOperation::Update
    } else {
        ResourceOperation::Same
    };

    (operation, diffs, requires_replacement)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::ResourceType;

    #[test]
    fn test_plan_builder_empty() {
        let plan = PlanBuilder::new("test-stack")
            .with_desired_resources(vec![])
            .build()
            .unwrap();

        assert!(!plan.has_changes());
        assert_eq!(plan.summary.creates, 0);
    }

    #[test]
    fn test_plan_builder_create() {
        let resource = Resource::new(
            "test",
            ResourceType::new("aws", "s3", "Bucket"),
            "my-bucket",
            PropertyValues::new(),
        );

        let plan = PlanBuilder::new("test-stack")
            .with_desired_resources(vec![resource])
            .build()
            .unwrap();

        assert!(plan.has_changes());
        assert_eq!(plan.summary.creates, 1);
    }
}
