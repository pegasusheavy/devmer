//! # devmer-convert
//!
//! HCL to code conversion for Devmer.
//!
//! This crate provides functionality to convert Terraform/OpenTofu HCL projects
//! to scripting languages supported by Devmer (TypeScript, Python, Go, Rhai).
//!
//! ## Features
//!
//! - Parse HCL files (`.tf`, `.tf.json`)
//! - Extract resources, variables, outputs, data sources
//! - Generate equivalent code in target language
//! - Preserve comments and structure where possible
//! - Handle provider configurations
//! - Convert expressions and interpolations
//!
//! ## Example
//!
//! ```no_run
//! use devmer_convert::{convert_project, ConvertOptions, Language};
//! use std::path::Path;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let options = ConvertOptions {
//!         output_dir: Some("./output".into()),
//!         project_name: Some("my-project".to_string()),
//!         generate_config: true,
//!         ..Default::default()
//!     };
//!
//!     let project = convert_project(
//!         Path::new("./terraform"),
//!         Language::TypeScript,
//!         options,
//!     ).await?;
//!
//!     project.write_files()?;
//!     Ok(())
//! }
//! ```

pub mod codegen;
pub mod error;
pub mod hcl;
pub mod ir;

pub use codegen::{CodeGenerator, GeneratedFile, Language};
pub use error::{ConvertError, Result};
pub use hcl::HclParser;
pub use ir::{ConvertedProject, IrBlock, IrModule};

use std::path::Path;
use tracing::info;

/// Convert an HCL project to a target language
pub async fn convert_project(
    source_dir: &Path,
    target_lang: Language,
    options: ConvertOptions,
) -> Result<ConvertedProject> {
    info!("Converting HCL project from {:?} to {}", source_dir, target_lang);

    // Parse HCL files
    let parser = HclParser::new();
    let ir_module = parser.parse_directory(source_dir)?;

    info!(
        "Parsed {} resources, {} variables, {} outputs",
        ir_module.resources.len(),
        ir_module.variables.len(),
        ir_module.outputs.len()
    );

    // Generate code
    let generator = CodeGenerator::new(target_lang, options.clone());
    let mut files = generator.generate(&ir_module)?;

    // Generate Devmer.toml if requested
    if options.generate_config {
        let project_name = options
            .project_name
            .clone()
            .unwrap_or_else(|| {
                source_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("devmer-project")
                    .to_string()
            });
        files.push(generator.generate_config(&ir_module, &project_name)?);
    }

    Ok(ConvertedProject {
        source_dir: source_dir.to_path_buf(),
        target_language: target_lang,
        module: ir_module,
        generated_files: files,
        options,
    })
}

/// Convert a single HCL file to code
pub fn convert_file(
    source_file: &Path,
    target_lang: Language,
    options: ConvertOptions,
) -> Result<Vec<GeneratedFile>> {
    // Create a temporary module with just this file
    let content = std::fs::read_to_string(source_file)?;
    let body: ::hcl::Body = ::hcl::from_str(&content)
        .map_err(|e: ::hcl::Error| ConvertError::parse_error(source_file, e.to_string()))?;

    let mut module = ir::IrModule::default();
    // Process the body into the module
    for structure in body.iter() {
        match structure {
            ::hcl::Structure::Block(block) => {
                process_block_into_module(block, &mut module)?;
            }
            _ => {}
        }
    }
    module.source_files.push(source_file.to_path_buf());

    let generator = CodeGenerator::new(target_lang, options);
    generator.generate(&module)
}

