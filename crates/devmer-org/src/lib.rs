//! # devmer-org
//!
//! Organizational administration and Role-Based Access Control (RBAC) for Devmer.
//!
//! This crate provides:
//! - **Organization hierarchy**: Organizations → Teams → Users
//! - **Role-based access control**: Fine-grained permissions system
//! - **Resource policies**: Control which teams can deploy which infrastructure
//! - **Approval workflows**: Require approvals for sensitive deployments
//!
//! ## Example
//!
//! ```rust,ignore
//! use devmer_org::{Organization, Team, User, Role, Permission, ResourcePolicy};
//!
//! // Create an organization
//! let org = Organization::new("acme-corp", "Acme Corporation");
//!
//! // Create teams
//! let marketing_team = Team::new("marketing", "Marketing Team");
//! let platform_team = Team::new("platform", "Platform Engineering");
//!
//! // Define what marketing can deploy
//! let marketing_policy = ResourcePolicy::builder()
//!     .allow_stacks(&["marketing/*", "shared/cdn"])
//!     .allow_resources(&["aws:s3:*", "aws:cloudfront:*", "aws:route53:*"])
//!     .deny_resources(&["aws:iam:*", "aws:kms:*"])
//!     .require_approval_for(&["production/*"])
//!     .build();
//! ```

pub mod error;
pub mod organization;
pub mod team;
pub mod user;
pub mod role;
pub mod permission;
pub mod policy;
pub mod access;
pub mod approval;

pub use error::{OrgError, Result};
pub use organization::{Organization, OrganizationId};
pub use team::{Team, TeamId, TeamMembership};
pub use user::{User, UserId, UserStatus};
pub use role::{Role, RoleId, BuiltinRole};
pub use permission::{Permission, Action, ResourceScope};
pub use policy::{ResourcePolicy, PolicyRule, PolicyEffect};
pub use access::{AccessDecision, AccessContext, AccessChecker};
pub use approval::{ApprovalWorkflow, ApprovalRequest, ApprovalStatus};
