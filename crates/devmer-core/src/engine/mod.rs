//! Execution engine for Devmer
//!
//! This module provides the core deployment engine that:
//! - Runs programs in various language runtimes
//! - Collects resource registrations
//! - Builds dependency graphs
//! - Plans and executes deployments

mod executor;
mod plan;
mod step;

pub use executor::DeploymentExecutor;
pub use plan::{DeploymentPlan, PlanBuilder, ResourceOperation};
pub use step::{ExecutionStep, StepResult, StepStatus};