/// Process a single block into the module (helper for convert_file)
fn process_block_into_module(block: &::hcl::Block, module: &mut ir::IrModule) -> Result<()> {
    // Use the parser's internal methods via re-parsing
    // This is a simplified version - in practice we'd refactor the parser
    let block_type = block.identifier.as_str();
    let labels: Vec<&str> = block.labels.iter().map(|l| l.as_str()).collect();

    match block_type {
        "resource" if labels.len() >= 2 => {
            let resource = ir::IrResource {
                resource_type: labels[0].to_string(),
                name: labels[1].to_string(),
                provider: None,
                attributes: process_body_attributes(&block.body),
                blocks: process_nested_blocks(&block.body),
                depends_on: vec![],
                count: None,
                for_each: None,
                lifecycle: None,
                comment: None,
            };
            module.resources.push(resource);
        }
        "variable" if !labels.is_empty() => {
            let var = ir::IrVariable {
                name: labels[0].to_string(),
                var_type: None,
                default: None,
                description: None,
                sensitive: false,
                nullable: true,
                validations: vec![],
                comment: None,
            };
            module.variables.push(var);
        }
        "output" if !labels.is_empty() => {
            let output = ir::IrOutput {
                name: labels[0].to_string(),
                value: ir::IrExpression::Null,
                description: None,
                sensitive: false,
                depends_on: vec![],
                comment: None,
            };
            module.outputs.push(output);
        }
        _ => {}
    }

    Ok(())
}

/// Process body attributes
fn process_body_attributes(body: &::hcl::Body) -> indexmap::IndexMap<String, ir::IrExpression> {
    let mut attrs = indexmap::IndexMap::new();
    for structure in body.iter() {
        if let ::hcl::Structure::Attribute(attr) = structure {
            let key = attr.key.as_str().to_string();
            let value = hcl::parse_expression(&attr.expr);
            attrs.insert(key, value);
        }
    }
    attrs
}

/// Process nested blocks
fn process_nested_blocks(body: &::hcl::Body) -> Vec<ir::IrBlock> {
    let mut blocks = Vec::new();
    for structure in body.iter() {
        if let ::hcl::Structure::Block(block) = structure {
            let ir_block = ir::IrBlock {
                block_type: block.identifier.as_str().to_string(),
                labels: block.labels.iter().map(|l| l.as_str().to_string()).collect(),
                attributes: process_body_attributes(&block.body),
                blocks: process_nested_blocks(&block.body),
            };
            blocks.push(ir_block);
        }
    }
    blocks
}

/// Options for conversion
#[derive(Debug, Clone, Default)]
pub struct ConvertOptions {
    /// Output directory (default: current directory)
    pub output_dir: Option<std::path::PathBuf>,

    /// Project name (default: derived from directory)
    pub project_name: Option<String>,

    /// Generate Devmer.toml configuration
    pub generate_config: bool,

    /// Include comments from HCL
    pub preserve_comments: bool,

    /// Generate sample tests
    pub generate_tests: bool,

    /// Provider mappings (terraform provider -> devmer provider)
    pub provider_mappings: std::collections::HashMap<String, String>,

    /// Use async/await syntax where applicable
    pub use_async: bool,

    /// Format output code
    pub format_output: bool,

    /// Generate type definitions (TypeScript)
    pub generate_types: bool,

    /// Target JavaScript runtime (node, deno, bun)
    pub js_runtime: Option<String>,

    /// Verbose output
    pub verbose: bool,
}

impl ConvertOptions {
    /// Create options for TypeScript conversion
    pub fn typescript() -> Self {
        Self {
            generate_types: true,
            use_async: true,
            js_runtime: Some("node".to_string()),
            ..Default::default()
        }
    }

    /// Create options for Python conversion
    pub fn python() -> Self {
        Self {
            use_async: true,
            ..Default::default()
        }
    }

    /// Create options for Go conversion
    pub fn go() -> Self {
        Self::default()
    }

    /// Create options for Rhai conversion
    pub fn rhai() -> Self {
        Self::default()
    }
}

