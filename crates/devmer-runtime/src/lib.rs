//! # devmer-runtime
//!
//! Language runtime abstraction for Devmer.
//!
//! This crate provides the ability to execute infrastructure programs written in:
//! - TypeScript/JavaScript (via Node.js, Deno, or Bun)
//! - Python
//! - Go
//! - Rhai (embedded scripting)
//!
//! ## Architecture
//!
//! The runtime uses a context-based architecture to avoid circular dependencies:
//! - `RuntimeContext` provides access to configuration, secrets, and resource registration
//! - Provider traits (`ConfigProvider`, `SecretsProvider`, `ResourceProvider`) are implemented
//!   by the DI layer and passed to the runtime
//! - Scripts interact with services through the context, not directly with the DI container
//!
//! Each language runtime implements the `LanguageRuntime` trait, which defines
//! how to execute a program and collect resource registrations via gRPC/IPC.

pub mod context;
pub mod error;
pub mod host;
pub mod registry;
pub mod rhai_runtime;
pub mod runtime;

pub use context::{
    ConfigProvider, ResourceProvider, RuntimeContext, SecretsProvider, SimpleConfigProvider,
    SimpleResourceProvider,
};
pub use error::{Result, RuntimeError};
pub use host::LanguageHost;
pub use registry::ResourceRegistry;
pub use rhai_runtime::RhaiRuntime;
pub use runtime::{LanguageRuntime, RunResult, RuntimeConfig, RuntimeKind};
