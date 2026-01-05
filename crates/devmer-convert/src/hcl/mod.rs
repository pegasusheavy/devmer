//! HCL parsing module

mod parser;
mod expressions;

pub use parser::HclParser;
pub use expressions::parse_expression;

// Re-export hcl types for convenience
pub use hcl::{Body, Block, Structure, Expression};