/// Conversion statistics
#[derive(Debug, Clone, Default)]
pub struct ConversionStats {
    /// Number of files processed
    pub files_processed: usize,
    /// Number of resources converted
    pub resources: usize,
    /// Number of data sources converted
    pub data_sources: usize,
    /// Number of variables converted
    pub variables: usize,
    /// Number of outputs converted
    pub outputs: usize,
    /// Number of modules converted
    pub modules: usize,
    /// Number of locals converted
    pub locals: usize,
    /// Warnings generated
    pub warnings: Vec<String>,
    /// Unsupported features encountered
    pub unsupported: Vec<String>,
}

impl ConversionStats {
    /// Create stats from an IR module
    pub fn from_module(module: &ir::IrModule) -> Self {
        Self {
            files_processed: module.source_files.len(),
            resources: module.resources.len(),
            data_sources: module.data_sources.len(),
            variables: module.variables.len(),
            outputs: module.outputs.len(),
            modules: module.modules.len(),
            locals: module.locals.len(),
            warnings: vec![],
            unsupported: vec![],
        }
    }
}

/// Provider mapping for Terraform to Devmer providers
pub fn default_provider_mappings() -> std::collections::HashMap<String, String> {
    let mut mappings = std::collections::HashMap::new();
    mappings.insert("hashicorp/aws".to_string(), "aws".to_string());
    mappings.insert("hashicorp/google".to_string(), "gcp".to_string());
    mappings.insert("hashicorp/azurerm".to_string(), "azure".to_string());
    mappings.insert("hashicorp/kubernetes".to_string(), "kubernetes".to_string());
    mappings.insert("hashicorp/helm".to_string(), "helm".to_string());
    mappings.insert("hashicorp/random".to_string(), "random".to_string());
    mappings.insert("hashicorp/null".to_string(), "null".to_string());
    mappings.insert("hashicorp/local".to_string(), "local".to_string());
    mappings.insert("hashicorp/tls".to_string(), "tls".to_string());
    mappings.insert("hashicorp/http".to_string(), "http".to_string());
    mappings.insert("hashicorp/archive".to_string(), "archive".to_string());
    mappings.insert("hashicorp/external".to_string(), "external".to_string(),);
    mappings.insert("hashicorp/time".to_string(), "time".to_string());
    mappings.insert("hashicorp/vault".to_string(), "vault".to_string());
    mappings.insert("hashicorp/consul".to_string(), "consul".to_string());
    mappings
}

