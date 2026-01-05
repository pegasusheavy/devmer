//! # devmer-secrets
//!
//! Secrets encryption and management for Devmer.
//!
//! This crate provides:
//! - Multiple encryption providers (passphrase, AWS KMS, etc.)
//! - Secure memory handling
//! - Secret rotation

pub mod error;
pub mod memory;
pub mod provider;
pub mod types;

#[cfg(feature = "passphrase")]
pub mod passphrase;

pub use error::{SecretsError, Result};
pub use provider::SecretsProvider;
pub use types::{EncryptedValue, EncryptionContext, SecretMetadata};
