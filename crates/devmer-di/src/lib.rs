//! # devmer-di
//!
//! Dependency injection framework for Devmer using shaku.
//!
//! This crate provides:
//! - Service container definition
//! - Service interfaces (traits)
//! - Module composition
//! - Testing utilities

pub mod container;
pub mod interfaces;
pub mod modules;

pub use container::AppContainer;
pub use interfaces::*;
pub use modules::{
    ConfigServiceImpl, ExecutionServiceImpl, ProviderRegistryServiceImpl, RuntimeServiceImpl,
    StateServiceImpl,
};
