//! Devmer Cloud Providers
//!
//! This crate provides implementations for various cloud providers:
//!
//! - **AWS** - Amazon Web Services (S3, Lambda, IAM, EC2, RDS, etc.)
//! - **Mock** - A mock provider for testing
//!
//! ## Example
//!
//! ```rust,ignore
//! use devmer_providers::aws::AwsProvider;
//! use devmer_core::registry::ProviderRegistry;
//! use std::sync::Arc;
//!
//! let registry = ProviderRegistry::new();
//! registry.register("aws", Arc::new(AwsProvider::new()));
//! ```

pub mod aws;
pub mod mock;

pub use aws::{AwsConfig, AwsCredentials, AwsProvider, AwsRegion};
pub use mock::MockProvider;
