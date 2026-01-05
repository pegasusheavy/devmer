//! Resource dependency graph

use crate::resource::{Resource, Urn};
use crate::{DevmerError, Result};
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::HashMap;

/// A directed graph of resource dependencies
#[derive(Debug)]
pub struct ResourceGraph {
    /// The underlying petgraph
    graph: DiGraph<Resource, DependencyEdge>,

    /// Map from URN to node index
    urn_to_index: HashMap<String, NodeIndex>,
}

/// Edge type representing a dependency relationship
#[derive(Debug, Clone)]
pub struct DependencyEdge {
    /// Kind of dependency
    pub kind: DependencyKind,
}

/// Kind of dependency between resources
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyKind {
    /// Explicit dependency via depends_on
    Explicit,
    /// Implicit dependency via property reference
    PropertyRef,
    /// Parent-child relationship (component resources)
    Parent,
}

impl ResourceGraph {
    /// Create a new empty resource graph
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            urn_to_index: HashMap::new(),
        }
    }

    /// Add a resource to the graph
    pub fn add_resource(&mut self, resource: Resource) -> NodeIndex {
        let urn = resource.urn.as_str().to_string();

        if let Some(&index) = self.urn_to_index.get(&urn) {
            // Update existing resource
            self.graph[index] = resource;
            index
        } else {
            // Add new resource
            let index = self.graph.add_node(resource);
            self.urn_to_index.insert(urn, index);
            index
        }
    }

    /// Get a resource by URN
    pub fn get_resource(&self, urn: &Urn) -> Option<&Resource> {
        self.urn_to_index
            .get(urn.as_str())
            .map(|&idx| &self.graph[idx])
    }

    /// Get a mutable resource by URN
    pub fn get_resource_mut(&mut self, urn: &Urn) -> Option<&mut Resource> {
        self.urn_to_index
            .get(urn.as_str())
            .copied()
            .map(move |idx| &mut self.graph[idx])
    }

    /// Add a dependency edge between resources
    pub fn add_dependency(
        &mut self,
        from_urn: &Urn,
        to_urn: &Urn,
        kind: DependencyKind,
    ) -> Result<()> {
        let from_idx = self
            .urn_to_index
            .get(from_urn.as_str())
            .copied()
            .ok_or_else(|| DevmerError::resource_not_found(from_urn.as_str()))?;

        let to_idx = self
            .urn_to_index
            .get(to_urn.as_str())
            .copied()
            .ok_or_else(|| DevmerError::resource_not_found(to_urn.as_str()))?;

        // Edge goes from dependent to dependency
        // (from depends on to)
        self.graph.add_edge(from_idx, to_idx, DependencyEdge { kind });

        Ok(())
    }

    /// Check if adding a dependency would create a cycle
    pub fn would_create_cycle(&self, from_urn: &Urn, to_urn: &Urn) -> bool {
        let from_idx = match self.urn_to_index.get(from_urn.as_str()) {
            Some(&idx) => idx,
            None => return false,
        };

        let to_idx = match self.urn_to_index.get(to_urn.as_str()) {
            Some(&idx) => idx,
            None => return false,
        };

        // Check if there's already a path from to_idx to from_idx
        petgraph::algo::has_path_connecting(&self.graph, to_idx, from_idx, None)
    }

    /// Get topological order for resource operations (creation order)
    pub fn creation_order(&self) -> Result<Vec<&Resource>> {
        let sorted = toposort(&self.graph, None).map_err(|cycle| {
            let resource = &self.graph[cycle.node_id()];
            DevmerError::DependencyCycle(format!(
                "Cycle detected involving resource: {}",
                resource.urn
            ))
        })?;

        // Reverse because toposort gives us leaves first (things with no dependencies)
        // which is actually what we want for creation (create dependencies first)
        Ok(sorted.into_iter().rev().map(|idx| &self.graph[idx]).collect())
    }

    /// Get deletion order (reverse of creation order)
    pub fn deletion_order(&self) -> Result<Vec<&Resource>> {
        let sorted = toposort(&self.graph, None).map_err(|cycle| {
            let resource = &self.graph[cycle.node_id()];
            DevmerError::DependencyCycle(format!(
                "Cycle detected involving resource: {}",
                resource.urn
            ))
        })?;

        // Don't reverse - delete dependents first
        Ok(sorted.into_iter().map(|idx| &self.graph[idx]).collect())
    }

    /// Get topological sort as URNs (creation order)
    pub fn topological_sort(&self) -> Result<Vec<Urn>> {
        let sorted = toposort(&self.graph, None).map_err(|cycle| {
            let resource = &self.graph[cycle.node_id()];
            DevmerError::DependencyCycle(format!(
                "Cycle detected involving resource: {}",
                resource.urn
            ))
        })?;

        // Reverse because toposort gives us leaves first
        Ok(sorted
            .into_iter()
            .rev()
            .map(|idx| self.graph[idx].urn.clone())
            .collect())
    }

    /// Get reverse topological sort as URNs (deletion order)
    pub fn reverse_topological_sort(&self) -> Result<Vec<Urn>> {
        let sorted = toposort(&self.graph, None).map_err(|cycle| {
            let resource = &self.graph[cycle.node_id()];
            DevmerError::DependencyCycle(format!(
                "Cycle detected involving resource: {}",
                resource.urn
            ))
        })?;

        // No reverse for deletion - delete dependents first
        Ok(sorted
            .into_iter()
            .map(|idx| self.graph[idx].urn.clone())
            .collect())
    }

    /// Get direct dependencies of a resource
    pub fn dependencies(&self, urn: &Urn) -> Vec<&Resource> {
        self.urn_to_index
            .get(urn.as_str())
            .map(|&idx| {
                self.graph
                    .edges_directed(idx, Direction::Outgoing)
                    .map(|edge| &self.graph[edge.target()])
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get direct dependents of a resource
    pub fn dependents(&self, urn: &Urn) -> Vec<&Resource> {
        self.urn_to_index
            .get(urn.as_str())
            .map(|&idx| {
                self.graph
                    .edges_directed(idx, Direction::Incoming)
                    .map(|edge| &self.graph[edge.source()])
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all resources in the graph
    pub fn resources(&self) -> impl Iterator<Item = &Resource> {
        self.graph.node_weights()
    }

    /// Get the number of resources
    pub fn len(&self) -> usize {
        self.graph.node_count()
    }

    /// Check if the graph is empty
    pub fn is_empty(&self) -> bool {
        self.graph.node_count() == 0
    }

    /// Remove a resource from the graph
    pub fn remove_resource(&mut self, urn: &Urn) -> Option<Resource> {
        if let Some(idx) = self.urn_to_index.remove(urn.as_str()) {
            self.graph.remove_node(idx)
        } else {
            None
        }
    }

    /// Build the graph from resources, automatically detecting dependencies
    pub fn build_from_resources(resources: Vec<Resource>) -> Result<Self> {
        let mut graph = Self::new();

        // First pass: add all resources
        for resource in resources {
            graph.add_resource(resource);
        }

        // Collect dependency info from resources
        let dependency_info: Vec<_> = graph
            .graph
            .node_weights()
            .map(|resource| {
                (
                    resource.urn.clone(),
                    resource.options.depends_on.clone(),
                    resource.options.parent.clone(),
                )
            })
            .collect();

        // Second pass: add dependency edges
        for (from_urn, depends_on, parent) in dependency_info {
            // Add explicit dependencies
            for dep_urn in depends_on {
                if graph.urn_to_index.contains_key(dep_urn.as_str()) {
                    if graph.would_create_cycle(&from_urn, &dep_urn) {
                        return Err(DevmerError::DependencyCycle(format!(
                            "Adding dependency from {} to {} would create a cycle",
                            from_urn, dep_urn
                        )));
                    }
                    graph.add_dependency(&from_urn, &dep_urn, DependencyKind::Explicit)?;
                }
            }

            // Add parent dependency
            if let Some(parent_urn) = parent {
                if graph.urn_to_index.contains_key(parent_urn.as_str()) {
                    graph.add_dependency(&from_urn, &parent_urn, DependencyKind::Parent)?;
                }
            }
        }

        Ok(graph)
    }
}

impl Default for ResourceGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::ResourceType;
    use crate::types::PropertyValues;

    fn create_test_resource(name: &str) -> Resource {
        Resource::new(
            "test",
            ResourceType::new("aws", "s3", "Bucket"),
            name,
            PropertyValues::new(),
        )
    }

    #[test]
    fn test_add_and_get_resource() {
        let mut graph = ResourceGraph::new();
        let resource = create_test_resource("bucket-1");
        let urn = resource.urn.clone();

        graph.add_resource(resource);

        assert_eq!(graph.len(), 1);
        assert!(graph.get_resource(&urn).is_some());
    }

    #[test]
    fn test_dependency_order() {
        let mut graph = ResourceGraph::new();

        let bucket = create_test_resource("bucket");
        let policy = create_test_resource("policy");

        let bucket_urn = bucket.urn.clone();
        let policy_urn = policy.urn.clone();

        graph.add_resource(bucket);
        graph.add_resource(policy);

        // Policy depends on bucket
        graph
            .add_dependency(&policy_urn, &bucket_urn, DependencyKind::Explicit)
            .unwrap();

        let creation_order = graph.creation_order().unwrap();
        let names: Vec<_> = creation_order.iter().map(|r| r.name.as_str()).collect();

        // Bucket should be created before policy
        let bucket_pos = names.iter().position(|&n| n == "bucket").unwrap();
        let policy_pos = names.iter().position(|&n| n == "policy").unwrap();
        assert!(bucket_pos < policy_pos);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = ResourceGraph::new();

        let a = create_test_resource("a");
        let b = create_test_resource("b");

        let a_urn = a.urn.clone();
        let b_urn = b.urn.clone();

        graph.add_resource(a);
        graph.add_resource(b);

        graph
            .add_dependency(&a_urn, &b_urn, DependencyKind::Explicit)
            .unwrap();

        // This would create a cycle
        assert!(graph.would_create_cycle(&b_urn, &a_urn));
    }
}
