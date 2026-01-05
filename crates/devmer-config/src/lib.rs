//! # devmer-config
//!
//! Configuration parsing and environment variable interpolation for Devmer.
//!
//! This crate handles:
//! - Parsing Devmer.toml configuration files
//! - Environment variable interpolation
//! - .env file loading
//! - Configuration validation

pub mod error;
pub mod interpolation;
pub mod loader;
pub mod schema;

pub use error::{ConfigError, Result};
pub use loader::ConfigLoader;
pub use schema::{
    BackendConfig, DevmerConfig, ProviderConfigEntry, SecretsConfig, StackConfig,
};