/// Terraform function to Devmer SDK function mapping
pub fn function_mappings() -> std::collections::HashMap<&'static str, FunctionMapping> {
    let mut mappings = std::collections::HashMap::new();

    // String functions
    mappings.insert("chomp", FunctionMapping::simple("devmer.chomp"));
    mappings.insert("format", FunctionMapping::simple("devmer.format"));
    mappings.insert("formatlist", FunctionMapping::simple("devmer.formatList"));
    mappings.insert("indent", FunctionMapping::simple("devmer.indent"));
    mappings.insert("join", FunctionMapping::method("join"));
    mappings.insert("lower", FunctionMapping::method("toLowerCase"));
    mappings.insert("upper", FunctionMapping::method("toUpperCase"));
    mappings.insert("regex", FunctionMapping::simple("devmer.regex"));
    mappings.insert("regexall", FunctionMapping::simple("devmer.regexAll"));
    mappings.insert("replace", FunctionMapping::method("replace"));
    mappings.insert("split", FunctionMapping::method("split"));
    mappings.insert("strrev", FunctionMapping::simple("devmer.strrev"));
    mappings.insert("substr", FunctionMapping::method("substring"));
    mappings.insert("title", FunctionMapping::simple("devmer.title"));
    mappings.insert("trim", FunctionMapping::method("trim"));
    mappings.insert("trimprefix", FunctionMapping::simple("devmer.trimPrefix"));
    mappings.insert("trimsuffix", FunctionMapping::simple("devmer.trimSuffix"));
    mappings.insert("trimspace", FunctionMapping::method("trim"));

    // Collection functions
    mappings.insert("concat", FunctionMapping::simple("devmer.concat"));
    mappings.insert("contains", FunctionMapping::method("includes"));
    mappings.insert("distinct", FunctionMapping::simple("devmer.distinct"));
    mappings.insert("element", FunctionMapping::simple("devmer.element"));
    mappings.insert("flatten", FunctionMapping::method("flat"));
    mappings.insert("index", FunctionMapping::method("indexOf"));
    mappings.insert("keys", FunctionMapping::simple("Object.keys"));
    mappings.insert("length", FunctionMapping::property("length"));
    mappings.insert("lookup", FunctionMapping::simple("devmer.lookup"));
    mappings.insert("merge", FunctionMapping::simple("devmer.merge"));
    mappings.insert("range", FunctionMapping::simple("devmer.range"));
    mappings.insert("reverse", FunctionMapping::method("reverse"));
    mappings.insert("setintersection", FunctionMapping::simple("devmer.setIntersection"));
    mappings.insert("setproduct", FunctionMapping::simple("devmer.setProduct"));
    mappings.insert("setsubtract", FunctionMapping::simple("devmer.setSubtract"));
    mappings.insert("setunion", FunctionMapping::simple("devmer.setUnion"));
    mappings.insert("slice", FunctionMapping::method("slice"));
    mappings.insert("sort", FunctionMapping::method("sort"));
    mappings.insert("values", FunctionMapping::simple("Object.values"));
    mappings.insert("zipmap", FunctionMapping::simple("devmer.zipMap"));

    // Encoding functions
    mappings.insert("base64decode", FunctionMapping::simple("devmer.base64Decode"));
    mappings.insert("base64encode", FunctionMapping::simple("devmer.base64Encode"));
    mappings.insert("base64gzip", FunctionMapping::simple("devmer.base64Gzip"));
    mappings.insert("csvdecode", FunctionMapping::simple("devmer.csvDecode"));
    mappings.insert("jsondecode", FunctionMapping::simple("JSON.parse"));
    mappings.insert("jsonencode", FunctionMapping::simple("JSON.stringify"));
    mappings.insert("urlencode", FunctionMapping::simple("encodeURIComponent"));
    mappings.insert("yamldecode", FunctionMapping::simple("devmer.yamlDecode"));
    mappings.insert("yamlencode", FunctionMapping::simple("devmer.yamlEncode"));

    // Hash functions
    mappings.insert("md5", FunctionMapping::simple("devmer.md5"));
    mappings.insert("sha1", FunctionMapping::simple("devmer.sha1"));
    mappings.insert("sha256", FunctionMapping::simple("devmer.sha256"));
    mappings.insert("sha512", FunctionMapping::simple("devmer.sha512"));
    mappings.insert("bcrypt", FunctionMapping::simple("devmer.bcrypt"));
    mappings.insert("uuid", FunctionMapping::simple("devmer.uuid"));

    // Filesystem functions
    mappings.insert("abspath", FunctionMapping::simple("devmer.abspath"));
    mappings.insert("dirname", FunctionMapping::simple("devmer.dirname"));
    mappings.insert("basename", FunctionMapping::simple("devmer.basename"));
    mappings.insert("file", FunctionMapping::simple("devmer.readFile"));
    mappings.insert("filebase64", FunctionMapping::simple("devmer.fileBase64"));
    mappings.insert("fileexists", FunctionMapping::simple("devmer.fileExists"));
    mappings.insert("fileset", FunctionMapping::simple("devmer.fileSet"));
    mappings.insert("pathexpand", FunctionMapping::simple("devmer.pathExpand"));
    mappings.insert("templatefile", FunctionMapping::simple("devmer.templateFile"));

    // Type conversion functions
    mappings.insert("tobool", FunctionMapping::simple("Boolean"));
    mappings.insert("tolist", FunctionMapping::simple("Array.from"));
    mappings.insert("tomap", FunctionMapping::simple("devmer.toMap"));
    mappings.insert("tonumber", FunctionMapping::simple("Number"));
    mappings.insert("toset", FunctionMapping::simple("new Set"));
    mappings.insert("tostring", FunctionMapping::simple("String"));

    // Numeric functions
    mappings.insert("abs", FunctionMapping::simple("Math.abs"));
    mappings.insert("ceil", FunctionMapping::simple("Math.ceil"));
    mappings.insert("floor", FunctionMapping::simple("Math.floor"));
    mappings.insert("log", FunctionMapping::simple("Math.log"));
    mappings.insert("max", FunctionMapping::simple("Math.max"));
    mappings.insert("min", FunctionMapping::simple("Math.min"));
    mappings.insert("pow", FunctionMapping::simple("Math.pow"));
    mappings.insert("signum", FunctionMapping::simple("Math.sign"));

    // Date/Time functions
    mappings.insert("formatdate", FunctionMapping::simple("devmer.formatDate"));
    mappings.insert("plantimestamp", FunctionMapping::simple("devmer.planTimestamp"));
    mappings.insert("timeadd", FunctionMapping::simple("devmer.timeAdd"));
    mappings.insert("timecmp", FunctionMapping::simple("devmer.timeCmp"));
    mappings.insert("timestamp", FunctionMapping::simple("new Date().toISOString"));

    // IP network functions
    mappings.insert("cidrhost", FunctionMapping::simple("devmer.cidrHost"));
    mappings.insert("cidrnetmask", FunctionMapping::simple("devmer.cidrNetmask"));
    mappings.insert("cidrsubnet", FunctionMapping::simple("devmer.cidrSubnet"));
    mappings.insert("cidrsubnets", FunctionMapping::simple("devmer.cidrSubnets"));

    // Control flow functions
    mappings.insert("coalesce", FunctionMapping::simple("devmer.coalesce"));
    mappings.insert("coalescelist", FunctionMapping::simple("devmer.coalesceList"));
    mappings.insert("try", FunctionMapping::simple("devmer.try"));
    mappings.insert("can", FunctionMapping::simple("devmer.can"));

    mappings
}

