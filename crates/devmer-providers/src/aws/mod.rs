//! AWS Provider for Devmer
//!
//! This module implements the AWS cloud provider, supporting resources like:
//! - S3 buckets
//! - Lambda functions
//! - IAM roles and policies
//! - DynamoDB tables
//! - EC2 instances
//! - VPC and networking resources
//! - API Gateway
//! - SQS/SNS messaging
//! - CloudWatch

mod config;
mod provider;
mod resources;
mod schemas;

pub use config::{AwsConfig, AwsCredentials, AwsRegion};
pub use provider::AwsProvider;
