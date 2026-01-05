//! # devmer-core
//!
//! Core types, resource graph, and execution engine for Devmer.
//!
//! This crate provides the fundamental building blocks for the Devmer IaC system:
//! - Resource and provider abstractions
//! - Dependency graph management
//! - Execution planning and diffing
//! - State representation

pub mod engine;
pub mod error;
pub mod graph;
pub mod provider;
pub mod registry;
pub mod resource;
pub mod state;
pub mod types;

pub use engine::{DeploymentExecutor, DeploymentPlan, PlanBuilder, ResourceOperation};
pub use error::{DevmerError, Result};
pub use graph::ResourceGraph;
pub use provider::{Provider, ProviderConfig, ProviderSchema};
pub use registry::{ProviderFactory, ProviderFactoryRegistry, ProviderRegistry};
pub use resource::{
    Resource, ResourceId, ResourceOptions, ResourceOutput, ResourceState, ResourceType, Urn,
};
pub use state::{StackState, StateCheckpoint};
pub use types::{PropertyValue, PropertyValues};