/// Function mapping types
#[derive(Debug, Clone)]
pub enum FunctionMapping {
    /// Simple function call replacement
    Simple(String),
    /// Method call (chained to first argument)
    Method(String),
    /// Property access
    Property(String),
    /// Custom transformation
    Custom(String),
}

impl FunctionMapping {
    pub fn simple(name: &str) -> Self {
        Self::Simple(name.to_string())
    }

    pub fn method(name: &str) -> Self {
        Self::Method(name.to_string())
    }

    pub fn property(name: &str) -> Self {
        Self::Property(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_project(dir: &Path) {
        // Create main.tf
        let main_tf = r#"
terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
  backend "s3" {
    bucket = "my-terraform-state"
    key    = "state.tfstate"
    region = "us-east-1"
  }
}

provider "aws" {
  region = var.region
}
"#;
        std::fs::write(dir.join("main.tf"), main_tf).unwrap();

        // Create variables.tf
        let variables_tf = r#"
variable "region" {
  type        = string
  description = "AWS region"
  default     = "us-east-1"
}

variable "environment" {
  type        = string
  description = "Environment name"
}

variable "tags" {
  type        = map(string)
  description = "Common tags"
  default     = {}
}
"#;
        std::fs::write(dir.join("variables.tf"), variables_tf).unwrap();

        // Create resources.tf
        let resources_tf = r#"
resource "aws_s3_bucket" "main" {
  bucket = "${var.environment}-my-bucket"
  
  tags = merge(var.tags, {
    Name        = "${var.environment}-my-bucket"
    Environment = var.environment
  })
}

resource "aws_s3_bucket_versioning" "main" {
  bucket = aws_s3_bucket.main.id
  
  versioning_configuration {
    status = "Enabled"
  }
}

resource "aws_dynamodb_table" "state_lock" {
  name           = "${var.environment}-state-lock"
  billing_mode   = "PAY_PER_REQUEST"
  hash_key       = "LockID"
  
  attribute {
    name = "LockID"
    type = "S"
  }
  
  tags = var.tags
}
"#;
        std::fs::write(dir.join("resources.tf"), resources_tf).unwrap();

        // Create outputs.tf
        let outputs_tf = r#"
output "bucket_arn" {
  value       = aws_s3_bucket.main.arn
  description = "The ARN of the S3 bucket"
}

output "bucket_name" {
  value = aws_s3_bucket.main.id
}

output "dynamodb_table_name" {
  value = aws_dynamodb_table.state_lock.name
}
"#;
        std::fs::write(dir.join("outputs.tf"), outputs_tf).unwrap();
    }

    #[tokio::test]
    async fn test_convert_project_typescript() {
        let temp = TempDir::new().unwrap();
        create_test_project(temp.path());

        let options = ConvertOptions {
            project_name: Some("test-project".to_string()),
            generate_config: true,
            ..ConvertOptions::typescript()
        };

        let result = convert_project(temp.path(), Language::TypeScript, options).await;
        assert!(result.is_ok());

        let project = result.unwrap();
        assert_eq!(project.target_language, Language::TypeScript);
        assert!(!project.generated_files.is_empty());

        // Check for expected files
        let file_names: Vec<&str> = project
            .generated_files
            .iter()
            .map(|f| f.path.to_str().unwrap())
            .collect();

        assert!(file_names.contains(&"index.ts"));
        assert!(file_names.contains(&"package.json"));
        assert!(file_names.contains(&"Devmer.toml"));
    }

    #[tokio::test]
    async fn test_convert_project_python() {
        let temp = TempDir::new().unwrap();
        create_test_project(temp.path());

        let options = ConvertOptions {
            project_name: Some("test-project".to_string()),
            generate_config: true,
            ..ConvertOptions::python()
        };

        let result = convert_project(temp.path(), Language::Python, options).await;
        assert!(result.is_ok());

        let project = result.unwrap();
        assert_eq!(project.target_language, Language::Python);

        let file_names: Vec<&str> = project
            .generated_files
            .iter()
            .map(|f| f.path.to_str().unwrap())
            .collect();

        assert!(file_names.contains(&"__main__.py"));
        assert!(file_names.contains(&"requirements.txt"));
    }

    #[tokio::test]
    async fn test_convert_project_go() {
        let temp = TempDir::new().unwrap();
        create_test_project(temp.path());

        let options = ConvertOptions {
            project_name: Some("test-project".to_string()),
            generate_config: true,
            ..ConvertOptions::go()
        };

        let result = convert_project(temp.path(), Language::Go, options).await;
        assert!(result.is_ok());

        let project = result.unwrap();
        assert_eq!(project.target_language, Language::Go);

        let file_names: Vec<&str> = project
            .generated_files
            .iter()
            .map(|f| f.path.to_str().unwrap())
            .collect();

        assert!(file_names.contains(&"main.go"));
        assert!(file_names.contains(&"go.mod"));
    }

    #[test]
    fn test_conversion_stats() {
        let mut module = IrModule::default();
        module.resources.push(ir::IrResource {
            resource_type: "aws_s3_bucket".to_string(),
            name: "test".to_string(),
            provider: None,
            attributes: indexmap::IndexMap::new(),
            blocks: vec![],
            depends_on: vec![],
            count: None,
            for_each: None,
            lifecycle: None,
            comment: None,
        });
        module.variables.push(ir::IrVariable {
            name: "test".to_string(),
            var_type: None,
            default: None,
            description: None,
            sensitive: false,
            nullable: true,
            validations: vec![],
            comment: None,
        });

        let stats = ConversionStats::from_module(&module);
        assert_eq!(stats.resources, 1);
        assert_eq!(stats.variables, 1);
    }
}
